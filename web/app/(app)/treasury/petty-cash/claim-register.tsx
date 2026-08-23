"use client";

// Step 4.5: the petty-cash claim register (legacy TankhahList equivalent).
// Dynamic expense-line rows (legacy TankhahEditAddu's modal line dialog,
// collapsed into inline rows here — see petty_cash.rs's own doc comment on
// why claims are whole-form saves, not incremental line CRUD).

import { useForm, useFieldArray } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useState } from "react";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import type { PettyCashClaimSummary } from "@/lib/treasury";

const schema = z.object({
  claimNumber: z.string().optional(),
  claimDate: z.string().min(1),
  description: z.string().optional(),
  lines: z
    .array(
      z.object({
        expenseAccountId: z.number().positive(),
        amount: z.coerce.number().positive(),
        description: z.string().optional(),
      }),
    )
    .min(1),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  at_least_one_line_required: "treasury.atLeastOneLineRequired",
  line_amount_must_be_positive: "treasury.amountMustBePositive",
  date_outside_fiscal_year: "treasury.dateOutsideFiscalYear",
  account_not_leaf: "treasury.accountNotLeaf",
  voucher_not_draft: "treasury.voucherNotDraft",
};

export function ClaimRegister({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [custodianAccountId, setCustodianAccountId] = useState<number | null>(null);

  const { data: claims, isLoading } = useQuery({
    queryKey: ["petty-cash-claims", fiscalYearId],
    queryFn: () => apiRequest<PettyCashClaimSummary[]>(`/api/v1/petty-cash-claims${fiscalYearId ? `?fiscalYearId=${fiscalYearId}` : ""}`),
    enabled: fiscalYearId !== null,
  });

  const {
    register,
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({
    resolver: zodResolver(schema),
    defaultValues: { lines: [{ expenseAccountId: 0, amount: undefined, description: "" }] },
  });
  const { fields, append, remove } = useFieldArray({ control, name: "lines" });

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) => {
      if (!custodianAccountId) throw new Error("accounts_required");
      return apiRequest<{ id: number }>("/api/v1/petty-cash-claims", {
        method: "POST",
        body: JSON.stringify({ ...values, fiscalYearId, custodianAccountId }),
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["petty-cash-claims"] });
      reset({ lines: [{ expenseAccountId: undefined, amount: undefined, description: "" }] });
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => apiRequest(`/api/v1/petty-cash-claims/${id}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["petty-cash-claims"] }),
  });

  if (fiscalYearId === null) {
    return <p className="text-sm text-muted-foreground">{t("shell.noFiscalYear")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("treasury.claimNumber")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.claimDate")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.listTotal")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.lineCount")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.description")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("common.actions")}</th>
              </tr>
            </thead>
            <tbody>
              {claims?.map((c) => (
                <tr key={c.id} className="border-b border-border last:border-0 hover:bg-muted">
                  <td className="px-3 py-2 text-foreground">{c.claimNumber ? toPersianDigits(c.claimNumber) : "—"}</td>
                  <td className="px-3 py-2 text-muted-foreground">{c.claimDate}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(c.totalAmount)}</td>
                  <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(c.lineCount)}</td>
                  <td className="px-3 py-2 text-foreground">{c.description}</td>
                  <td className="px-3 py-2">
                    <button
                      type="button"
                      onClick={() => deleteMutation.mutate(c.id)}
                      className="cursor-pointer text-sm text-danger hover:underline focus-visible:ring-2 focus-visible:ring-danger"
                    >
                      {t("treasury.deleteClaim")}
                    </button>
                  </td>
                </tr>
              ))}
              {claims?.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-3 py-6 text-center text-sm text-muted-foreground">
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
        className="flex flex-col gap-3 rounded-md border border-border p-3"
      >
        <h2 className="text-sm font-semibold text-foreground">{t("treasury.newClaim")}</h2>
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.claimNumber")}</label>
            <input type="text" {...register("claimNumber")} className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.claimDate")}</label>
            <input type="date" {...register("claimDate")} className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.description")}</label>
            <input type="text" {...register("description")} className="h-9 w-64 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <AccountField label={t("treasury.custodian")} value={custodianAccountId} onChangeAction={setCustodianAccountId} />
        </div>

        <div className="flex flex-col gap-2">
          <h3 className="text-sm font-medium text-foreground">{t("treasury.expenseLines")}</h3>
          {fields.map((field, index) => (
            <div key={field.id} className="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
              <ExpenseAccountField control={control} index={index} />
              <div className="flex flex-col gap-1">
                <label className="text-sm text-muted-foreground">{t("treasury.amount")}</label>
                <input
                  type="number"
                  {...register(`lines.${index}.amount`)}
                  className="h-9 w-28 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-sm text-muted-foreground">{t("treasury.description")}</label>
                <input
                  type="text"
                  {...register(`lines.${index}.description`)}
                  className="h-9 w-48 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              </div>
              {fields.length > 1 && (
                <button
                  type="button"
                  onClick={() => remove(index)}
                  className="h-9 cursor-pointer rounded-md border border-danger px-3 text-sm text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-danger"
                >
                  {t("treasury.removeLine")}
                </button>
              )}
            </div>
          ))}
          <button
            type="button"
            onClick={() => append({ expenseAccountId: 0, amount: undefined, description: "" })}
            className="h-9 w-fit cursor-pointer rounded-md border border-border px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("treasury.addLine")}
          </button>
        </div>

        <button
          type="submit"
          disabled={isSubmitting}
          className="h-9 w-fit cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
        >
          {t("treasury.save")}
        </button>
        {(errors.claimDate || errors.lines || errors.root) && (
          <p role="alert" className="text-sm text-danger">
            {errors.root?.message ?? t("common.error")}
          </p>
        )}
      </form>
    </div>
  );
}

/** One expense-line row's account picker — a real `AccountSummary` id is
 * only known once the operator picks one, but `react-hook-form`'s
 * `setValue` still needs a field name, so this small wrapper is kept
 * separate rather than inlined into the map above. */
import { useController, type Control } from "react-hook-form";

function ExpenseAccountField({ control, index }: { control: Control<FormValues>; index: number }) {
  const { field } = useController({ control, name: `lines.${index}.expenseAccountId` });
  return <AccountField value={field.value || null} onChangeAction={(id) => field.onChange(id)} />;
}
