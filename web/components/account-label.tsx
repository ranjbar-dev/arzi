"use client";

// Small helper used wherever a voucher line only carries an `accountId` —
// resolves it to a display string via 2.1's GET /accounts/{id} (cached by
// TanStack Query, so repeated lines on the same account cost one request).
import { useQuery } from "@tanstack/react-query";
import { apiRequest } from "@/lib/api-client";
import type { AccountDetail } from "@/lib/accounts";

export function AccountLabel({ accountId }: { accountId: number }) {
  const { data } = useQuery({
    queryKey: ["accounts", accountId],
    queryFn: () => apiRequest<AccountDetail>(`/api/v1/accounts/${accountId}`),
  });
  if (!data) return <span className="text-muted-foreground">…</span>;
  return (
    <span>
      {data.name} <span className="text-xs text-muted-foreground">({data.codeLtr})</span>
    </span>
  );
}
