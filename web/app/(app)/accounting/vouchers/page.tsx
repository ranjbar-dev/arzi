import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { VoucherList } from "./voucher-list";

export default async function VouchersPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("vouchers.title")}</h1>
      <VoucherList fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
