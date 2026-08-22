# 00 — Overview: what "arzi" is, and how to read these documents

## What this documentation set is for

The `arzi` application is a Persian (Farsi) desktop ERP written in Delphi (Object Pascal / VCL),
backed by Microsoft SQL Server over ADO. It handles double-entry accounting, warehouse inventory
with a pistachio-trading specialisation, cheque and petty-cash treasury, a party register, and a
large body of Persian-language printed reports.

These documents specify that system **as it actually behaves**, so it can be rebuilt as a web
application on a different stack:

| Layer | Target |
|---|---|
| Backend | Rust |
| Frontend | React |
| Database | PostgreSQL |
| Packaging | Docker |
| Test database | SQLite or in-memory |

**The business logic is to be preserved exactly. The code is a logic reference only** — not a
source of coding patterns, architecture, or naming. Legacy identifiers are transliterated Persian
and are frequently misleading; every document therefore proposes clean English names and keeps a
mapping table so nothing is lost.

## Reading order

| # | Document | Size | Read it for |
|---|---|---|---|
| 01 | `01-glossary.md` | 12 KB | **Read first.** Persian→English terms, the naming conventions every other doc follows, and §6b — the legacy names that mean the opposite of what they look like. |
| 02 | `02-data-model.md` | 405 KB | Schema, every table and column, stored procedures, dates, money, config, concurrency, and a full proposed PostgreSQL DDL. |
| 03 | `03-accounting-core.md` | 235 KB | Chart of accounts, the voucher (Sanad) model and state machine, period close, carry-forward. |
| 04 | `04-reporting.md` | ~250 KB | Every report, the trial balances in depth, ledgers, statements, print and export pipelines. |
| 05 | `05-inventory.md` | 334 KB | Items, invoices, stock mathematics, costing, and the pistachio specialisation. |
| 06 | `06-treasury.md` | 172 KB | Cheque state machine, deposit slips, petty cash, and treasury→accounting postings. |
| 07 | `07-parties-and-shareholders.md` | 127 KB | Party register, the fiscal-year/company model, current accounts. |
| 08 | `08-platform-and-security.md` | 127 KB | Navigation tree, startup, authentication, the permission matrix, settings, utilities. |
| 09 | `09-unit-index.md` | 56 KB | Every one of the 126 source units classified, dependency map, dead code, third-party stack. |
| 10 | `10-target-architecture.md` | — | How the above maps onto Rust + React + PostgreSQL + Docker. |
| 11 | `11-open-decisions.md` | — | **Everything awaiting your decision**, consolidated: blocking unknowns and proposed improvements. |

Total: roughly 1.8 MB of specification derived from 126 Pascal units (1.27 MB of code) and their
form definitions.

## What the system does

Six functional areas, in dependency order:

**1. Chart of accounts.** A four-segment account code — Kol (general) / Moein (subsidiary) /
Tafsil1 / Tafsil2. Only leaf nodes (`S_Child = 0`) accept postings. The chart is **global across
fiscal years**, not copied per year. There is no account-type column; account nature is inferred
from hard-coded Kol number ranges.

**2. Vouchers (Sanad).** Double-entry journal documents with header and lines. Lines live in
`Moein` and are the system of record; headers live in `DMoein` and their totals are caches that
can drift. A voucher moves through states `0 → 1 → 2` (draft → issued → permanent), with reverse
transitions. Debit/credit balancing is enforced **only** on the `0 → 1` transition.

**3. Inventory.** Item master, warehouses, and invoice documents covering purchase, sale,
production and transfer. Includes a pistachio-specific weight-and-grade deduction calculator that
is the commercial heart of the product. Inventory events generate accounting vouchers
automatically.

**4. Treasury.** Cheque lifecycle (in hand → at bank → cleared / bounced / returned), bank deposit
slips, and petty-cash expense claims — each generating its own accounting postings.

**5. Parties.** A person and legal-entity register. A counterparty is a leaf account node; contact
and tax details live on the account row itself. There is no separate customer or supplier table.

**6. Reporting.** Trial balances (4- and 6-column), general and subsidiary ledgers, journal,
running account cards, and a large body of FastReport print layouts plus Excel/CSV export.

## The five facts that most shape the rebuild

