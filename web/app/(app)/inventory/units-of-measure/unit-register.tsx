"use client";

// Step 5.9: `Anbar_Vahed`'s first-ever maintenance screen (specs/05-inventory §1.3 — "no
// maintenance screen for it" in the legacy, "must be populated by direct SQL").

import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";
import { Select } from "@/components/select";
import { DataTable, FilterInput, useDebounced, useSort } from "@/components/data-table";
import type { UnitOfMeasure } from "@/lib/inventory";

const schema = z.object({
  name: z.string().min(1),
  baseUnitId: z.coerce.number().int().positive().optional(),
  conversionFactor: z.coerce.string().optional(),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  invalid_name: "inventory.nameRequired",
  duplicate_name: "inventory.duplicateCode",
  base_unit_not_found: "common.error",
  base_unit_must_not_itself_be_derived: "common.error",
  unit_cannot_be_own_base: "common.error",
};

export function UnitRegister() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [createOpen, setCreateOpen] = useState(false);
  const [nameFilter, setNameFilter] = useState("");
  const debouncedName = useDebounced(nameFilter);
  const { sort, toggleSort } = useSort();

  const params = new URLSearchParams();
  if (debouncedName) params.set("search", debouncedName);
  if (sort) {
    params.set("sort", sort.field);
    params.set("order", sort.dir);
  }

  const { data: units, isLoading } = useQuery({
    queryKey: ["units-of-measure", debouncedName, sort],
    queryFn: () => apiRequest<UnitOfMeasure[]>(`/api/v1/units-of-measure?${params.toString()}`),
  });

  const {
    register,
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({ resolver: zodResolver(schema) });

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) =>
      apiRequest<{ id: number }>("/api/v1/units-of-measure", {
        method: "POST",
        body: JSON.stringify(values),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["units-of-measure"] });
      reset();
      setCreateOpen(false);
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const nameOf = (id: number | null) => units?.find((u) => u.id === id)?.name ?? "—";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-end">
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("inventory.newUnit")}</NewButton>
      </div>

      <DataTable<UnitOfMeasure>
        columns={[
          {
            key: "name",
            header: t("inventory.unitName"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={nameFilter} onChangeAction={setNameFilter} />,
            render: (u) => u.name,
          },
          {
            key: "baseUnitId",
            header: t("inventory.baseUnit"),
            tdClassName: "text-muted-foreground",
            render: (u) => (u.baseUnitId ? nameOf(u.baseUnitId) : t("inventory.none")),
          },
          {
            key: "conversionFactor",
            header: t("inventory.conversionFactor"),
            sortable: true,
            tdClassName: "tabular-nums text-muted-foreground",
            render: (u) => toPersianDigits(u.conversionFactor),
          },
        ]}
        rows={units}
        isLoading={isLoading}
        rowKeyAction={(u) => u.id}
        sort={sort}
        onSortAction={toggleSort}
      />

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("inventory.newUnit")}>
        <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <Field label={t("inventory.unitName")}>
              <input type="text" placeholder="کیلوگرم" {...register("name")} className={fieldInputClass} autoFocus />
            </Field>
            <Field label={t("inventory.baseUnit")}>
              <Controller
                name="baseUnitId"
                control={control}
                render={({ field }) => (
                  <Select
                    value={field.value ? String(field.value) : ""}
                    onChangeAction={(v) => field.onChange(v ? Number(v) : undefined)}
                    placeholder={t("inventory.none")}
                    className={fieldInputClass}
                    options={(units ?? []).filter((u) => !u.baseUnitId).map((u) => ({ value: String(u.id), label: u.name }))}
                  />
                )}
              />
            </Field>
            <Field label={t("inventory.conversionFactor")}>
              <input type="number" step="0.000001" placeholder="1000" {...register("conversionFactor")} className={fieldInputClass} />
            </Field>
          </div>
          {(errors.name || errors.root) && (
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
