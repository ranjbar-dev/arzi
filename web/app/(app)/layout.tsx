"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useTranslation } from "react-i18next";
import { useSession } from "@/lib/use-session";
import { toPersianDigits } from "@/lib/format";
import { NavLinks } from "./nav-links";
import { LogoutButton } from "./logout-button";
import { Breadcrumbs } from "./breadcrumbs";

/** The step 1.6 "protected route wrapper", client-side: a 401 from
 * `/api/v1/me` bounces to `/login`. UX only — every `/api/v1/*` route
 * enforces the real check server-side regardless of whether this redirect
 * fires (docs/00-overview.md). Also the "session-aware layout" bullet:
 * tenant/fiscal-year/user come from the same call. */
export default function AppLayout({ children }: LayoutProps<"/">) {
  const router = useRouter();
  const { t } = useTranslation();
  const { data: session, isError, isLoading } = useSession();

  useEffect(() => {
    if (isError) router.replace("/login");
  }, [isError, router]);

  if (isLoading || isError || !session) {
    return null;
  }

  return (
    <div className="flex min-h-full flex-col bg-background">
      <header className="border-b border-border bg-surface">
        <div className="mx-auto flex max-w-6xl flex-wrap items-center justify-between gap-3 px-4 py-3">
          <div className="flex items-center gap-4">
            <span className="text-base font-semibold text-foreground">
              {t("common.appName")}
            </span>
            <span className="text-sm text-muted-foreground">
              {t("shell.tenant")}: {session.tenantName}
            </span>
            <span className="text-sm text-muted-foreground">
              {t("shell.fiscalYear")}:{" "}
              {session.currentFiscalYear
                ? toPersianDigits(session.currentFiscalYear)
                : t("shell.noFiscalYear")}
            </span>
          </div>
          <div className="flex items-center gap-3">
            <span className="text-sm text-foreground">
              {session.username}
              {session.isSuperuser && (
                <span className="ms-1 text-xs text-accent">({t("shell.superuser")})</span>
              )}
            </span>
            <LogoutButton />
          </div>
        </div>
        <div className="mx-auto max-w-6xl px-4 pb-2">
          <NavLinks />
        </div>
      </header>
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6">
        <Breadcrumbs />
        {children}
      </main>
    </div>
  );
}
