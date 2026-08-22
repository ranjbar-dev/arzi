import { cookies } from "next/headers";

// Server-only: Server Components/Actions call the Rust API directly over
// the internal docker network (docs/00-overview.md's "server-rendered pages
// used only where they help (login, print views)" — this is that path).
// Client-rendered screens instead call same-origin `/api/v1/...`, which
// `next.config.ts`'s rewrite proxies to the same backend — see
// `lib/api-client.ts`.
const API_INTERNAL_URL = process.env.API_INTERNAL_URL ?? "http://localhost:8080";

export const SESSION_COOKIE = "arzi_session";

/** Forwards the current request's session cookie to the Rust API. Works
 * with no session too (login has none yet) — just omits the header. */
export async function apiFetch(path: string, init: RequestInit = {}): Promise<Response> {
  const jar = await cookies();
  const token = jar.get(SESSION_COOKIE)?.value;
  const headers = new Headers(init.headers);
  if (token) headers.set("Cookie", `${SESSION_COOKIE}=${token}`);
  return fetch(`${API_INTERNAL_URL}${path}`, { ...init, headers, cache: "no-store" });
}

/** Login has no cookie yet, so it calls the API directly rather than through
 * `apiFetch`, and needs the *raw* `Set-Cookie` value back to re-issue on the
 * Next.js origin (see `app/login/actions.ts`) — `fetch()` never exposes that
 * header's value to server code the way a browser would apply it, so this
 * parses it out by hand instead of pulling in a cookie-parsing dependency
 * for one field.
 */
export function extractSessionToken(response: Response): string | null {
  const setCookie = response.headers.get("set-cookie");
  if (!setCookie) return null;
  const match = setCookie.match(new RegExp(`${SESSION_COOKIE}=([^;]+)`));
  return match?.[1] ?? null;
}

export function apiInternalUrl(path: string): string {
  return `${API_INTERNAL_URL}${path}`;
}
