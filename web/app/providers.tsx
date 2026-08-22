"use client";

import { useState } from "react";
import { QueryClient, QueryClientProvider } from "@tanstack/react-query";
import { I18nextProvider } from "react-i18next";
import i18n from "@/lib/i18n/client";

/** One client-only boundary wrapping the whole app: TanStack Query (the
 * locked-in data-fetching layer for the "data-dense client-rendered
 * screens" per docs/00-overview.md) and react-i18next. Server Components
 * (login page, the shell layout's session check) never need this — they use
 * `t()` from `lib/i18n/fa.ts` directly. */
export function Providers({ children }: { children: React.ReactNode }) {
  const [queryClient] = useState(() => new QueryClient());
  return (
    <QueryClientProvider client={queryClient}>
      <I18nextProvider i18n={i18n}>{children}</I18nextProvider>
    </QueryClientProvider>
  );
}
