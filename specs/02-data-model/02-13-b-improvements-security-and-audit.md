_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 13.13 (C) Model banks, branches and cheque books

**Current.** A "bank account" is just a leaf `Sarfasl` node. **The issuing bank, the branch, the
drawer's account number and the drawer's name for a received cheque are recorded nowhere** — the
operator can only smuggle them into the free-text `S_Desc` (`06-treasury.md` §1.1, §1.7). There is
no cheque-book, series or serial-range entity at all (`06-treasury.md` §1.8), and individual issued
cheques do not even carry numbers — only the batch does, as free-text `CM_No` (§1.4).
`CheckDetail.CD_BankNo` holds the payee's account number or IBAN as unvalidated free text, even
though `TUtil.IS_ShabaNo` exists (`Utility.pas:90`) and is not called.
**Proposed.** A `banks` / `bank_branches` / `bank_accounts` trio (seeded from `BN_*` if it exists,
§12.13), `cheques.drawer_bank_account_id`, and a `cheque_books` table with issued-cheque serials.
**Why.** Reconciling a bank statement is impossible without it; so is detecting a duplicate cheque
number (§13.14).
**Risk.** Pure addition — no existing behaviour changes — but it is **new scope**, needs new UI, and
the historical data cannot be back-filled (the information was never captured).
**Cost of not adopting.** Bank reconciliation stays a manual exercise.

### 13.14 (A) ⚠ Uniqueness and validation on cheque and slip numbers

**Current.** `DCheck.S_CheckNo` and `DFish.S_FishNo` are free text, **never validated and never
checked for uniqueness** (`06-treasury.md` §1.1, §1.3). `CheckDetail.CD_BankNo` (26 chars = an
Iranian IBAN) is never validated. `Sahamdar.S_ShabaNo` *is* validated (`TDM.IsValidShaba`,
`Dmu.pas:196-214`), so the capability exists and is simply not applied consistently.
**Proposed.** Apply `IsValidShaba` to every IBAN column; add `UNIQUE (drawer_bank_account_id,
cheque_number)` once 13.13 exists (a bare `UNIQUE (cheque_number)` is wrong — two banks can issue
the same number).
**Risk.** ⚠ Duplicates certainly exist. Without 13.13 there is no correct uniqueness scope, so this
item **depends on 13.13** and should not be adopted alone.

### 13.15 (C) ⚠ Allocate document numbers at save time, not at form open

**Current.** The voucher/invoice number is allocated when the form opens, so the operator sees it
before saving. Two users opening the same form get the same number (§5.6 R2, R3).
**Proposed.** Allocate inside the save transaction from a per-year counter (§5.7); show `(new)`
until persisted.
**Why.** It is the only way to make the number both gapless and race-free.
**Risk.** ⚠ **Operator-visible change**, explicitly flagged in §5.7 as needing approval. Some
workflows write the number on a paper document before saving.
**Cost of not adopting.** Keep `MAX+1`, keep the race, and accept occasional duplicate numbers.

### 13.16 (C) ⚠ Unify the two rounding rules

**Current.** Two different, undocumented rounding rules coexist: truncation toward zero on the
invoice path (`AnbarFactorAddU.pas:107,168-170`) and round-half-even on the pistachio path
(`PestehD_U.pas`) — §7.4. The invoice path also compounds truncation through an `Extended` float
intermediate, which is why stored `AF_Total` values may not reproduce from stored components
(§7.7 check 3).
**Proposed.** Pick one rule and apply it everywhere.
**Why.** Two rules means two answers to the same arithmetic.
**Risk.** ⚠ **Changes money.** §7.7 already takes the port-as-is position (implement both as named,
tested functions and select per call site). Unification must be an accountant's decision, and it
will make new documents differ from old ones by up to one rial per line.
**Cost of not adopting.** Two rounding functions in the codebase forever — which is the safe
default and what §11 assumes.

### 13.17 (D) Replace plaintext passwords

**Current.** `PassWord.Password varchar(20)`, **plaintext, no hash, no salt, no KDF**
(`08-platform-and-security.md` §3.1, §3.2). Read directly at `GetPassu.pas:84`. There is no
`LastLogin`, `FailedAttempts`, `LockedUntil` or `PasswordChangedAt`. The application connects with a
single shared SQL login stored obfuscated in `CS2` with the constant `53269` (§1.3), so anyone with
the ini file has full database access.
**Proposed.** Argon2id password hashes, per-user database-independent sessions, `DATABASE_URL` from
the environment (§8.6), and the permission check moved from the VCL layer to the API layer.
**Why.** The current authorization check is **presentation-only** — with three exceptions every
call site merely sets `.Enabled`/`.Visible` on a control (`08-platform-and-security.md` §4.2).
There is no authorization at the data layer at all.
**Risk.** Existing passwords cannot be migrated as hashes without a forced reset on first login.
That is an operational event needing a communication plan.
**Cost of not adopting.** Unacceptable; this is the one item where "port as is" should not be on
the table. Recorded here anyway because it is a behaviour change.

### 13.18 (D/A) Make `Pass_Config` writes transactional

