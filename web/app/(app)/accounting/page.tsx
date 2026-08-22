import Link from "next/link";
import { t } from "@/lib/i18n/fa";
import { AccountSearch } from "./account-search";

export default function AccountingPage() {
  return (
    <div className="flex flex-col gap-8">
      <div>
        <h1 className="text-lg font-semibold text-foreground">{t("nav.accounting")}</h1>
        <Link
          href="/accounting/chart-of-accounts"
          className="mt-2 inline-block text-sm text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("accounts.title")} ←
        </Link>
      </div>
      <AccountSearch />
    </div>
  );
}
