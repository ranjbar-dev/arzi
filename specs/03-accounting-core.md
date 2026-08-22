# 03 — Accounting Core: Chart of Accounts and Vouchers (Sanad)

Specification extracted from the legacy Delphi/VCL application at `C:\Users\root\Desktop\arzi\arzi`.
This is a **behavioural specification for the Rust + React + PostgreSQL rebuild**. The Delphi code is
the source of truth for *logic only*; its naming, structure and UI idioms are not to be copied.

Read `docs/01-glossary.md` first. All proposed English names in this document come from there.

Every behavioural claim is cited as `file.pas:line`. Persian strings are quoted verbatim followed by
an English translation.

---

This document has been split into per-section files under `docs/03-accounting-core/` for readability.
Cross-references elsewhere in this repo of the form `03-accounting-core.md §N` refer to the section
numbers below; see the linked file for that section's content. Full index (with opening paragraphs
and approximate line counts): [docs/03-accounting-core/00-index.md](03-accounting-core/00-index.md).

| Section | Title | File(s) |
|---|---|---|
| 0 | Reading notes, encoding, and dead code | [03-00-reading-notes-encoding-dead-code.md](03-accounting-core/03-00-reading-notes-encoding-dead-code.md) |
| 1 | The account hierarchy model | [03-01-a](03-accounting-core/03-01-a-account-hierarchy-model.md), [03-01-b](03-accounting-core/03-01-b-account-hierarchy-model.md) |
| 2 | Account CRUD rules | [03-02-a](03-accounting-core/03-02-a-account-crud-rules.md), [03-02-b](03-accounting-core/03-02-b-account-crud-rules.md) |
| 3 | The voucher (Sanad) model | [03-03-a](03-accounting-core/03-03-a-voucher-sanad-model.md), [03-03-b](03-accounting-core/03-03-b-voucher-sanad-model.md), [03-03-c](03-accounting-core/03-03-c-voucher-sanad-model.md) |
| 4 | Voucher validation rules | [03-04-voucher-validation-rules.md](03-accounting-core/03-04-voucher-validation-rules.md) |
| 5 | Voucher line editing behaviour | [03-05-a](03-accounting-core/03-05-a-voucher-line-editing-behaviour.md), [03-05-b](03-accounting-core/03-05-b-voucher-line-editing-behaviour.md) |
| 6 | Automatic voucher generation | [03-06-a](03-accounting-core/03-06-a-automatic-voucher-generation.md), [03-06-b](03-accounting-core/03-06-b-automatic-voucher-generation.md) |
| 7 | Merging vouchers — `MergeSanad.pas` | [03-07-merging-vouchers-mergesanad-pas.md](03-accounting-core/03-07-merging-vouchers-mergesanad-pas.md) |
| 8 | Journal (Rooznameh) generation | [03-08-journal-rooznameh-generation.md](03-accounting-core/03-08-journal-rooznameh-generation.md) |
| 9 | Period close and year-end | [03-09-a](03-accounting-core/03-09-a-period-close-and-year-end.md), [03-09-b](03-accounting-core/03-09-b-period-close-and-year-end.md), [03-09-c](03-accounting-core/03-09-c-period-close-and-year-end.md) |
| 10 | Aggregation / consolidation — `TajmiU.pas` | [03-10-aggregation-consolidation-tajmiu-pas.md](03-accounting-core/03-10-aggregation-consolidation-tajmiu-pas.md) |
| 11 | Index of all SQL in the accounting core | [03-11-index-of-all-sql-in-the-accounting-core.md](03-accounting-core/03-11-index-of-all-sql-in-the-accounting-core.md) |
| 12 | Screen-by-screen UI specification | [03-12-a](03-accounting-core/03-12-a-screen-by-screen-ui-specification.md), [03-12-b](03-accounting-core/03-12-b-screen-by-screen-ui-specification.md), [03-12-c](03-accounting-core/03-12-c-screen-by-screen-ui-specification.md) |
| 13 | Permissions | [03-13-permissions.md](03-accounting-core/03-13-permissions.md) |
| 14 | Open questions | [03-14-open-questions.md](03-accounting-core/03-14-open-questions.md) |
| 15 | PROPOSED IMPROVEMENTS (needs user approval) | [03-15-proposed-improvements-needs-user-approval.md](03-accounting-core/03-15-proposed-improvements-needs-user-approval.md) |
| 16 | Naming map | [03-16-naming-map.md](03-accounting-core/03-16-naming-map.md) |
