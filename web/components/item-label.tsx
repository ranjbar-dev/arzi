"use client";

// Resolves an inventory document line's `itemId` to a display name — same pattern as
// account-label.tsx (2.4), one GET per distinct item, cached by TanStack Query.
import { useQuery } from "@tanstack/react-query";
import { apiRequest } from "@/lib/api-client";
import type { Item } from "@/lib/inventory";

export function ItemLabel({ itemId }: { itemId: number }) {
  const { data } = useQuery({
    queryKey: ["items", itemId],
    queryFn: () => apiRequest<Item>(`/api/v1/items/${itemId}`),
  });
  if (!data) return <span className="text-muted-foreground">…</span>;
  return <span>{data.name}</span>;
}
