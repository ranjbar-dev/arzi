"use client";

// Step 4.5: the deposit-slip register (legacy FishListD equivalent).

import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useState } from "react";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import type { DepositChannel, DepositSlip } from "@/lib/treasury";

const schema = z.object({
  slipNumber: z.string().optional(),
  slipDate: z.string().min(1),
  amount: z.coerce.number().positive(),
  description: z.string().optional(),
  channel: z.enum(["pos_terminal", "cash_slip", "card_to_card", "wire_transfer"]),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  amount_must_be_positive: "treasury.amountMustBePositive",
  invalid_channel: "common.error",
  date_outside_fiscal_year: "treasury.dateOutsideFiscalYear",
  account_not_leaf: "treasury.accountNotLeaf",
  voucher_not_draft: "treasury.voucherNotDraft",
};

const CHANNEL_OPTIONS: DepositChannel[] = ["pos_terminal", "cash_slip", "card_to_card", "wire_transfer"];
const CHANNEL_LABEL: Record<DepositChannel, string> = {
  pos_terminal: "treasury.channelPosTerminal",
  cash_slip: "treasury.channelCashSlip",
  card_to_card: "treasury.channelCardToCard",
  wire_transfer: "treasury.channelWireTransfer",
};

export function SlipRegister({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [payerAccountId, setPayerAccountId] = useState<number | null>(null);
  const [bankAccountId, setBankAccountId] = useState<number | null>(null);

  const { data: slips, isLoading } = useQuery({
    queryKey: ["deposit-slips", fiscalYearId],
    queryFn: () => apiRequest<DepositSlip[]>(`/api/v1/deposit-slips${fiscalYearId ? `?fiscalYearId=${fiscalYearId}` : ""}`),
    enabled: fiscalYearId !== null,
  });

  const {
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({ resolver: zodResolver(schema), defaultValues: { channel: "pos_terminal" } });

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) => {
      if (!payerAccountId || !bankAccountId) throw new Error("accounts_required");
      return apiRequest<{ id: number }>("/api/v1/deposit-slips", {
        method: "POST",
        body: JSON.stringify({ ...values, fiscalYearId, payerAccountId, bankAccountId }),
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["deposit-slips"] });
      reset();
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => apiRequest(`/api/v1/deposit-slips/${id}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["deposit-slips"] }),
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
                <th className="px-3 py-2 text-start font-medium">{t("treasury.channel")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.slipNumber")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.slipDate")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.amount")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.description")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("common.actions")}</th>
              </tr>
            </thead>
            <tbody>
              {slips?.map((s) => (
                <tr key={s.id} className="border-b border-border last:border-0 hover:bg-muted">
                  <td className="px-3 py-2 text-muted-foreground">{t(CHANNEL_LABEL[s.channel])}</td>
                  <td className="px-3 py-2 text-foreground">{s.slipNumber ? toPersianDigits(s.slipNumber) : "—"}</td>
                  <td className="px-3 py-2 text-muted-foreground">{s.slipDate}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(s.amount)}</td>
                  <td className="px-3 py-2 text-foreground">{s.description}</td>
                  <td className="px-3 py-2">
                    <button
                      type="button"
                      onClick={() => deleteMutation.mutate(s.id)}
                      className="cursor-pointer text-sm text-danger hover:underline focus-visible:ring-2 focus-visible:ring-danger"
                    >
                      {t("treasury.deleteSlip")}
                    </button>
                  </td>
                </tr>
              ))}
              {slips?.length === 0 && (
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
        <h2 className="text-sm font-semibold text-foreground">{t("treasury.newDepositSlip")}</h2>
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.slipNumber")}</label>
            <input type="text" {...register("slipNumber")} className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.slipDate")}</label>
            <input type="date" {...register("slipDate")} className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.amount")}</label>
            <input type="number" {...register("amount")} className="h-9 w-32 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.channel")}</label>
            <select {...register("channel")} className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent">
              {CHANNEL_OPTIONS.map((c) => (
                <option key={c} value={c}>
                  {t(CHANNEL_LABEL[c])}
                </option>
              ))}
            </select>
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.description")}</label>
            <input type="text" {...register("description")} className="h-9 w-64 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
          </div>
          <AccountField label={t("treasury.depositor")} value={payerAccountId} onChangeAction={setPayerAccountId} />
          <AccountField label={t("treasury.bankCode")} value={bankAccountId} onChangeAction={setBankAccountId} />
          <button
            type="submit"
            disabled={isSubmitting}
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
          >
            {t("treasury.save")}
          </button>
        </div>
        {(errors.slipDate || errors.amount || errors.channel || errors.root) && (
          <p role="alert" className="text-sm text-danger">
            {errors.root?.message ?? t("common.error")}
          </p>
        )}
      </form>
    </div>
  );
}