**Current.** Permission saving is delete-all-then-reinsert-granted, **not wrapped in a
transaction** (`Admin.pas:192-214`). A crash between the `Delete` and the last `Insert` leaves the
user with a partial or empty permission set. The runtime check is one round trip per permission per
check; `TMain.Reload` issues ~35 queries on every login (`Mainu.pas:907-953`).
**Proposed.** One transaction, or a `user_permissions` row-per-permission upsert; load the whole
permission set once per session.
**Risk.** None — this is a pure defect fix. Listed here only because §13 is where changes go.

### 13.19 (E) Audit columns everywhere, and an audit log for accounting settings

**Current.** Audit coverage is inconsistent: `Moein` has `M_User`/`M_Time` but `M_Time` is not
written by every path (§2.7); `DMoein` has both create and modify pairs but **with the C/M prefixes
apparently swapped** (§2.8, §12.10 item 5); `DCheck`, `DFish`, `CheckMaster`, `TankhahMaster` have
only `S_UserID`/`CM_UserID`, **overwritten on every edit**, so it is really "last editor" and there
is no created/updated distinction and **no timestamp at all** (`06-treasury.md` §1.1); `DCheck2` has
an operator but no timestamp. `Tanzim`, `Base` and `Anbar_Config` writes are un-audited entirely
(§8.3).
**Proposed.** `created_at timestamptz NOT NULL DEFAULT now()`, `updated_at timestamptz`,
`created_by bigint`, `updated_by bigint` on every table (§11), plus an append-only change log for
the settings that alter accounting behaviour (`system_accounts`, `warehouses.*_account_id`,
`warehouses.default_tax_rate`) — §8.6 rule 4.
**Risk.** Historical rows have no creation timestamp to back-fill. Migration must either leave them
null (and the columns nullable for legacy rows) or stamp them with the migration date, which is a
lie. **Recommend nullable with a `legacy_migrated_at` marker.**

### 13.20 (E) Move Jalali out of storage entirely

**Current.** Dates are Jalali strings compared lexicographically; the conversion lives in a binary
control and a stored procedure, neither of which is in this repository (§6, §12.1).
**Proposed.** §6.8 — store `date`, derive Jalali at the edge.
**Risk.** ⚠ This is the largest single change in the migration and it **cannot be attempted until
§12.1 is answered.** §6.8 already prescribes keeping `legacy_date_jalali text` as a shadow column
for the first release so discrepancies stay auditable.
**Alternative if rejected:** keep `char(10)` Jalali columns in PostgreSQL and reproduce the string
comparison. This preserves behaviour bug-for-bug (including the `12/30` leap-year artefacts,
`Dmu.pas:897`) at the cost of making every date range query, every index and every report
non-standard. **Not recommended, but it is a legitimate port-as-is position.**

### 13.21 (E) Stop persisting report date-range defaults

**Current.** `[AnbarReport_F] D1/D2` persists the last report date range to the ini file
(`AnbarReportU.pas:132-133,176-177`, §8.1.2), so a report re-opened next year defaults to last
year's dates.
**Proposed.** Derive the default range from the active fiscal year; do not persist it (§8.6).
**Risk.** Operators who deliberately reuse a range lose that convenience. Trivial either way.

### 13.22 (E) Server-side backup only

**Current.** `BACKUP DATABASE … TO DISK` fired from a client at login, to a `BackupDir` stored on
the fiscal-year row, with an `.abs` export encrypted with the hard-coded constant
`'Mohsen68411211'`, and **no restore path whatsoever** (§10.1–§10.3).
**Proposed.** §10.8 — operator-owned `pg_dump`/WAL archiving, no application involvement, a
documented restore runbook and an automated restore drill.
**Risk.** None technically; it moves a responsibility from the product to operations, which needs
an owner named before cutover.

---

### 13.23 Decision log — fill this in before implementation starts

| # | Item | Category | Decision | Decided by | Date |
|---|---|---|---|---|---|
| 13.1 | Account natural key unique | A | | | |
| 13.2 | Voucher / invoice number unique | A | | | |
| 13.3 | ⚠ Voucher balance constraint | A | | | |
| 13.4 | ⚠ Debit/credit exclusivity + non-negativity | A | | | |
| 13.5 | Real foreign keys | A | | | |
| 13.6 | `accounts.parent_id` | B | | | |
| 13.7 | `accounts.party_id` | B | | | |
| 13.8 | Replace `(M_ID, M_Link)` | B | | | |
| 13.9 | ⚠ Split `Base` | B | | | |
| 13.10 | Unified `system_accounts` | B | | | |
| 13.11 | ⚠ Drop denormalised name/code columns | B | | | |
| 13.12 | ⚠ Split cheque state 1 | C | | | |
| 13.13 | Bank / branch / cheque book model | C | | | |
| 13.14 | ⚠ Cheque number uniqueness + IBAN validation | A | | | |
| 13.15 | ⚠ Allocate numbers at save time | C | | | |
| 13.16 | ⚠ Unify rounding rules | C | | | |
| 13.17 | Password hashing + server-side authz | D | | | |
| 13.18 | Transactional permission save | D/A | | | |
| 13.19 | Audit columns + settings change log | E | | | |
| 13.20 | ⚠ Gregorian storage, Jalali at the edge | E | | | |
| 13.21 | Stop persisting report date defaults | E | | | |
| 13.22 | Server-side backup only | E | | | |


---

[← 02-13-a-improvements-integrity-and-keys.md](02-13-a-improvements-integrity-and-keys.md) | [02-14-a-naming-map-tables-and-columns.md →](02-14-a-naming-map-tables-and-columns.md)
