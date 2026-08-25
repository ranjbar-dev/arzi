"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { VoucherList } from "./voucher-list";

export default function VouchersPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("vouchers.title")}</h1>
      <VoucherList fiscalYearId={session?.currentFiscalYearId ?? null} />
    </div>
  );
}
