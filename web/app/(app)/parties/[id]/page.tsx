"use client";

import { useParams } from "next/navigation";
import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { PartyCard } from "./party-card";

export default function PartyCardPage() {
  const { id } = useParams<{ id: string }>();
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("parties.balanceTitle")}</h1>
      <PartyCard partyId={Number(id)} currentFiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
