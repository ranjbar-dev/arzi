import { redirect } from "next/navigation";
import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { toPersianDigits } from "@/lib/format";
import { NavLinks } from "./nav-links";
import { LogoutButton } from "./logout-button";

/** The step 1.6 "protected route wrapper": every route under this group
 * requires a valid session, checked server-side on every request (no
 * client-side-only guard) — a 401 from `/api/v1/me` redirects straight to
 * `/login`, satisfying the manual test's "unauthenticated → redirected to
 * login". Also the "session-aware layout" bullet: tenant/fiscal-year/user
 * come from the same call. */
export default async function AppLayout({ children }: LayoutProps<"/">) {
  const session = await getSession();
  if (!session) {
    redirect("/login");
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
      <main className="mx-auto w-full max-w-6xl flex-1 px-4 py-6">{children}</main>
    </div>
  );
}
