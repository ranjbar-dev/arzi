"use client";

// Step 4.5: the received-cheque detail — event history plus the transition
// forms appropriate for the cheque's current status. Manual test #3:
// "trigger every transition (deposit, bounce, collect, return, endorse,
// delete) from the register UI → confirm each opens the correct screen and
// updates the list on completion."
//
// Each transition gets its OWN `useForm` instance (`TransitionForm` below) —
// sharing one `register("eventDate")` across several simultaneously-mounted
// `<form>` elements silently breaks react-hook-form's field-ref tracking
// (only the last-registered input actually holds a value); caught live in a
// real browser, not by `next build`/`tsc`/`eslint`, none of which flag it.

import { useState } from "react";
import { useForm, useController } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import { AccountLabel } from "@/components/account-label";
import { DateField } from "@/components/date-field";
import type { ChequeDetail as ChequeDetailType, ChequeStatus } from "@/lib/treasury";

const STATUS_LABEL: Record<ChequeStatus, string> = {
  in_hand: "treasury.statusInHand",
  at_bank: "treasury.statusAtBank",
  bounced: "treasury.statusBounced",
  returned_to_issuer: "treasury.statusReturned",
  cleared: "treasury.statusCleared",
  endorsed_to_third_party: "treasury.statusEndorsed",
};

const ERROR_KEYS: Record<string, string> = {
  account_not_leaf: "treasury.accountNotLeaf",
  date_outside_fiscal_year: "treasury.dateOutsideFiscalYear",
  fiscal_year_closed: "vouchers.dateOutsideFiscalYear",
  cheque_not_deletable: "treasury.chequeNotDeletable",
  cheque_has_transition_history: "treasury.chequeHasTransitionHistory",
};

const transitionSchema = z.object({ eventDate: z.string().min(1), description: z.string().optional() });
type TransitionValues = z.infer<typeof transitionSchema>;

