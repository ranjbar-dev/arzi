import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";

export default async function DashboardPage() {
  const session = await getSession();
  return (
    <div>
      <h1 className="text-lg font-semibold text-foreground">
        {t("nav.dashboard")}
      </h1>
      <p className="mt-2 text-sm text-muted-foreground">
        {session?.username}, {t("common.appName")}
      </p>
    </div>
  );
}
