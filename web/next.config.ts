import type { NextConfig } from "next";

// Client-rendered screens (TanStack Query, docs/00-overview.md's locked-in
// stack) call same-origin `/api/v1/...` — this proxies them server-side to
// the Rust API. Keeps the browser same-origin the whole time: no CORS to
// configure on the API, and the session cookie (set on the Next.js origin by
// app/login/actions.ts) is forwarded transparently, Set-Cookie included.
const API_INTERNAL_URL = process.env.API_INTERNAL_URL ?? "http://localhost:8080";

const nextConfig: NextConfig = {
  // Step 7.4: a self-contained runtime bundle (only the actually-used dependency subset, traced by
  // Next.js itself) instead of shipping the whole `node_modules` (devDependencies included —
  // eslint, tailwind's build-time tooling, @types/* — into the runtime image, which is what
  // Dockerfile's runtime stage did before this: ~936MB for a Next.js app is that, not a genuinely
  // large app). `web/Dockerfile`'s runtime stage copies `.next/standalone` instead.
  output: "standalone",
  async rewrites() {
    return [
      {
        source: "/api/v1/:path*",
        destination: `${API_INTERNAL_URL}/api/v1/:path*`,
      },
    ];
  },
};

export default nextConfig;
