"use client";

// Step 5.9: `AnbarCalaU`/`AnbarCalaAddU`'s equivalent (specs/05-inventory §2.2-§2.5) — item
// master list + create form, including the real many-to-many warehouse assignment (5.1's fix for
// the legacy's single-scalar `AJ_ID`) and the low-stock alert (5.3's `isLowStock`, closing 5.1's
// own deferred manual test #4).

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
import type { Item, PistachioGrade, UnitOfMeasure, Warehouse } from "@/lib/inventory";

const schema = z.object({
  code: z.coerce.number().int(),
  name: z.string().min(1),
  specification: z.string().optional(),
  unitOfMeasureId: z.coerce.number().int().positive(),
  salePrice: z.coerce.number().int(),
  minStock: z.coerce.number().int().default(0),
  isTaxable: z.boolean().optional(),
  allowNegativeStock: z.boolean().optional(),
  pistachioGradeId: z.coerce.number().int().positive().optional(),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  invalid_name: "inventory.nameRequired",
  sale_price_required: "inventory.saleRequired",
  duplicate_code: "inventory.duplicateCode",
  unit_of_measure_not_found: "common.error",
  pistachio_grade_not_found: "common.error",
};

export function ItemRegister() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [warehouseIds, setWarehouseIds] = useState<number[]>([]);
  const [createOpen, setCreateOpen] = useState(false);

  const { data: items, isLoading } = useQuery({
    queryKey: ["items"],
    queryFn: () => apiRequest<Item[]>("/api/v1/items"),
  });
  const { data: units } = useQuery({
    queryKey: ["units-of-measure"],
    queryFn: () => apiRequest<UnitOfMeasure[]>("/api/v1/units-of-measure"),
  });
  const { data: warehouses } = useQuery({
    queryKey: ["warehouses"],
    queryFn: () => apiRequest<Warehouse[]>("/api/v1/warehouses?activeOnly=true"),
  });
  const { data: grades } = useQuery({
    queryKey: ["pistachio-grades"],
    queryFn: () => apiRequest<PistachioGrade[]>("/api/v1/pistachio-grades"),
  });

  const {
    register,
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({ resolver: zodResolver(schema), defaultValues: { allowNegativeStock: true } });

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) =>
      apiRequest<{ id: number }>("/api/v1/items", {
        method: "POST",
        body: JSON.stringify({ ...values, warehouseIds }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["items"] });
      reset();
      setWarehouseIds([]);
      setCreateOpen(false);
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const unitName = (id: number) => units?.find((u) => u.id === id)?.name ?? "—";

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-end">
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("inventory.newItem")}</NewButton>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("inventory.itemCode")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.itemName")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.unitOfMeasure")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.salePrice")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.minStock")}</th>
              </tr>
            </thead>
            <tbody>
              {items?.map((i) => (
                <tr key={i.id} className="border-b border-border last:border-0 hover:bg-muted">
                  <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(i.code)}</td>
                  <td className="px-3 py-2 text-foreground">{i.name}</td>
                  <td className="px-3 py-2 text-muted-foreground">{unitName(i.unitOfMeasureId)}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(i.salePrice)}</td>
                  <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(i.minStock)}</td>
                </tr>
              ))}
              {items?.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-3 py-6 text-center text-sm text-muted-foreground">
                    —
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("inventory.newItem")}>
        <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <Field label={t("inventory.itemCode")}>
              <input type="number" placeholder="101" {...register("code")} className={fieldInputClass} autoFocus />
            </Field>
            <Field label={t("inventory.itemName")} wide>
              <input type="text" placeholder="پسته اکبری درجه یک" {...register("name")} className={fieldInputClass} />
            </Field>
            <Field label={t("inventory.specification")}>
              <input type="text" placeholder="بسته ۵۰۰ گرمی" {...register("specification")} className={fieldInputClass} />
            </Field>
            <Field label={t("inventory.unitOfMeasure")}>
              <Controller
                name="unitOfMeasureId"
                control={control}
                render={({ field }) => (
                  <Select
                    value={field.value ? String(field.value) : ""}
                    onChangeAction={(v) => field.onChange(v ? Number(v) : undefined)}
                    placeholder="—"
                    className={fieldInputClass}
                    options={(units ?? []).map((u) => ({ value: String(u.id), label: u.name }))}
                  />
                )}
              />
            </Field>
            <Field label={t("inventory.salePrice")}>
              <input type="number" placeholder="250000" {...register("salePrice")} className={fieldInputClass} />
            </Field>
            <Field label={t("inventory.minStock")}>
              <input type="number" placeholder="50" {...register("minStock")} className={fieldInputClass} />
            </Field>
            <Field label={t("inventory.pistachioGrade")}>
              <Controller
                name="pistachioGradeId"
                control={control}
                render={({ field }) => (
                  <Select
                    value={field.value ? String(field.value) : ""}
                    onChangeAction={(v) => field.onChange(v ? Number(v) : undefined)}
                    placeholder={t("inventory.none")}
                    className={fieldInputClass}
                    options={(grades ?? []).map((g) => ({ value: String(g.id), label: g.name }))}
                  />
                )}
              />
            </Field>
          </div>
          <div className="flex flex-wrap items-center gap-4">
            <label className="flex items-center gap-2 text-sm text-foreground">
              <input type="checkbox" {...register("isTaxable")} className="h-4 w-4 accent-accent" />
              {t("inventory.taxable")}
            </label>
            <label className="flex items-center gap-2 text-sm text-foreground">
              <input type="checkbox" {...register("allowNegativeStock")} className="h-4 w-4 accent-accent" />
              {t("inventory.allowNegativeStock")}
            </label>
          </div>
          <div className="flex flex-col gap-1">
            <span className="text-sm text-muted-foreground">{t("inventory.assignedWarehouses")}</span>
            <div className="flex flex-wrap gap-3">
              {warehouses?.map((w) => (
                <label key={w.id} className="flex items-center gap-2 text-sm text-foreground">
                  <input
                    type="checkbox"
                    checked={warehouseIds.includes(w.id)}
                    onChange={(e) =>
                      setWarehouseIds((ids) => (e.target.checked ? [...ids, w.id] : ids.filter((id) => id !== w.id)))
                    }
                    className="h-4 w-4 accent-accent"
                  />
                  {w.name}
                </label>
              ))}
            </div>
          </div>
          {(errors.name || errors.code || errors.salePrice || errors.unitOfMeasureId || errors.root) && (
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
