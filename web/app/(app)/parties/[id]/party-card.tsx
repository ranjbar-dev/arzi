"use client";

// Step 3.4 (docs/phase-3-parties.md §3.4): `CardJariU`'s party-linkage
// aspects only (specs/07-parties-and-shareholders/07-10.md §10.10) — the
// accounting identity + net current-account balance (3.2's API) for a
// chosen fiscal year, plus a link out. Full subsidiary-ledger rendering is
// Phase 6 (the Build bullet's own scope line) — not built here.

import { useState } from "react";
import Link from "next/link";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import type { PartyDetail, PartyBalance } from "@/lib/parties";

interface FiscalYear {
  id: number;
  year: number;
}

export function PartyCard({
  partyId,
  currentFiscalYearId,
}: {
  partyId: number;
  currentFiscalYearId: number | null;
}) {
  const { t } = useTranslation();
  const [fiscalYearId, setFiscalYearId] = useState<number | null>(currentFiscalYearId);

  const { data: party } = useQuery({
    queryKey: ["parties", partyId],
    queryFn: () => apiRequest<PartyDetail>(`/api/v1/parties/${partyId}`),
  });
  const { data: years } = useQuery({
    queryKey: ["fiscal-years"],
    queryFn: () => apiRequest<FiscalYear[]>("/api/v1/fiscal-years"),
  });
  const { data: balance } = useQuery({
    queryKey: ["parties", partyId, "balance", fiscalYearId],
    queryFn: () => apiRequest<PartyBalance>(`/api/v1/parties/${partyId}/balance?fiscalYearId=${fiscalYearId}`),
    enabled: fiscalYearId !== null,
  });

  if (!party) return <p className="text-sm text-muted-foreground">{t("common.loading")}</p>;

  const isCreditor = (balance?.total ?? 0) >= 0;

  return (
    <div className="flex flex-col gap-4">
      <div className="rounded-md border border-border bg-surface p-4 text-sm">
        <div className="flex flex-wrap gap-x-6 gap-y-1">
          <span className="text-muted-foreground">
            {t("parties.cardNumber")}: <span className="tabular-nums text-foreground">{toPersianDigits(party.cardNumber)}</span>
          </span>
          <span className="text-foreground">
            {party.firstName} {party.lastName}
          </span>
          {party.fatherName && <span className="text-muted-foreground">{party.fatherName}</span>}
          {party.mobile && <span className="text-muted-foreground">{party.mobile}</span>}
          {party.nationalId && <span className="text-muted-foreground">{party.nationalId}</span>}
          {party.isLocked && <span className="text-warning">{t("parties.lockedParty")}</span>}
        </div>
      </div>

      <div className="flex items-center gap-3">
        <label className="text-sm text-muted-foreground">{t("parties.fiscalYear")}</label>
        <select
          value={fiscalYearId ?? ""}
          onChange={(e) => setFiscalYearId(e.target.value ? Number(e.target.value) : null)}
          className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
        >
          <option value="">—</option>
          {years?.map((y) => (
            <option key={y.id} value={y.id}>
              {toPersianDigits(y.year)}
            </option>
          ))}
        </select>
      </div>

      {balance && (
        <div className="rounded-md border border-border bg-surface p-4">
          <div className="flex items-baseline gap-2">
            <span className="text-sm text-muted-foreground">{t("parties.balance")}:</span>
            <span className="text-lg font-semibold tabular-nums text-foreground">
              {toPersianDigits(Math.abs(balance.total))}
            </span>
            <span className={isCreditor ? "text-success" : "text-danger"}>
              {isCreditor ? t("parties.creditor") : t("parties.debtor")}
            </span>
          </div>

          {balance.breakdown.length > 0 && (
            <table className="mt-3 w-full text-sm">
              <thead>
                <tr className="border-b border-border text-muted-foreground">
                  <th className="px-2 py-1 text-start font-medium">{t("parties.groupName")}</th>
                  <th className="px-2 py-1 text-center font-medium">{t("vouchers.debit")}</th>
                  <th className="px-2 py-1 text-center font-medium">{t("vouchers.credit")}</th>
                </tr>
              </thead>
              <tbody>
                {balance.breakdown.map((row) => (
                  <tr key={row.configId} className="border-b border-border last:border-0">
                    <td className="px-2 py-1 text-foreground">{row.name}</td>
                    <td className="tabular-nums px-2 py-1 text-center text-foreground">
                      {toPersianDigits(row.debit)}
                    </td>
                    <td className="tabular-nums px-2 py-1 text-center text-foreground">
                      {toPersianDigits(row.credit)}
                    </td>
                  </tr>
                ))}
              </tbody>
            </table>
          )}
        </div>
      )}

      <Link href="/reporting" className="text-sm text-accent hover:underline">
        {t("parties.balanceTitle")} ›
      </Link>
    </div>
  );
}
