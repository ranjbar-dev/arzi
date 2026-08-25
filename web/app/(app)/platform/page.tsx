"use client";

import Link from "next/link";
import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { FiscalYearsPanel } from "./fiscal-years-panel";

/** Manual test item 4 (docs/phase-1-platform-and-auth.md §1.6): "as a non-
 * superuser without the admin permission, confirm the admin nav item is
 * absent or disabled" — the `admin.users` link only renders for a session's
 * `isSuperuser`. This is UX only; `/api/v1/admin/*` itself is what actually
 * enforces it (1.3's `RequireSuperuser`) even if someone hits the URL
 * directly, same as every other route in this app. */
export default function PlatformPage() {
  const { t } = useTranslation();
  const { data: session } = useSession();

  return (
    <div className="flex flex-col gap-8">
      <div>
        <h1 className="text-lg font-semibold text-foreground">{t("nav.platform")}</h1>
        {session?.isSuperuser && (
          <Link
            href="/platform/users"
            className="mt-2 inline-block text-sm text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("admin.users")} ←
          </Link>
        )}
      </div>
      <FiscalYearsPanel canManage={!!session?.isSuperuser} />
    </div>
  );
}
