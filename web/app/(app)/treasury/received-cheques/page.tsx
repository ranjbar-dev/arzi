import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { ChequeRegister } from "./cheque-register";

export default async function ReceivedChequesPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.receivedChequesTitle")}</h1>
      <ChequeRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
