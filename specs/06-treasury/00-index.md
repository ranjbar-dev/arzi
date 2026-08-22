# 06 — Treasury: Cheques, Deposit Slips, Petty Cash, Bank Settings

> Domain specification extracted from the legacy Delphi/VCL application `arzi`.
> Source is a **logic reference only**. Naming proposals are new English names for the
> Rust + React + PostgreSQL rebuild.
> Read `docs/01-glossary.md` first for the Persian→English term map.

This document was split into per-section files for readability. Cross-references elsewhere in
the docs of the form `06-treasury.md §N` refer to the section numbers below, which are unchanged
from the original single-file version.

| Section | Title | File | Approx lines |
|---|---|---|---|
| 1 | Entity model | [06-01-entity-model.md](06-01-entity-model.md) | 216 |
| 2 | The cheque state machine | [06-02-cheque-state-machine.md](06-02-cheque-state-machine.md) | 292 |
| 3 | Received versus issued cheques | [06-03-received-versus-issued-cheques.md](06-03-received-versus-issued-cheques.md) | 86 |
| 4 | Endorsement / transfer to a third party | [06-04-endorsement-transfer-third-party.md](06-04-endorsement-transfer-third-party.md) | 71 |
| 5 | Due-date logic | [06-05-due-date-logic.md](06-05-due-date-logic.md) | 138 |
| 6 | Deposit slips (Fish) | [06-06-deposit-slips-fish.md](06-06-deposit-slips-fish.md) | 138 |
| 7 | Petty cash (Tankhah) | [06-07-petty-cash-tankhah.md](06-07-petty-cash-tankhah.md) | 149 |
| 8 | Accounting integration | [06-08-accounting-integration.md](06-08-accounting-integration.md) | 161 |
| 9 | Validation rules | [06-09-validation-rules.md](06-09-validation-rules.md) | 184 |
| 10 (a) | SQL and stored procedures — §10.0–§10.5 | [06-10-a-sql-and-stored-procedures.md](06-10-a-sql-and-stored-procedures.md) | 143 |
| 10 (b) | SQL and stored procedures — §10.6–§10.7 | [06-10-b-sql-and-stored-procedures.md](06-10-b-sql-and-stored-procedures.md) | 127 |
| 10 (c) | SQL and stored procedures — §10.8–§10.12 | [06-10-c-sql-and-stored-procedures.md](06-10-c-sql-and-stored-procedures.md) | 142 |
| 11 (a) | Screen specifications — §11.1–§11.2 | [06-11-a-screen-specifications.md](06-11-a-screen-specifications.md) | 132 |
| 11 (b) | Screen specifications — §11.3–§11.7 | [06-11-b-screen-specifications.md](06-11-b-screen-specifications.md) | 131 |
| 11 (c) | Screen specifications — §11.8–§11.15 | [06-11-c-screen-specifications.md](06-11-c-screen-specifications.md) | 153 |
| 12 | Open questions | [06-12-open-questions.md](06-12-open-questions.md) | 105 |
| 13 | PROPOSED IMPROVEMENTS (needs user approval) | [06-13-proposed-improvements.md](06-13-proposed-improvements.md) | 72 |
| 14 | Naming map | [06-14-naming-map.md](06-14-naming-map.md) | 227 |

Sections 10 and 11 exceeded ~300 lines in the original and were further split at their `###`
subheadings; the section numbering (`§10.x`, `§11.x`) is unchanged, only the file boundaries are
new.
