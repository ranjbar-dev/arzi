"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { ChequeRegister } from "./cheque-register";

export default function ReceivedChequesPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.receivedChequesTitle")}</h1>
      <ChequeRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
