"use server";

import { cookies } from "next/headers";
import { redirect } from "next/navigation";
import { apiInternalUrl, extractSessionToken, SESSION_COOKIE } from "@/lib/api-server";

const SESSION_TTL_HOURS = 12; // must match api/src/auth/mod.rs's SESSION_TTL_HOURS

/** No session exists yet at login, so this is the one place that calls the
 * API directly instead of through `apiFetch` — and the one place that turns
 * the API's `Set-Cookie` into Next's own cookie (docs/00-overview.md:
 * "Server-rendered pages are used only where they help (login, print
 * views)"). Every later request — server or, via the rewrite, client — reads
 * that same cookie back. */
export async function loginAction(
  _prevState: { error?: string },
  formData: FormData,
): Promise<{ error?: string }> {
  const tenantSlug = String(formData.get("tenantSlug") ?? "");
  const username = String(formData.get("username") ?? "");
  const password = String(formData.get("password") ?? "");

  const res = await fetch(apiInternalUrl("/api/v1/auth/login"), {
    method: "POST",
    headers: { "Content-Type": "application/json" },
    body: JSON.stringify({ tenantSlug, username, password }),
    cache: "no-store",
  });

  if (res.status === 429) {
    return { error: "tooManyAttempts" };
  }
  if (!res.ok) {
    return { error: "invalidCredentials" };
  }

  const token = extractSessionToken(res);
  if (!token) {
    return { error: "invalidCredentials" };
  }

  const jar = await cookies();
  jar.set(SESSION_COOKIE, token, {
    httpOnly: true,
    secure: process.env.SESSION_COOKIE_SECURE !== "false",
    sameSite: "lax",
    path: "/",
    maxAge: SESSION_TTL_HOURS * 3600,
  });

  redirect("/");
}

export async function logoutAction(): Promise<void> {
  const jar = await cookies();
  const token = jar.get(SESSION_COOKIE)?.value;
  if (token) {
    await fetch(apiInternalUrl("/api/v1/auth/logout"), {
      method: "POST",
      headers: { Cookie: `${SESSION_COOKIE}=${token}` },
      cache: "no-store",
    });
  }
  jar.delete(SESSION_COOKIE);
  redirect("/login");
}
