"use client";

// Step 4.5: the three treasury registers are kept as distinct routes/lists
// (received cheques, deposit slips, petty cash) rather than one combined
// screen — matches the legacy's own correct instinct to keep received and
// issued cheques as separate lists (06-03-received-versus-issued-cheques.md).

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTranslation } from "react-i18next";

const TABS = [
  { href: "/treasury/received-cheques", key: "treasury.receivedChequesTitle" },
  { href: "/treasury/issued-cheques", key: "treasury.issuedChequesTitle" },
  { href: "/treasury/deposit-slips", key: "treasury.depositSlipsTitle" },
  { href: "/treasury/petty-cash", key: "treasury.pettyCashTitle" },
] as const;

export function TreasuryTabs() {
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
