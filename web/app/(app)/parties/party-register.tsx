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
import { DataTable, FilterInput, useDebounced, useSort } from "@/components/data-table";
import { PartyForm } from "./party-form";

export function PartyRegister({ canLock }: { canLock: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [kind, setKind] = useState<PartyType>("natural_person");
  const [dialog, setDialog] = useState<"create" | number | null>(null);
  const [cardNumberFilter, setCardNumberFilter] = useState("");
  const [firstNameFilter, setFirstNameFilter] = useState("");
  const [lastNameFilter, setLastNameFilter] = useState("");
  const [nationalIdFilter, setNationalIdFilter] = useState("");
  const [mobileFilter, setMobileFilter] = useState("");
  const debouncedCardNumber = useDebounced(cardNumberFilter);
  const debouncedFirstName = useDebounced(firstNameFilter);
  const debouncedLastName = useDebounced(lastNameFilter);
  const debouncedNationalId = useDebounced(nationalIdFilter);
  const debouncedMobile = useDebounced(mobileFilter);
  const { sort, toggleSort } = useSort();

  const params = new URLSearchParams({ kind });
  if (debouncedCardNumber) params.set("cardNumber", debouncedCardNumber);
  if (debouncedFirstName) params.set("firstName", debouncedFirstName);
  if (debouncedLastName) params.set("lastName", debouncedLastName);
  if (debouncedNationalId) params.set("nationalId", debouncedNationalId);
  if (debouncedMobile) params.set("mobile", debouncedMobile);
  if (sort) {
    params.set("sort", sort.field);
    params.set("order", sort.dir);
  }

  const { data: parties, isLoading } = useQuery({
    queryKey: ["parties", "list", kind, debouncedCardNumber, debouncedFirstName, debouncedLastName, debouncedNationalId, debouncedMobile, sort],
    queryFn: () => apiRequest<PartySummary[]>(`/api/v1/parties?${params.toString()}`),
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

      <DataTable<PartySummary>
        columns={[
          {
            key: "cardNumber",
            header: t("parties.cardNumber"),
            sortable: true,
            thClassName: "text-center",
            tdClassName: "tabular-nums text-center",
            filter: <FilterInput value={cardNumberFilter} onChangeAction={setCardNumberFilter} />,
            render: (p) => (
              <Link href={`/parties/${p.id}`} className="text-accent hover:underline">
                {toPersianDigits(p.cardNumber)}
              </Link>
            ),
          },
          {
            key: "firstName",
            header: isPerson ? t("parties.firstName") : t("parties.entityName"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={firstNameFilter} onChangeAction={setFirstNameFilter} />,
            render: (p) => (
              <span className="block cursor-pointer" onClick={() => setDialog(p.id)}>
                {p.firstName}
              </span>
            ),
          },
          {
            key: "lastName",
            header: isPerson ? t("parties.lastName") : t("parties.representative"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={lastNameFilter} onChangeAction={setLastNameFilter} />,
            render: (p) => (
              <span className="block cursor-pointer" onClick={() => setDialog(p.id)}>
                {p.lastName}
              </span>
            ),
          },
          {
            key: "nationalId",
            header: isPerson ? t("parties.nationalId") : t("parties.entityNationalId"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            filter: <FilterInput value={nationalIdFilter} onChangeAction={setNationalIdFilter} />,
            render: (p) => p.nationalId ?? "—",
          },
          {
            key: "mobile",
            header: t("parties.mobile"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            filter: <FilterInput value={mobileFilter} onChangeAction={setMobileFilter} />,
            render: (p) => p.mobile ?? "—",
          },
          {
            key: "lock",
            header: t("parties.lock"),
            thClassName: "w-16 text-center",
            tdClassName: "text-center",
            render: (p) =>
              canLock ? (
                <button
                  type="button"
                  onClick={() => lockMutation.mutate({ id: p.id, lock: !p.isLocked })}
                  className="cursor-pointer focus-visible:ring-2 focus-visible:ring-accent"
                >
                  <LockIcon locked={p.isLocked} className="mx-auto h-4 w-4 text-warning" />
                </button>
              ) : (
                p.isLocked && <LockIcon locked className="mx-auto h-4 w-4 text-warning" />
              ),
          },
        ]}
        rows={parties}
        isLoading={isLoading}
        rowKeyAction={(p) => p.id}
        sort={sort}
        onSortAction={toggleSort}
      />

      <PartyForm
        open={dialog !== null}
        kind={kind}
        partyId={dialog === "create" || dialog === null ? null : dialog}
        onCloseAction={() => setDialog(null)}
      />
    </div>
  );
}
