"use client";

// Step 4.5 (docs/phase-4-treasury.md §4.5): the received-cheque register.
// **B15 fix**: every visible filter (status, aging cutoff) is wired to a
// real query parameter — unlike the legacy, where the equivalent controls
// exist but have no `OnClick` handler at all (§5.5) and the list always
// shows every cheque of every state and every fiscal year, unfiltered.

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import type { ChequeStatus, ChequeSummary } from "@/lib/treasury";

const schema = z.object({
  chequeNumber: z.string().optional(),
  receivedOn: z.string().min(1),
  dueDate: z.string().min(1),
  amount: z.coerce.number().positive(),
  description: z.string().min(1),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  amount_must_be_positive: "treasury.amountMustBePositive",
  description_required: "treasury.descriptionRequired",
  date_outside_fiscal_year: "treasury.dateOutsideFiscalYear",
  account_not_leaf: "treasury.accountNotLeaf",
  fiscal_year_closed: "vouchers.dateOutsideFiscalYear",
};

const STATUS_OPTIONS: ChequeStatus[] = ["in_hand", "at_bank", "bounced", "returned_to_issuer", "cleared", "endorsed_to_third_party"];
const STATUS_LABEL: Record<ChequeStatus, string> = {
  in_hand: "treasury.statusInHand",
  at_bank: "treasury.statusAtBank",
  bounced: "treasury.statusBounced",
  returned_to_issuer: "treasury.statusReturned",
  cleared: "treasury.statusCleared",
  endorsed_to_third_party: "treasury.statusEndorsed",
};

export function ChequeRegister({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const [statusFilter, setStatusFilter] = useState<ChequeStatus | "">("");
  const [dueBefore, setDueBefore] = useState("");
  const [payerAccountId, setPayerAccountId] = useState<number | null>(null);
  const [notesAccountId, setNotesAccountId] = useState<number | null>(null);

  const params = new URLSearchParams();
  if (fiscalYearId) params.set("fiscalYearId", String(fiscalYearId));
  if (statusFilter) params.set("status", statusFilter);
  if (dueBefore) params.set("dueBefore", dueBefore);

  const { data: cheques, isLoading } = useQuery({
    queryKey: ["received-cheques", fiscalYearId, statusFilter, dueBefore],
    queryFn: () => apiRequest<ChequeSummary[]>(`/api/v1/received-cheques?${params.toString()}`),
    enabled: fiscalYearId !== null,
  });

  const {
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({ resolver: zodResolver(schema) });

  const receiveMutation = useMutation({
    mutationFn: (values: FormOutput) => {
      if (!payerAccountId || !notesAccountId) {
        throw new Error("accounts_required");
      }
      return apiRequest<{ id: number }>("/api/v1/received-cheques", {
        method: "POST",
        body: JSON.stringify({
          ...values,
          fiscalYearId,
          payerAccountId,
          notesReceivableAccountId: notesAccountId,
        }),
      });
    },
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["received-cheques"] });
      reset();
      router.push(`/treasury/received-cheques/${result.id}`);
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
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("treasury.status")}</label>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as ChequeStatus | "")}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <option value="">{t("treasury.allStatuses")}</option>
            {STATUS_OPTIONS.map((s) => (
              <option key={s} value={s}>
                {t(STATUS_LABEL[s])}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("treasury.agingAsOf")}</label>
          <input
            type="date"
            value={dueBefore}
            onChange={(e) => setDueBefore(e.target.value)}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </div>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("treasury.status")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.chequeNumber")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.receivedOn")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.dueDate")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.amount")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.description")}</th>
              </tr>
            </thead>
            <tbody>
              {cheques?.map((c) => (
                <tr key={c.id} className="border-b border-border last:border-0 hover:bg-muted">
                  <td className="px-3 py-2 text-muted-foreground">{t(STATUS_LABEL[c.status])}</td>
                  <td className="px-3 py-2">
                    <Link
                      href={`/treasury/received-cheques/${c.id}`}
                      className="text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                    >
                      {c.chequeNumber ? toPersianDigits(c.chequeNumber) : "—"}
                    </Link>
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">{c.receivedOn}</td>
                  <td className="px-3 py-2 text-muted-foreground">{c.dueDate}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(c.amount)}</td>
                  <td className="px-3 py-2 text-foreground">{c.description}</td>
                </tr>
              ))}
              {cheques?.length === 0 && (
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
        onSubmit={handleSubmit((values) => receiveMutation.mutate(values))}
        className="flex flex-col gap-3 rounded-md border border-border p-3"
      >
        <h2 className="text-sm font-semibold text-foreground">{t("treasury.receiveNewCheque")}</h2>
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.chequeNumber")}</label>
            <input
              type="text"
              {...register("chequeNumber")}
              className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.receivedOn")}</label>
            <input
              type="date"
              {...register("receivedOn")}
              className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.dueDate")}</label>
            <input
              type="date"
              {...register("dueDate")}
              className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.amount")}</label>
            <input
              type="number"
              {...register("amount")}
              className="h-9 w-32 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.description")}</label>
            <input
              type="text"
              {...register("description")}
              className="h-9 w-64 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <AccountField label={t("treasury.payer")} value={payerAccountId} onChangeAction={setPayerAccountId} />
          <AccountField label={t("treasury.notesReceivableAccount")} value={notesAccountId} onChangeAction={setNotesAccountId} />
          <button
            type="submit"
            disabled={isSubmitting}
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
          >
            {t("treasury.save")}
          </button>
        </div>
        {(errors.chequeNumber || errors.receivedOn || errors.dueDate || errors.amount || errors.description || errors.root) && (
          <p role="alert" className="text-sm text-danger">
            {errors.root?.message ?? t("common.error")}
          </p>
        )}
      </form>
    </div>
  );
}
