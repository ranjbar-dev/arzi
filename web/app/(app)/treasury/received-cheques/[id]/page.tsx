"use client";

import { useParams } from "next/navigation";
import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { ChequeDetail } from "./cheque-detail";

export default function ChequeDetailPage() {
  const { id } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.receivedChequesTitle")}</h1>
      <ChequeDetail chequeId={Number(id)} fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