**1. `CO_ID` is a fiscal year, not a company.** One physical database. Every transactional table
carries a `*_COID` stamp naming its fiscal year; master data (accounts, parties) is global and
unstamped. The `Base` table holds one row per year, and that row also carries the operating
entity's letterhead identity. "Multi-company" is emulated by adding `Base` rows — there is no
isolation of any kind. **A genuinely multi-tenant online product requires a tenancy model that
does not exist in the source.** See `11-open-decisions.md`.

**2. The Jalali (Persian) date format stored in the database cannot be determined from source.**
Dates are stored as strings and all range filtering is string comparison. Two mutually
incompatible conversion algorithms exist in the repo — and the reporting and treasury screens use
**neither**, because conversion happens inside `Tools.TFullDate`, a binary-only control whose
source is not in the project. This blocks writing a migration. It is answerable only by querying
the live database.

**3. Neither the stored-procedure bodies nor the table DDL exist in this repository.** Roughly 30
procedures and several scalar functions are called but undefined here. Their call signatures,
parameters and consumed output columns are documented; their internals are not knowable from
source. `02-data-model.md §12` lists precisely what one session against a live database must dump.

**4. Security must not be ported.** Passwords are stored in plaintext and compared
case-insensitively; the login screen lists every username in a dropdown; new users are created
with no password and can log in immediately. The database connection string — credentials included
— sits obfuscated with a compile-time constant in a world-readable ini file on every workstation,
so the entire permission system is bypassable with any SQL client. Authorization is
presentation-only: 118 of 121 permission checks merely toggle a control's `Enabled` or `Visible`
property. There is no audit trail of any kind. The rebuild needs real server-side authorization;
none of the current mechanism transfers.

**5. A significant fraction of the application does not work.** This is not stylistic legacy debt —
these are defects found in the live code, and each one is a decision about what the rebuild should
do instead:

- Purchase and opening-stock vouchers **do not balance**. Input VAT is disabled behind
  `if false then`, and the discount is posted to the wrong side. `MakeSanadU` has no balance check.
- **Production and transfer documents generate no accounting entry at all** —
  `' Not implemented yet. '`.
- A *report* (`Anbar_Amalkard`) runs an `UPDATE` with **no `WHERE` clause** against the movement
  table on every run. Another (`RoyatJU`) drops and recreates a permanent table per run.
- **Daftar Kol (the general ledger) shows nothing** until someone manually runs "make journal", and
  the rows it then reads are in draft state.
- The consolidated ledger **double-counts its opening balance**; `BedBes` splits the opening period
  one day differently from every other ledger.
- Cheque delete is a no-op behind a bare `Exit;`. Cheque endorsement does not exist despite having
  columns. Every cheque list filter is unreachable. Cheque collection never builds its voucher
  header.
- Invoice counterparty validation is unreachable due to an operator-precedence bug
  (`if not S_Bed.tag=0`), so invoices save with no counterparty.
- The pistachio deduction calculator — the core domain formula — sits on a panel that is never
  shown, behind a Save button with no handler.
- The tax-authority Excel export includes unposted drafts.

Every one of these is documented with `file:line` in its domain document and carried into
`11-open-decisions.md`. **The default position taken throughout is port-as-is; nothing has been
"fixed" in the specification without your approval.**

## Conventions used throughout

- Every behavioural claim cites `file:line`.
- Persian strings are quoted verbatim with an English translation. Where a string was unrecoverable
  from a mis-encoded source file, the document says so rather than guessing.
- Sections titled `PROPOSED IMPROVEMENTS (needs user approval)` are **suggestions only** and are
  quarantined at the end of each document. The as-is specification never silently incorporates them.
- In the proposed PostgreSQL DDL, every constraint is tagged `[AS-IS]`, `[NEW]` or `[VERIFY]`.
  A `[NEW]` constraint is a behaviour change and requires sign-off — the legacy system permitted
  data that a well-formed schema would reject.

## Source encoding note

Source files mix Windows-1256 and UTF-8 per file, and `.dfm` string literals use `#NNNN` decimal
escapes. Any tooling that reads this codebase must detect encoding per file:

```powershell
# Windows-1256 Pascal source
[IO.File]::ReadAllText('FILE.pas', [Text.Encoding]::GetEncoding(1256))
# .dfm escape run
-join((1587,1604,1575,1605) | % { [char]$_ })
```
