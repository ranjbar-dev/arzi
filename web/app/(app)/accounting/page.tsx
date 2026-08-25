"use client";

import Link from "next/link";
import { useTranslation } from "react-i18next";
import { AccountsTreeIcon } from "@/components/accounts-tree-icon";
import { VoucherIcon } from "@/components/voucher-icon";

function AccountingCard({
  href,
  title,
  subtitle,
  icon,
}: {
  href: string;
  title: string;
  subtitle: string;
  icon: React.ReactNode;
}) {
  return (
    <Link
      href={href}
      className="group flex items-start gap-4 rounded-lg border border-border bg-surface p-5 transition-colors duration-150 hover:border-accent focus-visible:ring-2 focus-visible:ring-accent"
    >
      <div className="flex h-11 w-11 shrink-0 items-center justify-center rounded-md bg-accent/10 text-accent">
        {icon}
      </div>
      <div className="flex flex-col gap-1">
        <span className="text-sm font-semibold text-foreground group-hover:text-accent">{title}</span>
        <span className="text-xs text-muted-foreground">{subtitle}</span>
      </div>
    </Link>
  );
}

export default function AccountingPage() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h1 className="text-lg font-semibold text-foreground">{t("nav.accounting")}</h1>
      </div>
      <div className="grid grid-cols-1 gap-4 sm:grid-cols-2">
        <AccountingCard
          href="/accounting/chart-of-accounts"
          title={t("accounts.title")}
          subtitle={t("accounts.subtitle")}
          icon={<AccountsTreeIcon />}
        />
        <AccountingCard
          href="/accounting/vouchers"
          title={t("vouchers.title")}
          subtitle={t("vouchers.subtitle")}
          icon={<VoucherIcon />}
        />
      </div>
    </div>
  );
}
