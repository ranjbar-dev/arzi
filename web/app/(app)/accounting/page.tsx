import Link from "next/link";
import { t } from "@/lib/i18n/fa";
import { AccountSearch } from "./account-search";

export default function AccountingPage() {
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h1 className="text-lg font-semibold text-foreground">{t("nav.accounting")}</h1>
        <div className="mt-2 flex flex-col items-start gap-1">
          <Link
            href="/accounting/chart-of-accounts"
            className="text-sm text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("accounts.title")} ←
          </Link>
          <Link
            href="/accounting/vouchers"
            className="text-sm text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("vouchers.title")} ←
          </Link>
        </div>
      </div>
      <AccountSearch />
    </div>
  );
}
