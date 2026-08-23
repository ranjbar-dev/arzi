import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { SlipRegister } from "./slip-register";

export default async function DepositSlipsPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.depositSlipsTitle")}</h1>
      <SlipRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
