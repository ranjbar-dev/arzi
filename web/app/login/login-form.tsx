"use client";

import { useActionState } from "react";
import { useTranslation } from "react-i18next";
import { loginAction } from "./actions";

export function LoginForm() {
  const { t } = useTranslation();
  const [state, action, pending] = useActionState(loginAction, {});

  return (
    <form action={action} className="flex flex-col gap-4">
      <div className="flex flex-col gap-1.5">
        <label htmlFor="tenantSlug" className="text-sm font-medium text-foreground">
          {t("auth.tenantSlug")}
        </label>
        <input
          id="tenantSlug"
          name="tenantSlug"
          type="text"
          required
          autoComplete="organization"
          placeholder="arzi-co"
          className="h-10 rounded-md border border-border bg-surface px-3 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-accent"
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <label htmlFor="username" className="text-sm font-medium text-foreground">
          {t("auth.username")}
        </label>
        <input
          id="username"
          name="username"
          type="text"
          required
          autoComplete="username"
          className="h-10 rounded-md border border-border bg-surface px-3 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-accent"
        />
      </div>

      <div className="flex flex-col gap-1.5">
        <label htmlFor="password" className="text-sm font-medium text-foreground">
          {t("auth.password")}
        </label>
        <input
          id="password"
          name="password"
          type="password"
          required
          autoComplete="current-password"
          className="h-10 rounded-md border border-border bg-surface px-3 text-sm text-foreground outline-none focus-visible:ring-2 focus-visible:ring-accent"
        />
      </div>

      {state.error && (
        <p role="alert" className="text-sm text-danger">
          {t(`auth.${state.error}`)}
        </p>
      )}

      <button
        type="submit"
        disabled={pending}
        className="mt-2 h-10 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition-colors duration-150 hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent focus-visible:ring-offset-2 disabled:cursor-not-allowed disabled:opacity-60"
      >
        {pending ? t("auth.loggingIn") : t("auth.login")}
      </button>
    </form>
  );
}
