"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { ChartOfAccountsEditor } from "./editor";

export default function ChartOfAccountsPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
  return (
    <div className="flex flex-col gap-4">
      <div>
        <h1 className="text-lg font-semibold text-foreground">{t("accounts.title")}</h1>
        <p className="text-sm text-muted-foreground">{t("accounts.subtitle")}</p>
      </div>
      <ChartOfAccountsEditor canLock={!!session?.isSuperuser} />
    </div>
  );
}
