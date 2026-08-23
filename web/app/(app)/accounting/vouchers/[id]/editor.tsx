"use client";

// Step 2.4 (docs/phase-2-accounting-core.md §2.4): the on-screen equivalent
// of `SanadEditU`. Unlike the legacy's single "edit the whole grid, replace
// all manual lines on Save" model, lines are added/edited/deleted
// individually against 2.3's per-line endpoints (same departure documented
// in api/src/vouchers.rs) — so the balance indicator is "live" in the sense
// that it reflects the server's maintained totals after every line
// mutation, not a client-side draft buffer.

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import type { VoucherDetail } from "@/lib/vouchers";
import { AccountPicker } from "@/components/account-picker";
import { AccountLabel } from "@/components/account-label";
import { LockIcon } from "@/components/lock-icon";

const lineSchema = z
  .object({
    accountId: z.number().int().positive(),
    debit: z.number().int().min(0),
    credit: z.number().int().min(0),
    description: z.string().min(1),
  })
  .refine((v) => (v.debit > 0) !== (v.credit > 0), { message: "vouchers.bothSidesFilled", path: ["debit"] });
type LineValues = z.infer<typeof lineSchema>;

const ERROR_KEYS: Record<string, string> = {
  voucher_empty: "vouchers.voucherEmpty",
  voucher_not_balanced: "vouchers.voucherNotBalanced",
  not_draft: "vouchers.notDraft",
  voucher_locked: "vouchers.voucherLocked",
  account_not_leaf: "vouchers.accountNotLeaf",
  amount_required: "vouchers.amountRequired",
  both_sides_filled: "vouchers.bothSidesFilled",
  description_required: "vouchers.voucherEmpty",
  generated_line_immutable: "vouchers.generatedLine",
  has_generated_lines: "vouchers.hasGeneratedLines",
  fiscal_year_closed: "fiscalYears.alreadyClosed",
  duplicate_voucher_number: "vouchers.duplicateVoucherNumber",
  date_outside_fiscal_year: "vouchers.dateOutsideFiscalYear",
  invalid_voucher_number: "vouchers.duplicateVoucherNumber",
};

const headerSchema = z.object({
  voucherNumber: z.number().int().positive(),
  voucherDate: z.string().min(1),
  description: z.string().min(1),
});
type HeaderValues = z.infer<typeof headerSchema>;

const STATUS_LABEL: Record<string, string> = {
  draft: "vouchers.draft",
  confirmed: "vouchers.confirmed",
  posted: "vouchers.posted",
};

