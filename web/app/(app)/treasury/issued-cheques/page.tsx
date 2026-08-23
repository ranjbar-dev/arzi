import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { BatchRegister } from "./batch-register";

export default async function IssuedChequesPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.issuedChequesTitle")}</h1>
      <BatchRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
