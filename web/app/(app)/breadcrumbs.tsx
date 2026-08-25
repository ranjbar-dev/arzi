"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTranslation } from "react-i18next";
import type { fa } from "@/lib/i18n/fa";

type FlatKey = { [S in keyof typeof fa]: `${S}.${Extract<keyof (typeof fa)[S], string>}` }[keyof typeof fa];

// Static route -> label map. Segments not listed here (record ids) fall back
// to the raw path segment rather than a fetched entity name (ponytail: no
// extra query just for a breadcrumb crumb).
const LABELS: Record<string, FlatKey> = {
  "/accounting": "nav.accounting",
  "/accounting/chart-of-accounts": "accounts.title",
  "/accounting/vouchers": "vouchers.title",
  "/inventory": "nav.inventory",
  "/inventory/warehouses": "inventory.warehousesTitle",
  "/inventory/units-of-measure": "inventory.unitsTitle",
  "/inventory/items": "inventory.itemsTitle",
  "/inventory/invoices": "inventory.invoicesTitle",
  "/treasury": "nav.treasury",
  "/treasury/received-cheques": "treasury.receivedChequesTitle",
  "/treasury/deposit-slips": "treasury.depositSlipsTitle",
  "/treasury/petty-cash": "treasury.pettyCashTitle",
  "/treasury/issued-cheques": "treasury.issuedChequesTitle",
  "/parties": "nav.parties",
  "/reporting": "nav.reporting",
  "/platform": "nav.platform",
  "/platform/users": "admin.users",
};

/** Breadcrumb trail for every page under the app shell except the dashboard
 * root itself (nothing to trail there). Driven purely by the URL — one
 * shared crumb bar instead of per-page markup. */
export function Breadcrumbs() {
  const pathname = usePathname();
  const { t } = useTranslation();

  if (pathname === "/") return null;

  const segments = pathname.split("/").filter(Boolean);
  const crumbs = segments.map((_, i) => {
    const href = "/" + segments.slice(0, i + 1).join("/");
    const key = LABELS[href];
    return { href, label: key ? t(key) : segments[i] };
  });

  return (
    <nav aria-label={t("nav.dashboard")} className="flex flex-wrap items-center gap-1 text-sm text-muted-foreground">
      <Link href="/" className="hover:text-accent">
        {t("nav.dashboard")}
      </Link>
      {crumbs.map((crumb, i) => {
        const isLast = i === crumbs.length - 1;
        return (
          <span key={crumb.href} className="flex items-center gap-1">
            <span>/</span>
            {isLast ? (
              <span className="text-foreground">{crumb.label}</span>
            ) : (
              <Link href={crumb.href} className="hover:text-accent">
                {crumb.label}
              </Link>
            )}
          </span>
        );
      })}
    </nav>
  );
}
