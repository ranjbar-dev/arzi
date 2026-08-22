"use client";

// The ONE reusable account-picker component (C5, specs/11-open-decisions.md
// Group C — merges the legacy's two competing pickers `Sarfasl_SelectU` and
// `SelectSarfasl` into a single implementation, used from every "pick an
// account" context from here on rather than reimplemented per screen).

import { useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { ownCode, type AccountSummary } from "@/lib/accounts";

interface AccountPickerProps {
  onSelect: (account: AccountSummary) => void;
  /** Which rows are pickable at all — clicking a non-selectable row with
   * children drills into it instead. Defaults to leaf-only (the legacy
   * pickers' `S_Child = 0` filter, 03-01-b.md §1.6). */
  isSelectable?: (account: AccountSummary) => boolean;
  triggerLabel: string;
}

const defaultSelectable = (a: AccountSummary) => a.childCount === 0;

export function AccountPicker({ onSelect, isSelectable = defaultSelectable, triggerLabel }: AccountPickerProps) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [stack, setStack] = useState<AccountSummary[]>([]); // breadcrumb, root..current parent

  const parentId = stack.length ? stack[stack.length - 1].id : null;
  const { data: children, isLoading } = useQuery({
    queryKey: ["accounts", "children", parentId],
    queryFn: () =>
      apiRequest<AccountSummary[]>(`/api/v1/accounts${parentId ? `?parent=${parentId}` : ""}`),
    enabled: open,
  });

  function handleRowClick(account: AccountSummary) {
    if (isSelectable(account)) {
      onSelect(account);
      setOpen(false);
      setStack([]);
      return;
    }
    if (account.childCount > 0 || account.level < 4) {
      setStack([...stack, account]);
    }
  }

  return (
    <div className="relative inline-block">
      <button
        type="button"
        onClick={() => setOpen((o) => !o)}
        className="h-9 cursor-pointer rounded-md border border-border bg-surface px-3 text-sm text-foreground transition-colors duration-150 hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
      >
        {triggerLabel}
      </button>

      {open && (
        <div className="absolute z-20 mt-1 w-80 rounded-md border border-border bg-surface p-2 shadow-lg">
          <div className="mb-2 flex flex-wrap items-center gap-1 text-xs text-muted-foreground">
            <button
              type="button"
              onClick={() => setStack([])}
              className="cursor-pointer hover:text-accent focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("accounts.root")}
            </button>
            {stack.map((node, i) => (
              <span key={node.id} className="flex items-center gap-1">
                <span>/</span>
                <button
                  type="button"
                  onClick={() => setStack(stack.slice(0, i + 1))}
                  className="cursor-pointer hover:text-accent focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {toPersianDigits(ownCode(node))} {node.name}
                </button>
              </span>
            ))}
          </div>

          {isLoading ? (
            <p className="p-2 text-sm text-muted-foreground">{t("common.loading")}</p>
          ) : (
            <ul className="max-h-64 overflow-y-auto">
              {children?.map((account) => (
                <li key={account.id}>
                  <button
                    type="button"
                    onClick={() => handleRowClick(account)}
                    className={`flex w-full cursor-pointer items-center justify-between rounded px-2 py-1.5 text-start text-sm hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent ${
                      isSelectable(account) ? "text-foreground" : "text-muted-foreground"
                    }`}
                  >
                    <span>
                      {toPersianDigits(ownCode(account))} {account.name}
                    </span>
                    {account.childCount > 0 && <span className="text-xs">›</span>}
                  </button>
                </li>
              ))}
              {children?.length === 0 && (
                <li className="p-2 text-sm text-muted-foreground">—</li>
              )}
            </ul>
          )}
        </div>
      )}
    </div>
  );
}