function TransitionForm({
  chequeId,
  fiscalYearId,
  action,
  title,
  accountField,
  onSuccessAction,
}: {
  chequeId: number;
  fiscalYearId: number | null;
  action: string;
  title: string;
  accountField?: { label: string; paramName: string };
  onSuccessAction: () => void;
}) {
  const { t } = useTranslation();
  const [accountId, setAccountId] = useState<number | null>(null);
  const {
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors },
  } = useForm<TransitionValues>({ resolver: zodResolver(transitionSchema) });
  const { field: eventDateField } = useController({ control, name: "eventDate" });

  const mutation = useMutation({
    mutationFn: (values: TransitionValues) =>
      apiRequest(`/api/v1/received-cheques/${chequeId}/${action}`, {
        method: "POST",
        body: JSON.stringify({
          ...values,
          fiscalYearId,
          ...(accountField ? { [accountField.paramName]: accountId } : {}),
        }),
      }),
    onSuccess: () => {
      reset();
      onSuccessAction();
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  return (
    <form
      onSubmit={handleSubmit((values) => mutation.mutate(values))}
      className="flex flex-wrap items-end gap-2 rounded-md border border-border p-3"
    >
      <h2 className="w-full text-sm font-semibold text-foreground">{title}</h2>
      <DateField
        label={t("treasury.eventDate")}
        value={eventDateField.value}
        onChangeAction={eventDateField.onChange}
      />
      {accountField && <AccountField label={accountField.label} value={accountId} onChangeAction={setAccountId} />}
      <button
        type="submit"
        className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
      >
        {t("treasury.save")}
      </button>
      {(errors.eventDate || errors.root) && (
        <p role="alert" className="w-full text-sm text-danger">
          {errors.root?.message ?? t("common.error")}
        </p>
      )}
    </form>
  );
}

export function ChequeDetail({ chequeId, fiscalYearId }: { chequeId: number; fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();

  const { data: cheque, isLoading } = useQuery({
    queryKey: ["received-cheques", chequeId],
    queryFn: () => apiRequest<ChequeDetailType>(`/api/v1/received-cheques/${chequeId}`),
  });

  function refresh() {
    queryClient.invalidateQueries({ queryKey: ["received-cheques"] });
  }

  const deleteMutation = useMutation({
    mutationFn: () => apiRequest(`/api/v1/received-cheques/${chequeId}`, { method: "DELETE" }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["received-cheques"] });
      router.push("/treasury/received-cheques");
    },
  });

  if (isLoading || !cheque) {
    return <p className="text-sm text-muted-foreground">{t("common.loading")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="rounded-md border border-border p-4">
        <dl className="grid grid-cols-2 gap-x-6 gap-y-2 text-sm sm:grid-cols-3">
          <div>
            <dt className="text-muted-foreground">{t("treasury.status")}</dt>
            <dd className="text-foreground">{t(STATUS_LABEL[cheque.status])}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("treasury.chequeNumber")}</dt>
            <dd className="text-foreground">{cheque.chequeNumber ? toPersianDigits(cheque.chequeNumber) : "—"}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("treasury.amount")}</dt>
            <dd className="tabular-nums text-foreground">{toPersianDigits(cheque.amount)}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("treasury.receivedOn")}</dt>
            <dd className="text-foreground">{cheque.receivedOn}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("treasury.dueDate")}</dt>
            <dd className="text-foreground">{cheque.dueDate}</dd>
          </div>
          <div>
            <dt className="text-muted-foreground">{t("treasury.payer")}</dt>
            <dd className="text-foreground">
              <AccountLabel accountId={cheque.payerAccountId} />
            </dd>
          </div>
        </dl>
      </div>

      {(cheque.status === "in_hand" || cheque.status === "bounced") && (
        <div className="flex flex-wrap gap-2">
          <TransitionForm
            chequeId={chequeId}
            fiscalYearId={fiscalYearId}
            action="deposit"
            title={t("treasury.depositToBank")}
            accountField={{ label: t("treasury.collectionAccount"), paramName: "collectionAccountId" }}
            onSuccessAction={refresh}
          />
          <TransitionForm
            chequeId={chequeId}
            fiscalYearId={fiscalYearId}
            action="return-to-issuer"
            title={t("treasury.returnToIssuer")}
            onSuccessAction={refresh}
          />
          <TransitionForm
            chequeId={chequeId}
            fiscalYearId={fiscalYearId}
            action="endorse"
            title={t("treasury.endorse")}
            accountField={{ label: t("treasury.beneficiaryAccount"), paramName: "beneficiaryAccountId" }}
            onSuccessAction={refresh}
          />
          {cheque.status === "in_hand" && cheque.events.length === 1 && (
            <button
              type="button"
              onClick={() => deleteMutation.mutate()}
              className="h-9 cursor-pointer self-start rounded-md border border-danger px-4 text-sm font-medium text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-danger"
            >
              {t("treasury.deleteCheque")}
            </button>
          )}
        </div>
      )}

      {cheque.status === "at_bank" && (
        <div className="flex flex-wrap gap-2">
          <TransitionForm
            chequeId={chequeId}
            fiscalYearId={fiscalYearId}
            action="collect"
            title={t("treasury.collect")}
            accountField={{ label: t("treasury.bankAccount"), paramName: "bankAccountId" }}
            onSuccessAction={refresh}
          />
          <TransitionForm
            chequeId={chequeId}
            fiscalYearId={fiscalYearId}
            action="bounce"
            title={t("treasury.bounceFromBank")}
            onSuccessAction={refresh}
          />
        </div>
      )}

      <div>
        <h2 className="mb-2 text-sm font-semibold text-foreground">{t("treasury.events")}</h2>
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("treasury.status")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.eventDate")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.voucher")}</th>
              </tr>
            </thead>
            <tbody>
              {cheque.events.map((e) => (
                <tr key={e.id} className="border-b border-border last:border-0">
                  <td className="px-3 py-2 text-foreground">{t(STATUS_LABEL[e.resultingStatus])}</td>
                  <td className="px-3 py-2 text-muted-foreground">{e.eventDate}</td>
                  <td className="tabular-nums px-3 py-2 text-muted-foreground">{e.voucherId ? toPersianDigits(e.voucherId) : "—"}</td>
                </tr>
              ))}
            </tbody>
          </table>
        </div>
      </div>
    </div>
  );
}
