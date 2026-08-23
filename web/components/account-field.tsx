"use client";

// A labelled leaf-account picker field — used across the treasury forms
// (step 4.5) wherever a screen needs one account id plus a visible name,
// combining the reused `AccountPicker` (C5) with local selection state.

import { useState } from "react";
import { useTranslation } from "react-i18next";
import { AccountPicker } from "./account-picker";
import { AccountLabel } from "./account-label";
import type { AccountSummary } from "@/lib/accounts";

export function AccountField({
  label,
  value,
  onChangeAction,
}: {
  label?: string;
  value: number | null;
  onChangeAction: (id: number) => void;
}) {
  const { t } = useTranslation();
  const [selected, setSelected] = useState<AccountSummary | null>(null);

  return (
    <div className="flex flex-col gap-1">
      {label && <label className="text-sm text-muted-foreground">{label}</label>}
      <div className="flex items-center gap-2">
        <AccountPicker
          triggerLabel={selected ? selected.name : t("accounts.selectAccount")}
          onSelect={(a) => {
            setSelected(a);
            onChangeAction(a.id);
          }}
        />
        {value !== null && !selected && <AccountLabel accountId={value} />}
      </div>
    </div>
  );
}
