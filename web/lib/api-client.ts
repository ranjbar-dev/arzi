"use client";

// Browser-side fetch helper for TanStack Query hooks — always a relative
// path, proxied same-origin by next.config.ts's rewrite (see lib/api-server.ts
// for why: no CORS, cookie just works).
export class ApiError extends Error {
  constructor(
    public status: number,
    public body: unknown,
  ) {
    super(typeof body === "object" && body && "error" in body ? String((body as { error: unknown }).error) : "api_error");
  }
}

export async function apiRequest<T>(path: string, init: RequestInit = {}): Promise<T> {
  const res = await fetch(path, {
    ...init,
    headers: { "Content-Type": "application/json", ...init.headers },
  });
  const isJson = res.headers.get("content-type")?.includes("application/json");
  const body = isJson ? await res.json() : undefined;
  if (!res.ok) {
    throw new ApiError(res.status, body);
  }
  return body as T;
}
