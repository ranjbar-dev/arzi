# 06 — Treasury: Cheques, Deposit Slips, Petty Cash, Bank Settings

> Domain specification extracted from the legacy Delphi/VCL application `arzi`.
> Source is a **logic reference only**. Naming proposals are new English names for the
> Rust + React + PostgreSQL rebuild.
> Read `docs/01-glossary.md` first for the Persian→English term map.

This document was split into per-section files under [`06-treasury/`](06-treasury/00-index.md)
for readability. Cross-references elsewhere in the docs of the form `06-treasury.md §N` refer to
the section numbers below, which are unchanged from the original single-file version — only the
file boundaries are new. Start at the [index](06-treasury/00-index.md).

## Table of contents

- **§1** [Entity model](06-treasury/06-01-entity-model.md)
- **§2** [The cheque state machine](06-treasury/06-02-cheque-state-machine.md)
- **§3** [Received versus issued cheques](06-treasury/06-03-received-versus-issued-cheques.md)
- **§4** [Endorsement / transfer to a third party](06-treasury/06-04-endorsement-transfer-third-party.md)
- **§5** [Due-date logic](06-treasury/06-05-due-date-logic.md)
- **§6** [Deposit slips (Fish)](06-treasury/06-06-deposit-slips-fish.md)
- **§7** [Petty cash (Tankhah)](06-treasury/06-07-petty-cash-tankhah.md)
- **§8** [Accounting integration](06-treasury/06-08-accounting-integration.md)
- **§9** [Validation rules](06-treasury/06-09-validation-rules.md)
- **§10** SQL and stored procedures
  - [10.0–10.5](06-treasury/06-10-a-sql-and-stored-procedures.md)
  - [10.6–10.7](06-treasury/06-10-b-sql-and-stored-procedures.md)
  - [10.8–10.12](06-treasury/06-10-c-sql-and-stored-procedures.md)
- **§11** Screen specifications
  - [11.1–11.2](06-treasury/06-11-a-screen-specifications.md)
  - [11.3–11.7](06-treasury/06-11-b-screen-specifications.md)
  - [11.8–11.15](06-treasury/06-11-c-screen-specifications.md)
- **§12** [Open questions](06-treasury/06-12-open-questions.md)
- **§13** [PROPOSED IMPROVEMENTS (needs user approval)](06-treasury/06-13-proposed-improvements.md)
- **§14** [Naming map](06-treasury/06-14-naming-map.md)
