"use client";

import { useQuery } from "@tanstack/react-query";
import { apiRequest } from "./api-client";
import type { Session } from "./session";

/** Client-side counterpart to `getSession()` — same `/api/v1/me` call, from
 * the browser instead of the server. Pages read `session?.field ?? default`
 * exactly as before; a missing/expired session just leaves `data` undefined. */
export function useSession() {
  return useQuery({
    queryKey: ["session"],
    queryFn: () => apiRequest<Session>("/api/v1/me"),
    retry: false,
  });
}
