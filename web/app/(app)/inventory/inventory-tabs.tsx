"use client";

// Step 5.9: warehouses/units/items/invoices as distinct routes/lists, same tab pattern already
// established by treasury-tabs.tsx (4.5) and party-register's own kind tabs (3.4).

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTranslation } from "react-i18next";

const TABS = [
  { href: "/inventory/warehouses", key: "inventory.warehousesTitle" },
  { href: "/inventory/units-of-measure", key: "inventory.unitsTitle" },
  { href: "/inventory/items", key: "inventory.itemsTitle" },
  { href: "/inventory/invoices", key: "inventory.invoicesTitle" },
] as const;

export function InventoryTabs() {
  const { t } = useTranslation();
  const pathname = usePathname();

  return (
    <nav className="flex gap-1 border-b border-border">
      {TABS.map((tab) => {
        const active = pathname?.startsWith(tab.href);
        return (
          <Link
            key={tab.href}
            href={tab.href}
            className={`-mb-px rounded-t-md border-b-2 px-3 py-2 text-sm transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-accent ${
              active ? "border-accent text-accent" : "border-transparent text-muted-foreground hover:text-foreground"
            }`}
          >
            {t(tab.key)}
          </Link>
        );
      })}
    </nav>
  );
}
