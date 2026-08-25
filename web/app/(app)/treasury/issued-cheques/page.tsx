"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { BatchRegister } from "./batch-register";

export default function IssuedChequesPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.issuedChequesTitle")}</h1>
      <BatchRegister fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
