"use client";

import { useTranslation } from "react-i18next";
import { logoutAction } from "../login/actions";

export function LogoutButton() {
  const { t } = useTranslation();
  return (
    <form action={logoutAction}>
      <button
        type="submit"
        className="cursor-pointer rounded-md px-3 py-1.5 text-sm text-muted-foreground transition-colors duration-150 hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-accent"
      >
        {t("auth.logout")}
      </button>
    </form>
  );
}
