---
description: Implement steps of docs/phase-*.md against memory.md's tracked state, self-verifying each; keeps going through consecutive steps until one needs the user, then stops for review.
argument-hint: "[step-id like 2.3 | 'status' | 'continue']"
---

# /implement-plans

You are resuming implementation of this project against the phased plan in `docs/01-roadmap.md`
and the per-phase docs `docs/phase-0-foundations.md` … `docs/phase-7-hardening-and-cutover.md`.
Work **one step at a time**. Never silently skip ahead, never implement a step out of order unless
the user explicitly says to.

## 0. Load state

- If `memory.md` does not exist at the repo root, create it now, seeded from every step row in
  `docs/01-roadmap.md`'s table — one entry per step, `status: pending`, empty notes. Use the exact
  format under "memory.md format" below. Do this before anything else.
- Otherwise read `memory.md`. It is the single source of truth for what's actually been built —
  trust it over assumptions, but verify it against the real codebase if anything looks stale (see
  "Reconciliation" below).
- Determine the target step:
  - No `$ARGUMENTS`, or `$ARGUMENTS` = `continue` → the first step in roadmap order whose status is
    not `done`.
  - `$ARGUMENTS` = `status` → report the current state (a compact table: step id, title, status) and
    **stop**. Do not implement anything.
  - `$ARGUMENTS` = a specific step id (e.g. `2.3`) → that step, regardless of what's next in order.
    If its listed dependencies (earlier steps in the same or prior phase) aren't `done`, warn the
    user and ask before proceeding.

## 1. Confirm scope before writing anything

State plainly, before any code changes:
- The step id + title you're about to implement.
- Its **Goal** and **Build** bullets, copied from the matching `docs/phase-N-*.md`.
- Any spec section it depends on that you haven't already read this session — read it now.
- Any open decision (from `specs/11-open-decisions.md`) the step's scope brushes up against that
  isn't yet settled. If one is genuinely blocking, stop and ask rather than guessing.

## 2. Implement

- Implement **exactly** the Build scope of that one step — not more (don't reach into a later
  step's territory even if it'd be convenient right now) and not less.
- Follow the step's Spec refs as the behavioural authority; the docs summarize, the specs are the
  ground truth on exact field names, validation order, and formulas.
- Where the step's Manual test describes something scriptable, write a real automated test
  (unit/integration) in addition to leaving the manual procedure runnable by hand — don't let
  "manual test" become the only check that ever exists for that logic.
- Match the stack decisions in `docs/00-overview.md` (Rust/axum/sqlx, Postgres, Next.js/TypeScript/
  Tailwind CSS 4) and the conventions in `specs/01-glossary.md` §7.

## 3. Verify it yourself — don't hand the user a checklist you could run

If it's runnable from the CLI/Docker (`docker compose up`/`build`, `cargo build`/`test`,
`sqlx migrate run`, `curl`, `psql`), **run it yourself** before claiming the step works — don't just
print instructions and ask the user to check. This includes the step doc's Manual test procedure:
execute the real commands against a real running stack (spin up `docker compose up -d db api` etc.
if it isn't already up), not just describe them. Only fall back to "here's how to check" for things
you genuinely cannot do yourself — a browser click-path, a GUI screen, anything needing credentials
or infra you don't have access to. If a live check fails, fix it and re-verify before moving on; if
it fails for a reason outside this step's scope, say so plainly instead of quietly working around it.

Still tell the user, briefly, how to run/check it themselves for their own reference — but as a
"here's how to look if you want to" note, not as the thing standing between the step and "done."
Cover: how to run the project right now (concrete commands matching what exists as of *this* step,
not the finished system), where to look (exact URL/port/endpoint/table/column/log line), and any
seed data/credentials/fixtures needed first.

## 4. Update memory.md

Update only this step's row:
- `status`: `done`, or `blocked` (with a one-line reason and what's needed to unblock), or
  `in-progress` (if you deliberately stopped partway through and are handing back control).
- `notes`: one or two lines — what was actually built, any deviation from the doc and why, any
  follow-up item created. If you made a judgment call the doc left open, record the call here so it
  isn't re-litigated next time.

Leave every other row untouched. Do not rewrite history for already-`done` steps unless you just
discovered one of them was actually wrong (see Reconciliation).

## 5. Keep going if there's nothing left for the user to do

After step 3's self-verification actually passes and step 4's `memory.md` update is written, decide:

- **Stop and ask for review** if any of: the step is `blocked`; you hit an open decision genuinely
  needing the user's call; the step can only be verified through something you can't do yourself
  (a browser click-path, a GUI screen, real credentials/infra you lack); you deliberately left it
  `in-progress`; or `$ARGUMENTS` named one specific step id (implement that one step, then stop —
  don't infer "continue" from a targeted invocation). Explain what's done, what you verified
  yourself, and exactly what the user needs to do or decide.
- **Otherwise, move on to the next not-`done` step in roadmap order in the same invocation** —
  repeat steps 1–5 for it without waiting for a reply. Keep chaining this way until you hit a step
  that needs the user (per above), run out of pending steps, or finish the phase/scope the user
  asked for. This reverses the old "always stop after one step" default: the user has said that
  when a step is fully self-verifiable and nothing is left for them to do, silence from them isn't
  a blocker — only genuine ambiguity or a hand-off point is.
- Regardless of which path, end every invocation's final message with a summary of everything
  implemented and self-verified this run, and what (if anything) is being handed to the user.

## Reconciliation

If you find the real codebase has diverged from what `memory.md` claims (a step marked `done` that
isn't, or work that exists but was never recorded), fix `memory.md` to match reality **before**
proceeding, and call out the discrepancy to the user plainly — don't silently correct it and move on.

## memory.md format

```markdown
# Implementation memory

Tracks actual build progress against docs/01-roadmap.md. Updated by /implement-plans after every step.

## Phase 0 — Foundations
| Step | Title | Status | Notes |
|---|---|---|---|
| 0.1 | Repo scaffold & Docker Compose | done | ... |
| 0.2 | Database bootstrap & migration tooling | pending | |

## Phase 1 — Platform, tenancy & auth
| Step | Title | Status | Notes |
|---|---|---|---|
| 1.1 | Core schema: tenants, fiscal years, org, users + RLS | pending | |
...
```

One table per phase, in the same order as `docs/01-roadmap.md`. `status` is one of `pending`,
`in-progress`, `blocked`, `done`. Keep `notes` terse — a sentence, not a paragraph; the full detail
lives in the git history and the code itself.
