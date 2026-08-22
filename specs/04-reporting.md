# 04 — Reporting, Ledgers, Statements, Printing and Exports

> Scope: everything the legacy Delphi/VCL "arzi" application does to **read** accounting data
> back out — trial balances, journals, ledgers, running-account cards, generic reports,
> the FastReport print pipeline, and the Excel/CSV/image export pipeline.
>
> Companion docs: `01-glossary.md` (Persian→English term map — read first),
> `03-accounting-core.md` (posting model), `07-parties-and-shareholders.md`,
> `08-platform-and-security.md`, `09-unit-index.md`.
>
> **Given context (established by sibling agents, not re-derived here):**
> - `CO_ID` / `COID` is a **fiscal-year** id, not a tenant id. One physical DB; every
>   transactional table carries a `*_COID` stamp. Every report is therefore year-scoped.
> - `Sarfasl` (chart of accounts) is **global across years**. Levels Kol / Moein /
>   Tafsil1 / Tafsil2 form a 4-segment code. Postable = leaf (`S_Child = 0`).
>   There is **no account-type/nature column**; nature is implied by hard-coded Kol
>   number ranges in `Sarfasl_SelectU.pas`.
> - `Moein` = voucher **lines** (system of record). `DMoein` = voucher **headers** whose
>   stored totals are drift-prone caches. Voucher state machine `0 → 1 → 2`.
> - Persian dates are stored as **strings**; two incompatible Jalali algorithms are live.
> - Stored-procedure **bodies do not exist in this repo**.

This document has been split into per-section files under [`04-reporting/`](04-reporting/00-index.md)
for easier review; all content is unchanged and verbatim, just reorganized. Section numbers referenced
elsewhere as `04-reporting.md §N` still apply — look up `§N` in the table below. Start at the
[index](04-reporting/00-index.md).

## Table of contents

### §1 — Report catalogue
- [04-01-a-report-catalogue.md](04-reporting/04-01-a-report-catalogue.md) — §1.0–1.3
- [04-01-b-report-catalogue.md](04-reporting/04-01-b-report-catalogue.md) — §1.4–1.8
- [04-01-c-report-catalogue.md](04-reporting/04-01-c-report-catalogue.md) — §1.9–1.13

### §2 — Trial balances in depth
- [04-02-a-trial-balances-in-depth.md](04-reporting/04-02-a-trial-balances-in-depth.md) — §2.0–2.1
- [04-02-b-trial-balances-in-depth.md](04-reporting/04-02-b-trial-balances-in-depth.md) — §2.2–2.3

### §3 — General and subsidiary ledgers
- [04-03-a-general-and-subsidiary-ledgers.md](04-reporting/04-03-a-general-and-subsidiary-ledgers.md) — §3.0–3.1
- [04-03-b-general-and-subsidiary-ledgers.md](04-reporting/04-03-b-general-and-subsidiary-ledgers.md) — §3.2–3.4
- [04-03-c-general-and-subsidiary-ledgers.md](04-reporting/04-03-c-general-and-subsidiary-ledgers.md) — §3.5–3.6

### §4 — Card Jari (running account statement)
- [04-04-a-card-jari.md](04-reporting/04-04-a-card-jari.md) — §4.0–4.7
- [04-04-b-card-jari.md](04-reporting/04-04-b-card-jari.md) — §4.8–4.11

### §5 — Date-range and fiscal-year filtering semantics
- [04-05-date-range-and-fiscal-year-filtering-semantics.md](04-reporting/04-05-date-range-and-fiscal-year-filtering-semantics.md) — §5.1–5.7

### §6 — Print pipeline
- [04-06-a-print-pipeline.md](04-reporting/04-06-a-print-pipeline.md) — §6.1–6.5
- [04-06-b-print-pipeline.md](04-reporting/04-06-b-print-pipeline.md) — §6.6–6.10

### §7 — Export pipeline
- [04-07-export-pipeline.md](04-reporting/04-07-export-pipeline.md) — §7.1–7.6

### §8 — Rebuild recommendations
- [04-08-rebuild-recommendations.md](04-reporting/04-08-rebuild-recommendations.md) — §8.0–8.3

### §9 — Open questions
- [04-09-open-questions.md](04-reporting/04-09-open-questions.md) — §9.1–9.4

### §10 — PROPOSED IMPROVEMENTS (needs user approval)
- [04-10-a-proposed-improvements.md](04-reporting/04-10-a-proposed-improvements.md) — Group A, Group B
- [04-10-b-proposed-improvements.md](04-reporting/04-10-b-proposed-improvements.md) — Group C, Recommended sequencing

### §11 — Naming map
- [04-11-naming-map.md](04-reporting/04-11-naming-map.md) — §11.1–11.5

See [`04-reporting/00-index.md`](04-reporting/00-index.md) for the same table with approximate line
counts per file.
