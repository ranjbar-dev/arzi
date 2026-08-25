"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { InvoiceList } from "./invoice-list";

export default function InvoicesPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.invoicesTitle")}</h1>
      <InvoiceList fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