export function VoucherEditor({ voucherId }: { voucherId: number }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const [actionError, setActionError] = useState<string | null>(null);
  const [pickedAccountLabel, setPickedAccountLabel] = useState<string | null>(null);
  const [editingHeader, setEditingHeader] = useState(false);

  const { data: voucher, isLoading } = useQuery({
    queryKey: ["vouchers", voucherId],
    queryFn: () => apiRequest<VoucherDetail>(`/api/v1/vouchers/${voucherId}`),
  });

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["vouchers", voucherId] });
  function reportError(err: unknown) {
    const code = err instanceof ApiError ? err.message : "internal_error";
    setActionError(t(ERROR_KEYS[code] ?? "common.error"));
  }

  const lineForm = useForm<LineValues>({ resolver: zodResolver(lineSchema) });
  const addLineMutation = useMutation({
    mutationFn: (values: LineValues) =>
      apiRequest(`/api/v1/vouchers/${voucherId}/lines`, { method: "POST", body: JSON.stringify(values) }),
    onSuccess: () => {
      lineForm.reset({ accountId: undefined, debit: 0, credit: 0, description: "" });
      setPickedAccountLabel(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const deleteLineMutation = useMutation({
    mutationFn: (lineId: number) =>
      apiRequest(`/api/v1/vouchers/${voucherId}/lines/${lineId}`, { method: "DELETE" }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const transitionMutation = useMutation({
    mutationFn: (to: string) =>
      apiRequest(`/api/v1/vouchers/${voucherId}/transition`, { method: "POST", body: JSON.stringify({ to }) }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const lockMutation = useMutation({
    mutationFn: (lock: boolean) =>
      apiRequest(`/api/v1/vouchers/${voucherId}/${lock ? "lock" : "unlock"}`, { method: "POST" }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const deleteVoucherMutation = useMutation({
    mutationFn: () => apiRequest(`/api/v1/vouchers/${voucherId}`, { method: "DELETE" }),
    onSuccess: () => router.push("/accounting/vouchers"),
    onError: reportError,
  });

  const headerForm = useForm<HeaderValues>({ resolver: zodResolver(headerSchema) });
  const updateHeaderMutation = useMutation({
    mutationFn: (values: HeaderValues) =>
      apiRequest(`/api/v1/vouchers/${voucherId}`, { method: "PUT", body: JSON.stringify(values) }),
    onSuccess: () => {
      setEditingHeader(false);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  if (isLoading || !voucher) {
    return <p className="text-sm text-muted-foreground">{t("common.loading")}</p>;
  }

  const isDraft = voucher.status === "draft";
  const balanced = voucher.totalDebit === voucher.totalCredit;

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div>
          <h1 className="text-lg font-semibold text-foreground">
            {t("vouchers.voucherNumber")}: {toPersianDigits(voucher.voucherNumber)}
          </h1>
          <p className="text-sm text-muted-foreground">
            {voucher.voucherDate} · {voucher.description}
          </p>
        </div>
        <div className="flex flex-wrap items-center gap-2">
          <span className="rounded-full border border-border px-3 py-1 text-xs text-muted-foreground">
            {t(STATUS_LABEL[voucher.status])}
          </span>
          {voucher.isLocked && (
            <span className="flex items-center gap-1 text-xs text-warning">
              <LockIcon locked className="h-3.5 w-3.5" />
              {t("vouchers.voucherLocked")}
            </span>
          )}
          <span className={`rounded-full px-3 py-1 text-xs ${balanced ? "bg-success/10 text-success" : "bg-danger/10 text-danger"}`}>
            {balanced ? t("vouchers.balanced") : t("vouchers.unbalanced")}
          </span>
          {isDraft && !voucher.isLocked && (
            <button
              type="button"
              onClick={() => {
                headerForm.reset({
                  voucherNumber: voucher.voucherNumber,
                  voucherDate: voucher.voucherDate,
                  description: voucher.description ?? "",
                });
                setEditingHeader((v) => !v);
              }}
              className="cursor-pointer rounded-md border border-border bg-surface px-3 py-1 text-xs text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("vouchers.editHeader")}
            </button>
          )}
        </div>
      </div>

      {editingHeader && (
        <form
          onSubmit={headerForm.handleSubmit((values) => updateHeaderMutation.mutate(values))}
          className="flex flex-wrap items-end gap-3 rounded-md border border-border bg-surface p-3"
        >
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("vouchers.voucherNumber")}</label>
            <input
              type="number"
              {...headerForm.register("voucherNumber", { valueAsNumber: true })}
              className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("vouchers.voucherDate")}</label>
            <input
              type="date"
              {...headerForm.register("voucherDate")}
              className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("vouchers.description")}</label>
            <input
              type="text"
              {...headerForm.register("description")}
              className="h-9 w-64 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <button
            type="submit"
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.save")}
          </button>
          <button
            type="button"
            onClick={() => setEditingHeader(false)}
            className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.cancel")}
          </button>
        </form>
      )}

      {actionError && (
        <p role="alert" className="text-sm text-danger">
          {actionError}
        </p>
      )}

      <div className="overflow-x-auto rounded-md border border-border">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/50 text-muted-foreground">
              <th className="px-3 py-2 text-start font-medium">{t("vouchers.account")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("vouchers.lineDescription")}</th>
              <th className="w-28 px-3 py-2 text-end font-medium">{t("vouchers.debit")}</th>
              <th className="w-28 px-3 py-2 text-end font-medium">{t("vouchers.credit")}</th>
              <th className="w-20 px-3 py-2 text-center font-medium">{t("common.actions")}</th>
            </tr>
          </thead>
          <tbody>
            {voucher.lines.map((line) => {
              const isManual = line.sourceModule === 0;
              return (
                <tr key={line.id} className={`border-b border-border last:border-0 ${!isManual ? "bg-muted/30" : ""}`}>
                  <td className="px-3 py-2 text-foreground">
                    <AccountLabel accountId={line.accountId} />
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">
                    {line.description}
                    {!isManual && (
                      <span className="ms-2 text-xs text-warning" title={t("vouchers.generatedLine")}>
                        ●
                      </span>
                    )}
                  </td>
                  <td className="tabular-nums px-3 py-2 text-end text-foreground">
                    {line.debitAmount > 0 ? toPersianDigits(line.debitAmount) : ""}
                  </td>
                  <td className="tabular-nums px-3 py-2 text-end text-foreground">
                    {line.creditAmount > 0 ? toPersianDigits(line.creditAmount) : ""}
                  </td>
                  <td className="px-3 py-2 text-center">
                    {isDraft && isManual && (
                      <button
                        type="button"
                        onClick={() => deleteLineMutation.mutate(line.id)}
                        className="cursor-pointer text-danger hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                      >
                        {t("common.delete")}
                      </button>
                    )}
                  </td>
                </tr>
              );
            })}
            {voucher.lines.length === 0 && (
              <tr>
                <td colSpan={5} className="px-3 py-6 text-center text-sm text-muted-foreground">
                  —
                </td>
              </tr>
            )}
          </tbody>
          <tfoot>
            <tr className="border-t border-border font-medium text-foreground">
              <td className="px-3 py-2" colSpan={2} />
              <td className="tabular-nums px-3 py-2 text-end">{toPersianDigits(voucher.totalDebit)}</td>
              <td className="tabular-nums px-3 py-2 text-end">{toPersianDigits(voucher.totalCredit)}</td>
              <td />
            </tr>
          </tfoot>
        </table>
      </div>

      {isDraft && !voucher.isLocked && (
        <form
          onSubmit={lineForm.handleSubmit((values) => addLineMutation.mutate(values))}
          className="flex flex-wrap items-end gap-3 rounded-md border border-border bg-surface p-3"
        >
          <div className="flex flex-col gap-1">
            <span className="text-sm text-muted-foreground">{t("vouchers.account")}</span>
            <AccountPicker
              triggerLabel={pickedAccountLabel ?? t("accounts.selectAccount")}
              onSelect={(account) => {
                lineForm.setValue("accountId", account.id, { shouldValidate: true });
                setPickedAccountLabel(`${account.name} (${account.generalLedgerCode})`);
              }}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("vouchers.debit")}</label>
            <input
              type="number"
              {...lineForm.register("debit", { valueAsNumber: true })}
              className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("vouchers.credit")}</label>
            <input
              type="number"
              {...lineForm.register("credit", { valueAsNumber: true })}
              className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("vouchers.lineDescription")}</label>
            <input
              type="text"
              {...lineForm.register("description")}
              className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <button
            type="submit"
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("vouchers.addLine")}
          </button>
        </form>
      )}

      <div className="flex flex-wrap gap-2">
        {isDraft && (
          <button
            type="button"
            onClick={() => transitionMutation.mutate("confirmed")}
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("vouchers.approve")}
          </button>
        )}
        {voucher.status === "confirmed" && (
          <>
            <button
              type="button"
              onClick={() => transitionMutation.mutate("draft")}
              className="h-9 cursor-pointer rounded-md border border-border bg-surface px-4 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("vouchers.revertToDraft")}
            </button>
            <button
              type="button"
              onClick={() => transitionMutation.mutate("posted")}
              className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("vouchers.postPermanently")}
            </button>
          </>
        )}
        {voucher.status === "posted" && (
          <button
            type="button"
            onClick={() => transitionMutation.mutate("confirmed")}
            className="h-9 cursor-pointer rounded-md border border-border bg-surface px-4 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("vouchers.revertToConfirmed")}
          </button>
        )}
        <button
          type="button"
          onClick={() => lockMutation.mutate(!voucher.isLocked)}
          className="flex h-9 cursor-pointer items-center gap-1.5 rounded-md border border-border bg-surface px-4 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
        >
          <LockIcon locked={voucher.isLocked} />
          {t("vouchers.lockVoucher")}
        </button>
        {isDraft && (
          <button
            type="button"
            onClick={() => {
              if (confirm(t("vouchers.deleteVoucher") + "?")) deleteVoucherMutation.mutate();
            }}
            className="h-9 cursor-pointer rounded-md border border-danger px-4 text-sm text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("vouchers.deleteVoucher")}
          </button>
        )}
      </div>
    </div>
  );
}
