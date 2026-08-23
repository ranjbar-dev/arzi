import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { ClaimRegister } from "./claim-register";

export default async function PettyCashPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.pettyCashTitle")}</h1>
      <ClaimRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
