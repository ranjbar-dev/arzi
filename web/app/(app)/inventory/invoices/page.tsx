import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { InvoiceList } from "./invoice-list";

export default async function InvoicesPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.invoicesTitle")}</h1>
      <InvoiceList fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
