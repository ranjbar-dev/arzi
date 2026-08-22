"use client";

// Second usage context for the ONE account picker (C5 — see
// components/account-picker.tsx's doc comment). Manual test #4 (docs/
// phase-2-accounting-core.md §2.2): "use the account picker from two
// different contexts ... confirm it's the same component, not two" — this
// is the standalone "account search" context; the chart-of-accounts
// editor's demote action is the other.

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest } from "@/lib/api-client";
import { AccountPicker } from "@/components/account-picker";
import type { AccountDetail, AccountSummary } from "@/lib/accounts";

export function AccountSearch() {
  const { t } = useTranslation();
  const [pickedId, setPickedId] = useState<number | null>(null);

  const { data: detail } = useQuery({
    queryKey: ["accounts", pickedId],
    queryFn: () => apiRequest<AccountDetail>(`/api/v1/accounts/${pickedId}`),
    enabled: pickedId !== null,
  });

  function handleSelect(account: AccountSummary) {
    setPickedId(account.id);
  }

  return (
    <div className="flex flex-col gap-3">
      <h2 className="text-base font-medium text-foreground">{t("accounts.search")}</h2>
      <AccountPicker triggerLabel={t("accounts.selectAccount")} onSelect={handleSelect} />
      {detail && (
        <div className="rounded-md border border-border bg-surface p-3 text-sm text-muted-foreground">
          <p className="text-foreground">{detail.fullNamePath}</p>
          <p>
            {detail.codeLtr} · {detail.codeRtl}
          </p>
        </div>
      )}
    </div>
  );
}
