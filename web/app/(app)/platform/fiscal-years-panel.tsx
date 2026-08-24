"use client";

import { useState } from "react";
import { useForm, useController } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { DateField } from "@/components/date-field";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";

interface FiscalYear {
  id: number;
  year: number;
  startDate: string;
  endDate: string;
  isActive: boolean;
}

const schema = z
  .object({
    year: z.number().int().min(1300).max(1600),
    startDate: z.string().min(1),
    endDate: z.string().min(1),
  })
  .refine((v) => v.endDate > v.startDate, {
    message: "fiscalYears.invalidDateRange",
    path: ["endDate"],
  });
type FormValues = z.infer<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  year_already_exists: "fiscalYears.yearAlreadyExists",
  date_range_overlaps: "fiscalYears.dateRangeOverlaps",
  invalid_date_range: "fiscalYears.invalidDateRange",
  already_closed: "fiscalYears.alreadyClosed",
};

export function FiscalYearsPanel({ canManage }: { canManage: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);

  const { data: years, isLoading } = useQuery({
    queryKey: ["fiscal-years"],
    queryFn: () => apiRequest<FiscalYear[]>("/api/v1/fiscal-years"),
  });
  const { data: current } = useQuery({
    queryKey: ["fiscal-years", "current"],
    queryFn: () => apiRequest<{ fiscalYearId: number | null }>("/api/v1/fiscal-years/current"),
  });

  const {
    register,
    control,
    handleSubmit,
    reset,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({ resolver: zodResolver(schema) });
  const { field: startDateField } = useController({ control, name: "startDate" });
  const { field: endDateField } = useController({ control, name: "endDate" });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["fiscal-years"] });
  };

  const createMutation = useMutation({
    mutationFn: (values: FormValues) =>
      apiRequest("/api/v1/fiscal-years", { method: "POST", body: JSON.stringify(values) }),
    onSuccess: () => {
      reset();
      invalidate();
      setCreateOpen(false);
    },
    onError: (err: ApiError) => {
      const key = ERROR_KEYS[err.message] ?? "common.error";
      setError("root", { message: t(key) });
    },
  });

  const closeMutation = useMutation({
    mutationFn: (id: number) => apiRequest(`/api/v1/fiscal-years/${id}/close`, { method: "POST" }),
    onSuccess: invalidate,
  });

  const switchMutation = useMutation({
    mutationFn: (fiscalYearId: number) =>
      apiRequest("/api/v1/fiscal-years/current", {
        method: "PUT",
        body: JSON.stringify({ fiscalYearId }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["fiscal-years", "current"] });
    },
  });

  return (
    <div className="flex flex-col gap-8">
      <section>
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-base font-medium text-foreground">{t("fiscalYears.title")}</h2>
          {canManage && <NewButton onClickAction={() => setCreateOpen(true)}>{t("fiscalYears.newFiscalYear")}</NewButton>}
        </div>

        {isLoading ? (
          <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
        ) : (
          <div className="overflow-x-auto rounded-md border border-border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50 text-start text-muted-foreground">
                  <th className="px-3 py-2 text-start font-medium">{t("fiscalYears.year")}</th>
                  <th className="px-3 py-2 text-start font-medium">{t("fiscalYears.startDate")}</th>
                  <th className="px-3 py-2 text-start font-medium">{t("fiscalYears.endDate")}</th>
                  <th className="px-3 py-2 text-start font-medium">{t("fiscalYears.status")}</th>
                  <th className="px-3 py-2 text-start font-medium">{t("common.actions")}</th>
                </tr>
              </thead>
              <tbody>
                {years?.map((fy) => (
                  <tr key={fy.id} className="border-b border-border last:border-0">
                    <td className="tabular-nums px-3 py-2 text-foreground">
                      {toPersianDigits(fy.year)}
                      {current?.fiscalYearId === fy.id && (
                        <span className="ms-2 text-xs text-accent">({t("fiscalYears.current")})</span>
                      )}
                    </td>
                    <td className="px-3 py-2 text-muted-foreground">{fy.startDate}</td>
                    <td className="px-3 py-2 text-muted-foreground">{fy.endDate}</td>
                    <td className="px-3 py-2">
                      {fy.isActive ? (
                        <span className="text-success">{t("common.active")}</span>
                      ) : (
                        <span className="text-muted-foreground">{t("common.inactive")}</span>
                      )}
                    </td>
                    <td className="px-3 py-2">
                      <div className="flex gap-2">
                        {current?.fiscalYearId !== fy.id && (
                          <button
                            type="button"
                            onClick={() => switchMutation.mutate(fy.id)}
                            className="cursor-pointer text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                          >
                            {t("fiscalYears.switchTo")}
                          </button>
                        )}
                        {canManage && fy.isActive && (
                          <button
                            type="button"
                            onClick={() => closeMutation.mutate(fy.id)}
                            className="cursor-pointer text-danger hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                          >
                            {t("fiscalYears.close")}
                          </button>
                        )}
                      </div>
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          </div>
        )}

        {canManage && (
          <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("fiscalYears.newFiscalYear")}>
            <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
              <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
                <Field label={t("fiscalYears.year")}>
                  <input
                    type="number"
                    placeholder="1403"
                    {...register("year", { valueAsNumber: true })}
                    className={fieldInputClass}
                    autoFocus
                  />
                </Field>
                <DateField label={t("fiscalYears.startDate")} value={startDateField.value} onChangeAction={startDateField.onChange} />
                <DateField label={t("fiscalYears.endDate")} value={endDateField.value} onChangeAction={endDateField.onChange} />
              </div>
              {(errors.year || errors.startDate || errors.endDate || errors.root) && (
                <p role="alert" className="text-sm text-danger">
                  {errors.root?.message ??
                    (errors.endDate?.message && t(errors.endDate.message)) ??
                    t("fiscalYears.invalidDateRange")}
                </p>
              )}
              <div className="flex gap-3">
                <button
                  type="submit"
                  disabled={isSubmitting}
                  className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition-colors duration-150 hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
                >
                  {t("common.save")}
                </button>
                <button
                  type="button"
                  onClick={() => setCreateOpen(false)}
                  className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("common.cancel")}
                </button>
              </div>
            </form>
          </Modal>
        )}
      </section>
    </div>
  );
}
