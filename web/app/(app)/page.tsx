"use client";

import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";

export default function DashboardPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();
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
