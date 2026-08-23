"use client";

// Step 5.9: the item search picker, replacing `AnbarCalaSelectU` (specs/05-inventory §2.6) — a
// server-side search (no 18-character truncation, no PATINDEX wildcard-injection trap) hitting
// items.rs's own `?search=` query param, debounced client-side rather than firing on every
// keystroke unthrottled.

import { useEffect, useState } from "react";
import { useQuery } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import type { Item } from "@/lib/inventory";

export function ItemPicker({
  onSelectAction,
  triggerLabel,
  pistachioOnly = false,
}: {
  onSelectAction: (item: Item) => void;
  triggerLabel: string;
  pistachioOnly?: boolean;
}) {
  const { t } = useTranslation();
  const [open, setOpen] = useState(false);
  const [term, setTerm] = useState("");
  const [debounced, setDebounced] = useState("");

  useEffect(() => {
    const id = setTimeout(() => setDebounced(term), 250);
    return () => clearTimeout(id);
  }, [term]);

  const { data: items, isLoading } = useQuery({
    queryKey: ["items", "search", debounced],
    queryFn: () => apiRequest<Item[]>(`/api/v1/items?activeOnly=true${debounced ? `&search=${encodeURIComponent(debounced)}` : ""}`),
    enabled: open,
  });
  const visible = pistachioOnly ? items?.filter((i) => i.pistachioGradeId !== null) : items;

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
        <div className="absolute z-10 mt-1 w-80 rounded-md border border-border bg-surface p-2 shadow-lg">
          <input
            autoFocus
            type="text"
            value={term}
            onChange={(e) => setTerm(e.target.value)}
            placeholder={t("inventory.itemName")}
            className="mb-2 h-9 w-full rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
          <div className="max-h-64 overflow-y-auto">
            {isLoading && <p className="p-2 text-sm text-muted-foreground">{t("common.loading")}</p>}
            {visible?.map((item) => (
              <button
                key={item.id}
                type="button"
                onClick={() => {
                  onSelectAction(item);
                  setOpen(false);
                  setTerm("");
                }}
                className="flex w-full cursor-pointer items-center justify-between rounded-md px-2 py-1.5 text-start text-sm hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
              >
                <span>{item.name}</span>
                <span className="tabular-nums text-muted-foreground">{toPersianDigits(item.code)}</span>
              </button>
            ))}
            {visible?.length === 0 && <p className="p-2 text-sm text-muted-foreground">—</p>}
          </div>
        </div>
      )}
    </div>
  );
}
