"use client";

import { useForm, useController } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import type { VoucherSummary } from "@/lib/vouchers";
import { DateField } from "@/components/date-field";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";
import { Select } from "@/components/select";
import { DataTable, FilterInput, filterSelectClass, useDebounced, useSort } from "@/components/data-table";
import { useState } from "react";

const schema = z.object({
  voucherDate: z.string().min(1),
  description: z.string().min(1),
});
type FormValues = z.infer<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  date_outside_fiscal_year: "vouchers.dateOutsideFiscalYear",
  duplicate_voucher_number: "vouchers.duplicateVoucherNumber",
  description_required: "vouchers.voucherEmpty",
  fiscal_year_closed: "fiscalYears.alreadyClosed",
};

const STATUS_LABEL: Record<string, string> = {
  draft: "vouchers.draft",
  confirmed: "vouchers.confirmed",
  posted: "vouchers.posted",
};

export function VoucherList({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);
  const [descriptionFilter, setDescriptionFilter] = useState("");
  const [statusFilter, setStatusFilter] = useState("");
  const debouncedDescription = useDebounced(descriptionFilter);
  const { sort, toggleSort } = useSort();

  const params = new URLSearchParams();
  if (fiscalYearId) params.set("fiscalYearId", String(fiscalYearId));
  if (debouncedDescription) params.set("description", debouncedDescription);
  if (statusFilter) params.set("status", statusFilter);
  if (sort) {
    params.set("sort", sort.field);
    params.set("order", sort.dir);
  }

  const { data: vouchers, isLoading } = useQuery({
    queryKey: ["vouchers", fiscalYearId, debouncedDescription, statusFilter, sort],
    queryFn: () => apiRequest<VoucherSummary[]>(`/api/v1/vouchers?${params.toString()}`),
    enabled: fiscalYearId !== null,
  });

  const {
    register,
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({ resolver: zodResolver(schema) });
  const { field: voucherDateField } = useController({ control, name: "voucherDate" });

  const createMutation = useMutation({
    mutationFn: (values: FormValues) =>
      apiRequest<{ id: number }>("/api/v1/vouchers", {
        method: "POST",
        body: JSON.stringify({ ...values, fiscalYearId }),
      }),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["vouchers"] });
      setCreateOpen(false);
      router.push(`/accounting/vouchers/${result.id}`);
    },
    onError: (err: ApiError) => {
      setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") });
    },
  });

  if (fiscalYearId === null) {
    return <p className="text-sm text-muted-foreground">{t("shell.noFiscalYear")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-end">
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("vouchers.newVoucher")}</NewButton>
      </div>

      <DataTable<VoucherSummary>
        columns={[
          {
            key: "voucherNumber",
            header: t("vouchers.voucherNumber"),
            sortable: true,
            render: (v) => (
              <Link
                href={`/accounting/vouchers/${v.id}`}
                className="tabular-nums text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
              >
                {toPersianDigits(v.voucherNumber)}
              </Link>
            ),
          },
          {
            key: "voucherDate",
            header: t("vouchers.voucherDate"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            render: (v) => v.voucherDate,
          },
          {
            key: "description",
            header: t("vouchers.description"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={descriptionFilter} onChangeAction={setDescriptionFilter} />,
            render: (v) => v.description,
          },
          {
            key: "totalDebit",
            header: t("vouchers.totalDebit"),
            sortable: true,
            tdClassName: "tabular-nums text-foreground",
            render: (v) => toPersianDigits(v.totalDebit),
          },
          {
            key: "totalCredit",
            header: t("vouchers.totalCredit"),
            sortable: true,
            tdClassName: "tabular-nums text-foreground",
            render: (v) => toPersianDigits(v.totalCredit),
          },
          {
            key: "status",
            header: t("vouchers.status"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            filter: (
              <Select
                value={statusFilter}
                onChangeAction={setStatusFilter}
                placeholder={t("treasury.allStatuses")}
                className={filterSelectClass}
                options={Object.entries(STATUS_LABEL).map(([value, key]) => ({ value, label: t(key) }))}
              />
            ),
            render: (v) => t(STATUS_LABEL[v.status]),
          },
        ]}
        rows={vouchers}
        isLoading={isLoading}
        rowKeyAction={(v) => v.id}
        sort={sort}
        onSortAction={toggleSort}
        onRowClickAction={(v) => router.push(`/accounting/vouchers/${v.id}`)}
      />

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("vouchers.newVoucher")}>
        <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <DateField
              label={t("vouchers.voucherDate")}
              value={voucherDateField.value}
              onChangeAction={voucherDateField.onChange}
            />
            <Field label={t("vouchers.description")} wide>
              <input type="text" placeholder="خرید پسته از انبار مرکزی" {...register("description")} className={fieldInputClass} />
            </Field>
          </div>
          {(errors.voucherDate || errors.description || errors.root) && (
            <p role="alert" className="text-sm text-danger">
              {errors.root?.message ?? t("common.error")}
            </p>
          )}
          <div className="flex gap-3">
            <button
              type="submit"
              disabled={isSubmitting}
              className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
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
    </div>
  );
}
