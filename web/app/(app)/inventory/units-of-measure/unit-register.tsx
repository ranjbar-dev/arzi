"use client";

// Step 5.9: `Anbar_Vahed`'s first-ever maintenance screen (specs/05-inventory §1.3 — "no
// maintenance screen for it" in the legacy, "must be populated by direct SQL").

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
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

  const { data: units, isLoading } = useQuery({
    queryKey: ["units-of-measure"],
    queryFn: () => apiRequest<UnitOfMeasure[]>("/api/v1/units-of-measure"),
  });

  const {
    register,
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
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const nameOf = (id: number | null) => units?.find((u) => u.id === id)?.name ?? "—";

  return (
    <div className="flex flex-col gap-4">
      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("inventory.unitName")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.baseUnit")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.conversionFactor")}</th>
              </tr>
            </thead>
            <tbody>
              {units?.map((u) => (
                <tr key={u.id} className="border-b border-border last:border-0 hover:bg-muted">
                  <td className="px-3 py-2 text-foreground">{u.name}</td>
                  <td className="px-3 py-2 text-muted-foreground">{u.baseUnitId ? nameOf(u.baseUnitId) : t("inventory.none")}</td>
                  <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(u.conversionFactor)}</td>
                </tr>
              ))}
              {units?.length === 0 && (
                <tr>
                  <td colSpan={3} className="px-3 py-6 text-center text-sm text-muted-foreground">
                    —
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      <form
        onSubmit={handleSubmit((values) => createMutation.mutate(values))}
        className="flex flex-wrap items-end gap-3 rounded-md border border-border p-3"
      >
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.unitName")}</label>
          <input
            type="text"
            {...register("name")}
            className="h-9 w-40 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.baseUnit")}</label>
          <select
            {...register("baseUnitId")}
            className="h-9 w-40 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <option value="">{t("inventory.none")}</option>
            {units?.filter((u) => !u.baseUnitId).map((u) => (
              <option key={u.id} value={u.id}>
                {u.name}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.conversionFactor")}</label>
          <input
            type="number"
            step="0.000001"
            {...register("conversionFactor")}
            className="h-9 w-32 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </div>
        <button
          type="submit"
          disabled={isSubmitting}
          className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
        >
          {t("inventory.newUnit")}
        </button>
        {(errors.name || errors.root) && (
          <p role="alert" className="text-sm text-danger">
            {errors.root?.message ?? t("common.error")}
          </p>
        )}
      </form>
    </div>
  );
}
