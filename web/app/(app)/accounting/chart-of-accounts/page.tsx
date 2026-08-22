import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { ChartOfAccountsEditor } from "./editor";

export default async function ChartOfAccountsPage() {
  const session = await getSession();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("accounts.title")}</h1>
      <ChartOfAccountsEditor canLock={!!session?.isSuperuser} />
    </div>
  );
}
