import { t } from "@/lib/i18n/fa";

/** Every domain's screens land in their own phase (2-6) — this is just the
 * nav/routing skeleton the Build bullet asks for. */
export function DomainPlaceholder({ titleKey }: { titleKey: string }) {
  return (
    <div>
      <h1 className="text-lg font-semibold text-foreground">{t(titleKey)}</h1>
      <p className="mt-2 text-sm text-muted-foreground">{t("common.comingSoon")}</p>
    </div>
  );
}
