FROM node:20-alpine AS deps
WORKDIR /app
COPY package.json package-lock.json ./
RUN npm ci

FROM node:20-alpine AS builder
WORKDIR /app
# next.config.ts's rewrites() destination is resolved once at `next build`
# time and frozen into the build output — `next start` does not re-read
# next.config.ts or re-evaluate env vars at runtime. So this must be a build
# ARG, not just docker-compose.yml's `environment:` (which only applies at
# container start).
ARG API_INTERNAL_URL=http://api:8080
ENV API_INTERNAL_URL=$API_INTERNAL_URL
COPY --from=deps /app/node_modules ./node_modules
COPY . .
RUN npm run build

FROM node:20-alpine AS runtime
WORKDIR /app
ENV NODE_ENV=production
# Step 7.4: `output: "standalone"` (next.config.ts) traces the actually-used dependency subset into
# `.next/standalone` — this replaces copying the whole `node_modules` (devDependencies included)
# into the runtime image. `.next/standalone` already contains its own minimal `node_modules` and a
# `server.js` entrypoint; `.next/static` and `public` are NOT included in `standalone` and must be
# copied alongside it (Next.js's own documented pattern).
COPY --from=builder /app/public ./public
COPY --from=builder /app/.next/standalone ./
COPY --from=builder /app/.next/static ./.next/static
EXPOSE 3000
CMD ["node", "server.js"]
