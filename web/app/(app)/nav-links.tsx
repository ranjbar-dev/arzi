"use client";

import Link from "next/link";
import { usePathname } from "next/navigation";
import { useTranslation } from "react-i18next";

const LINKS = [
  { href: "/", key: "dashboard" },
  { href: "/accounting", key: "accounting" },
  { href: "/inventory", key: "inventory" },
  { href: "/treasury", key: "treasury" },
  { href: "/parties", key: "parties" },
  { href: "/reporting", key: "reporting" },
  { href: "/platform", key: "platform" },
] as const;

/** The step 1.6 nav shell — six domains from specs/01-glossary.md §§1-4 plus
 * platform/settings. Plain `<Link>`s: focusable and activatable via keyboard
 * by default, no custom JS needed for that part of the manual test. */
export function NavLinks() {
  const pathname = usePathname();
  const { t } = useTranslation();

  return (
    <nav aria-label={t("nav.dashboard")} className="flex flex-wrap gap-1">
      {LINKS.map(({ href, key }) => {
        const active = href === "/" ? pathname === "/" : pathname.startsWith(href);
        return (
          <Link
            key={href}
            href={href}
            aria-current={active ? "page" : undefined}
            className={`rounded-md px-3 py-2 text-sm font-medium transition-colors duration-150 outline-none focus-visible:ring-2 focus-visible:ring-accent ${
              active
                ? "bg-primary text-primary-foreground"
                : "text-muted-foreground hover:bg-muted hover:text-foreground"
            }`}
          >
            {t(`nav.${key}`)}
          </Link>
        );
      })}
    </nav>
  );
}
