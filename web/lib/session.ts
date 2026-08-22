import { apiFetch } from "./api-server";

export interface Session {
  userId: number;
  tenantId: number;
  username: string;
  tenantName: string;
  isSuperuser: boolean;
  permissions: string[];
  currentFiscalYearId: number | null;
  currentFiscalYear: number | null;
}

/** The step 1.6 "protected route wrapper" — `null` on a missing/expired/
 * revoked session (the API's 401), never throws. Callers `redirect('/login')`
 * on `null` (see `app/(app)/layout.tsx`). */
export async function getSession(): Promise<Session | null> {
  const res = await apiFetch("/api/v1/me");
  if (!res.ok) return null;
  return res.json();
}
