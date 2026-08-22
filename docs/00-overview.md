# 00 — Implementation Plan: How to Use These Documents

This is the build plan for rebuilding **arzi** (see `specs/00-overview.md`) as a web application.
It translates `specs/` (the *what* — behaviour, data model, screens) into an ordered, independently
testable *build sequence* (the *how* and *in what order*).

**Read `specs/00-overview.md`, `specs/01-glossary.md` and `specs/11-open-decisions.md` before
starting any phase.** This plan does not repeat their content — every step below cites the exact
spec section that governs it. When a step's behaviour is unclear, the spec section is the source of
truth, not this document.

## Stack

| Layer | Choice | Note |
|---|---|---|
| Backend | Rust (`axum` + `tokio` + `sqlx`) | Per `specs/10-target-architecture.md` §2 |
| Database | PostgreSQL 17 | Per `specs/10-target-architecture.md` §4 |
| Frontend | **Next.js (App Router) + TypeScript + Tailwind CSS 4** | **Deviates from `specs/10-target-architecture.md` §3.1**, which suggested Vite + React Router. Next.js's own router replaces React Router; everything else in §3.1 (TanStack Query, TanStack Table, React Hook Form + Zod, `react-i18next`) still applies, adapted to Next.js client components. Server-rendered pages are used only where they help (login, print views); the bulk of the app is data-dense client-rendered screens, same as the original plan. |
| Frontend design system | [`ui-ux-pro-max`](https://github.com/nextlevelbuilder/ui-ux-pro-max-skill) Claude Code skill | **Not installed yet.** Before Phase 1 UI work starts, run `/plugin marketplace add nextlevelbuilder/ui-ux-pro-max-skill` then `/plugin install ui-ux-pro-max@ui-ux-pro-max-skill`. It activates automatically on UI-building prompts once installed — no slash command needed per step. |
| Packaging | Docker / `docker-compose` | Per `specs/10-target-architecture.md` §5 |
| Money | `bigint` rials end to end | Per `specs/10-target-architecture.md` §2.3 — no floats, no currency column |
| Auth | Argon2id + server-side sessions | Per `specs/10-target-architecture.md` §2.5 |
| Tenancy | Real `tenant_id` + Postgres RLS on every table | Per A3 in `specs/11-open-decisions.md` |

## Decisions locked in before this plan was written

From `specs/11-open-decisions.md` (already resolved 2026-08-18/19) plus four calls made today —
treat all of these as settled; do not re-litigate them mid-build:

1. **Fresh build, no legacy migration yet.** A1 (Jalali date format), A9 (`CheckMaster` batch
   semantics) and A11–A16 all require querying a *populated* legacy database, which is not
   available. The system is built clean against the proposed PostgreSQL schema. Phase 7 includes a
   migration-readiness doc with the exact queries to run once that access exists — nothing in
   Phases 0–6 depends on it.
2. **A17 — item change-history: build it.** `Anbar_Jens`'s undocumented audit trigger is
   replicated as a first-class feature: item mutations write to the same `audit_log` table every
   other mutation writes to (see Phase 1 step 1.4), not a bespoke shadow table.
3. **B25 — fix.** The purchase/sales report gets a fiscal-year predicate, joining the other 24
   confirmed defects already ruled "fix" in `specs/11-open-decisions.md` Group B.
4. **Build order = dependency order.** Platform/auth → accounting-core → parties → treasury →
   inventory → reporting → hardening/cutover. This matches `specs/00-overview.md`'s "six functional
   areas, in dependency order" and the module layout in `specs/10-target-architecture.md` §2.2.

All 24 original Group B defects are fixed, not replicated (per the existing ruling) — every phase
below says explicitly where a defect fix changes behaviour from the legacy screen. All Group C
improvements are approved (constraints, denormalisation cleanup, dead-code drop, form merges).

## How a "step" works

Every step in `phase-*.md` follows the same shape:

- **Goal** — one sentence.
- **Build** — backend (schema + API) and frontend work, as bullets.
- **Spec refs** — exact `specs/...md §N` sections that define the behaviour. Read these before
  building, not after.
- **Defects fixed / decisions applied** — only where relevant (Bn / An / Cn references).
- **Manual test** — a numbered procedure you can run by hand (`curl`/browser) to confirm the step
  works, before moving to the next one. No step depends on automated tests existing yet, though
  writing them alongside is expected (`specs/10-target-architecture.md` §6).
- **Done when** — the acceptance line for that step.

Steps are ordered so each one only depends on steps already done. Within a phase they can mostly be
built back-to-back; across phases, do not start inventory (Phase 5) before accounting-core (Phase 2)
exists, since every inventory document posts a voucher.

## Explicitly out of scope for this plan

- **Legacy data migration execution** (only the readiness doc — Phase 7).
- **Multi-currency** — the legacy system has none despite its name; not being added
  (`specs/10-target-architecture.md` §7).
- **CI pipeline / automated deploy** — not requested; add when wanted.
- Anything in `specs/*/*-proposed-improvements.md` beyond the Group C themes already approved in
  `specs/11-open-decisions.md` — the ~90 individual suggestions are not re-scored here; if one
  becomes relevant while building a step, flag it rather than silently including or excluding it.

## Roadmap

See `01-roadmap.md` for the full phase/step checklist.
