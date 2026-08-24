"use client";

// Step 3.4 (docs/phase-3-parties.md §3.4): `SahamdarU` equivalent
// (specs/07-parties-and-shareholders/07-10.md §10.1) — one route with a
// kind tab (persons/companies), matching the legacy's two-grid-one-
// datasource shape. Server-side filtering (`?kind=`) replaces the legacy's
// load-everything-then-filter-client-side grid.

import { useState } from "react";
import Link from "next/link";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import type { PartySummary, PartyType } from "@/lib/parties";
import { LockIcon } from "@/components/lock-icon";
import { PartyForm } from "./party-form";

export function PartyRegister({ canLock }: { canLock: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [kind, setKind] = useState<PartyType>("natural_person");
  const [dialog, setDialog] = useState<"create" | number | null>(null);

  const { data: parties, isLoading } = useQuery({
    queryKey: ["parties", "list", kind],
    queryFn: () => apiRequest<PartySummary[]>(`/api/v1/parties?kind=${kind}`),
  });

  const lockMutation = useMutation({
    mutationFn: ({ id, lock }: { id: number; lock: boolean }) =>
      apiRequest(`/api/v1/parties/${id}/${lock ? "lock" : "unlock"}`, { method: "POST" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["parties"] }),
  });

  const isPerson = kind === "natural_person";

  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("parties.title")}</h1>

      <div className="flex gap-2">
        {(["natural_person", "legal_entity"] as PartyType[]).map((k) => (
          <button
            key={k}
            type="button"
            onClick={() => setKind(k)}
            className={`h-9 cursor-pointer rounded-md px-4 text-sm font-medium transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-accent ${
              kind === k
                ? "bg-primary text-primary-foreground"
                : "border border-border bg-surface text-foreground hover:bg-muted"
            }`}
          >
            {k === "natural_person" ? t("parties.persons") : t("parties.companies")}
          </button>
        ))}
        <button
          type="button"
          onClick={() => setDialog("create")}
          className="h-9 cursor-pointer rounded-md border border-border bg-surface px-4 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("parties.newParty")}
        </button>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-center font-medium">{t("parties.cardNumber")}</th>
                <th className="px-3 py-2 text-start font-medium">
                  {isPerson ? t("parties.firstName") : t("parties.entityName")}
                </th>
                <th className="px-3 py-2 text-start font-medium">
                  {isPerson ? t("parties.lastName") : t("parties.representative")}
                </th>
                <th className="px-3 py-2 text-start font-medium">
                  {isPerson ? t("parties.nationalId") : t("parties.entityNationalId")}
                </th>
                <th className="px-3 py-2 text-start font-medium">{t("parties.mobile")}</th>
                <th className="w-16 px-3 py-2 text-center font-medium">{t("parties.lock")}</th>
              </tr>
            </thead>
            <tbody>
              {parties?.map((p) => (
                <tr key={p.id} className="border-b border-border last:border-0 hover:bg-muted">
                  <td className="tabular-nums px-3 py-2 text-center">
                    <Link href={`/parties/${p.id}`} className="text-accent hover:underline">
                      {toPersianDigits(p.cardNumber)}
                    </Link>
                  </td>
                  <td
                    className="cursor-pointer px-3 py-2 text-foreground"
                    onClick={() => setDialog(p.id)}
                  >
                    {p.firstName}
                  </td>
                  <td className="cursor-pointer px-3 py-2 text-foreground" onClick={() => setDialog(p.id)}>
                    {p.lastName}
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">{p.nationalId ?? "—"}</td>
                  <td className="px-3 py-2 text-muted-foreground">{p.mobile ?? "—"}</td>
                  <td className="px-3 py-2 text-center">
                    {canLock ? (
                      <button
                        type="button"
                        onClick={() => lockMutation.mutate({ id: p.id, lock: !p.isLocked })}
                        className="cursor-pointer focus-visible:ring-2 focus-visible:ring-accent"
                      >
                        <LockIcon locked={p.isLocked} className="mx-auto h-4 w-4 text-warning" />
                      </button>
                    ) : (
                      p.isLocked && <LockIcon locked className="mx-auto h-4 w-4 text-warning" />
                    )}
                  </td>
                </tr>
              ))}
              {parties?.length === 0 && (
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

      <PartyForm
        open={dialog !== null}
        kind={kind}
        partyId={dialog === "create" || dialog === null ? null : dialog}
        onCloseAction={() => setDialog(null)}
      />
    </div>
  );
}
