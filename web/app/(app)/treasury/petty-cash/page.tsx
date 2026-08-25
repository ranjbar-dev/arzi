"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { ClaimRegister } from "./claim-register";

export default function PettyCashPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.pettyCashTitle")}</h1>
      <ClaimRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
