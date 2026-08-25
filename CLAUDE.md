# CLAUDE.md

This file provides guidance to Claude Code (claude.ai/code) when working with code in this repository.

## What this repo is

Build plan + specs for rebuilding **arzi** — a Persian (Farsi) desktop ERP (Delphi/VCL + SQL
Server: accounting, inventory with a pistachio-trading specialisation, cheque/petty-cash treasury,
party register) — as a web app. **No application code exists yet** — only `docs/` (the build plan)
and `specs/` (the behavioural specification reverse-engineered from the legacy Delphi source).
Implementation proceeds strictly phase-by-phase against these documents.

## Workflow: use `/implement-plans`, not ad-hoc changes

This repo is driven by the `/implement-plans` slash command (`.claude/commands/implement-plans.md`),
which implements one step at a time from `docs/01-roadmap.md` and tracks progress in `memory.md`
(repo root — created on first run, single source of truth for what's actually built). Default to
that command for any "build the next thing" request rather than picking a step yourself:

- `/implement-plans status` — report progress, implement nothing.
- `/implement-plans continue` — implement the next not-`done` step in roadmap order.
- `/implement-plans <step-id>` — implement a specific step (e.g. `2.3`), out of order.

One step per invocation. It stops for review after each step rather than chaining ahead — do not
implement multiple roadmap steps in one turn unless explicitly told to keep going.

## Document hierarchy — read in this order

1. **`docs/00-overview.md`** — stack table, locked-in decisions, how a "step" is shaped. Read first.
2. **`docs/01-roadmap.md`** — the 46-step checklist, dependency-ordered across 8 phases. This is
   the authoritative build order.
3. **`docs/phase-N-*.md`** — per-phase Goal/Build/Spec-refs/Manual-test detail for each step.
4. **`specs/01-glossary.md`** — Persian→English naming, and §6b (legacy names that mean the
   *opposite* of what they look like — e.g. `CO_ID` looks like a company id but is a fiscal year;
   `Sahamdar` looks like the shareholder register but is the person/legal-entity party register).
   Read before touching anything named after a legacy identifier.
5. **`specs/NN-*.md`** — the full behavioural spec per domain (accounting, inventory, treasury,
   parties, reporting, platform/security), each with `file:line` citations into the legacy Pascal.
6. **`specs/11-open-decisions.md`** — everything still awaiting a decision. If a step's scope
   brushes an open item, stop and ask rather than guessing (also called out by `/implement-plans`).

**`docs/` is the *how* and *when*; `specs/` is the *what* — the behavioural ground truth.** A
`docs/phase-*.md` step cites the exact `specs/...md §N` that governs it; read that section before
building, not after. When they conflict, `specs/` wins on behaviour, `docs/` wins on build order.

The legacy Pascal source itself is **a logic reference only** — business logic (voucher balancing,
stock math, the pistachio deduction formula, period close) is preserved exactly; its architecture,
naming, and defects are not. `specs/00-overview.md` §"five facts" and the Group B defect list are
essential context: a fair amount of the legacy app doesn't actually work (unbalanced purchase
vouchers, no production/transfer postings, no cheque endorsement, etc.), and each defect is a
per-item decision in `11-open-decisions.md` about whether the rebuild ports the bug or fixes it —
already resolved for most of them (see `docs/00-overview.md` "Decisions locked in").

## Target architecture (once scaffolded — see `specs/10-target-architecture.md`)

- **Backend**: Rust, `axum` + `tokio` + `sqlx` (compile-time-checked SQL, no ORM), PostgreSQL 17.
  Module layout mirrors the spec docs 1:1: `auth/`, `accounting/`, `inventory/`, `treasury/`,
  `parties/`, `reporting/`, `platform/`, each with `mod.rs`/`model.rs`/`logic.rs`/`queries.rs`.
- **Frontend**: Next.js (App Router) + TypeScript + Tailwind CSS 4 — this **deviates** from
  `specs/10-target-architecture.md` §3.1 (which specified Vite + React Router); Next.js's router
  replaces React Router, everything else there (TanStack Query/Table, React Hook Form + Zod,
  `react-i18next`) still applies. RTL Persian UI, `dir="rtl"`, CSS logical properties, no Persian
  in identifiers (translation keys only).
  - **SPA, not SSR.** Every `app/**/page.tsx` and `layout.tsx` under `(app)/` is a `"use client"`
    component — no Server Components, no server-side data fetching, no server-side `redirect()` for
    page content or auth guards. Session data (`session.isSuperuser`, `currentFiscalYearId`, ...)
    comes from `lib/use-session.ts`'s `useSession()` (TanStack Query over `/api/v1/me`), the client
    counterpart to the server-only `lib/session.ts#getSession()`. Route params come from
    `useParams()`, not an `async` page awaiting `params`. Tab-default routes (`/inventory`,
    `/treasury`) client-redirect via `useRouter().replace()` in a `useEffect`, not `redirect()`.
    The `(app)/layout.tsx` auth guard is client-side too (redirects to `/login` on a 401 from
    `useSession()`) — this is UX only, exactly like every other client-side permission check above:
    real enforcement is still the API's job on every request.
  - **Two deliberate exceptions**, both pre-existing and left alone: `app/layout.tsx` (the root
    HTML shell — `<html>`/`<body>`, the `Vazirmatn` font, `metadata` export) is necessarily a
    Server Component because Next's App Router requires that for the root layout; it does no data
    fetching or business logic, just static markup. `app/login/*` (`page.tsx`, `actions.ts`) stays
    server-rendered on purpose — `docs/00-overview.md`: "Server-rendered pages are used only where
    they help (login, print views)" — because the session cookie is `httpOnly` and can only be set
    via a `Set-Cookie` response header, which `app/login/actions.ts`'s Server Action does by calling
    the Rust API directly and re-issuing the cookie on the Next.js origin. Don't convert either of
    these without re-solving the cookie/root-layout problem first.
- **Money**: `bigint`/`i64` rials end-to-end, never floating point.
- **Auth**: Argon2id + server-side sessions (nothing from the legacy plaintext/no-auth design
  transfers). Authorization enforced server-side on every request; client-side checks are UX only.
- **Tenancy**: real `tenant_id` on every table (including what was legacy "global" master data),
  enforced with Postgres row-level security — the app's DB role has no `BYPASSRLS`.
- **Packaging**: Docker / `docker-compose` — `api` (Rust, multi-stage/`cargo-chef`), `web`, `db`
  (`postgres:17`).
- **Testing**: `sqlx::test` against disposable real Postgres instances (not SQLite/in-memory,
  despite `specs/`'s original suggestion — see `specs/10-target-architecture.md` §6). Reconciliation
  tests (new system's trial balance/ledgers/stock must match legacy output) are the highest-value
  tests and are built in Phase 7.

Once Phase 0 lands the actual scaffold, the real build/lint/test commands belong here — do not
invent them before the code exists.
