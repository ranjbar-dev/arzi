import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { PartyCard } from "./party-card";

export default async function PartyCardPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("parties.balanceTitle")}</h1>
      <PartyCard partyId={Number(id)} currentFiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
