import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { ChequeDetail } from "./cheque-detail";

export default async function ChequeDetailPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("treasury.receivedChequesTitle")}</h1>
      <ChequeDetail chequeId={Number(id)} fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
