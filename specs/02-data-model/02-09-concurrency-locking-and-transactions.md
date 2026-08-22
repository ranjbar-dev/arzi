_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 9. Concurrency, locking and transactions

### 9.1 `LockUnit.pas` does not do record locking

**This is the most important correction in this section.** The name is misleading in exactly the
way `docs/01-glossary.md` §6b warns about.

`LockUnit.pas` (246 lines) contains **no database code whatsoever** — no `uses DB`, no `ADODB`,
no SQL. It is a **hardware-fingerprinting unit for the copy-protection "lock"**:

| Member | What it does | Line |
|---|---|---|
| `TSysInfo.GetHDSerialNumber` | `GetVolumeInformation('C:\')` → volume serial | `LockUnit.pas:26-37` |
| `TSysInfo.GetReg_String` / `SetReg_String` | read/write `HKLM\Software\<PrgName>\<KeyName>` | `LockUnit.pas:39-59` |
| `TSysInfo.ElfHash` | the classic ELF 32-bit string hash | `LockUnit.pas:62-75` |
| `TSysInfo.GetCpuID` | inline `asm` `CPUID` leaf 1 → `EAX-0-ECX-EDX` hex | `LockUnit.pas:78-96` |
| `TSysInfo.GetBiosDate` / `GetVideoDate` | `HKLM\HARDWARE\DESCRIPTION\SYSTEM\SystemBiosDate` / `VideoBiosDate` | `LockUnit.pas:100-120` |
| `TSysInfo.GetSystemName` | `HKLM\HARDWARE\DESCRIPTION\System\Bios\SystemProductName` | `LockUnit.pas:122-131` |
| `TSysInfo.GetCpuName` | `asm` `CPUID` leaves `$80000002-4` → CPU brand string | `LockUnit.pas:133-244` |

Consumers:

- **Licence key derivation** — `testmainU.pas:124-168` combines `ElfHash('Arzi' + paramstr(0))`,
  `ElfHash(GetCpuName + GetCpuID)`, `ElfHash(GetBiosDate + GetVideoDate)`,
  `ElfHash(GetSystemName)` into the activation number; `Mainu.pas:879` validates it. (`test.dpr` is
  the key **generator**, not a test suite — `docs/01-glossary.md` §6b.)
- **Password hashing** — `GetPassword.pas:52`: `Password := SysInfo.ElfHash(Pass.Text);`
  A 32-bit unsalted ELF hash. See `docs/08-platform-and-security.md`.

Note also that the global `SysInfo : TSysInfo` (`LockUnit.pas:21`) is **never instantiated** —
repo-wide grep for `SysInfo :=` returns nothing. The calls work only because none of the methods
dereferences `Self`. Any future member field would crash the application. Do not port this pattern.

**Conclusion: arzi has no record-locking layer.** Whatever concurrency safety exists comes from
the three mechanisms below.

### 9.2 Mechanism 1 — ADO client cursors, optimistic, with no conflict handling

| Setting | Count in `.dfm` | Meaning |
|---|---|---|
| `CursorType = ctStatic` | 186 | client-side static snapshot; the whole result set is materialised in the client and **does not see other users' changes** until re-opened |
| `LockType = ltBatchOptimistic` | 40 (all on `TADOStoredProc`) | batch/disconnected mode; edits accumulate client-side |
| `LockType = ltReadOnly` | 1 | |
| *(unset — default `ltOptimistic`)* | all `TADOTable` handles (`Base`, `Moein`, `Sarfasl`, `Sahamdar`, `AnbarFactor`, …, `Dmu.dfm:438-450` and siblings) | row is locked only during `Post` |

Direct dataset editing is used for real writes, e.g.

```pascal
dm.Moein.FieldByName('M_Bed').AsString := Bed.Inttext;     // ArticleMoeinu.pas:150
dm.Moein.FieldByName('M_Tx').AsInteger := 0;               // ArticleMoeinu.pas:158
```

and `MakeNewU.pas:116-125` (`T1.Append` … `T1.Post`).

**There is not a single `OnUpdateError`, `OnPostError`, `OnReconcileError`, `UpdateBatch` or
`CancelBatch` handler in the entire codebase** (repo-wide grep: zero hits). So when ADO's optimistic
concurrency check fires — "row cannot be located for updating; some values may have been changed
since it was last read" — the user gets a raw Delphi exception dialog and the edit is lost. There
is no merge, no retry, no "reload and reapply".

Combined with `ctStatic`, the practical model is **last-writer-wins on a stale snapshot**, with an
unhandled exception as the only conflict signal.

The two deliberate write-blocks that *do* exist are total, not conditional:

```pascal
procedure TDM.QCheckBeforeDelete(DataSet: TDataSet);   begin Abort; end;   // Dmu.pas:1273-1276
procedure TDM.Anbar_TasfiehBeforeDelete(DataSet: TDataSet); begin Abort; end; // Dmu.pas:700-703
```

— grid-level deletion on those datasets is simply forbidden.

### 9.3 Mechanism 2 — transactions are **T-SQL text inside the batch**, never ADO transactions

`TADOConnection.BeginTrans` / `CommitTrans` / `RollbackTrans` are **never called** (repo-wide grep:
zero hits). Instead the literal strings `Begin Transaction` and `Commit` are appended to the SQL
text of a scratch query and shipped as one batch:

```pascal
QS.SQL.Add(' Begin Transaction');            // FactorPesteh_U.pas:181
   ... 12 statements ...
QS.SQL.Add(' Commit ');                      // FactorPesteh_U.pas:231
QS.Open;
```

Sites (non-exhaustive but complete for the write paths found):

| Unit | `Begin Transaction` | `Commit` | Note |
|---|---|---|---|
| `FactorPesteh_U.pas` | 181 | 231 | pistachio receipt → external invoice + 2 ledger lines |
| `AnbarListU.pas` | 379, **433** | 387, **— none** | the block opened at `:433` has **no matching `Commit`** in the batch |
| `CheckBargashtu.pas` | 207, 220 | 244 | **two** `Begin`, **one** `Commit` — but sequential, see §9.4 |
| `CheckDaryaft2U.pas` | 187, 200 | 224 | same shape |
| `CheckEsterdadU.pas` | 186, 199 | 223 | same shape |
| `CheckVosoolU.pas` | 220, 233 | — | **no `Commit` at all** in the grepped range |
| `CheckDaryaftU.pas` | 326 | 348 | balanced |
| `CheckListU.pas` | 187 | — | unbalanced |
| `EnteghalU.pas` | 250 | — | year-end carry-forward; unbalanced in the grepped range |
| `FISHDaryaftU.pas` | 415, 458 | 454, 481 | balanced pairs |
| `MergeSanad.pas` | 118 | — | |
| `MoeinToRU.pas` | 199 | 219 | balanced |
| `RooznamehViewU.pas` | 320 | — | |
| `SanadViewU.pas` | 450, 486, 525 | 458, 494, 533 | balanced — **except** the `Tx 0→1` confirm path at `:290-306`, which has **no transaction at all** |
| `MoeinZipU.pas` | 624, 628 | — | commented out |

**Every unbalanced pair must be re-checked against the full source before the rebuild** (I read the
grep window, not every full procedure — recorded as an open question in §12). Where a `Begin
Transaction` genuinely has no `Commit`, SQL Server leaves `@@TRANCOUNT > 0` on that session; the
transaction is rolled back when the connection is closed or reset by pooling, so **the writes are
silently discarded** — and, worse, the locks are held until then, blocking every other workstation.

Consequences of the text-in-batch approach:

1. **No error handling.** There is no `SET XACT_ABORT ON`, no `TRY…CATCH`, no `IF @@ERROR <> 0
   ROLLBACK` anywhere in the codebase (grep: zero hits for `xact_abort`, `rollback`). If statement
   3 of 12 fails, statements 1–2 are already applied and the batch continues to the `Commit`, which
   commits the partial work.
2. **The transaction cannot span two batches**, so anything the client must decide between
   statements is necessarily outside the transaction.
3. Isolation is whatever the server default is — **`READ COMMITTED`**. Never set explicitly.
   `MAX(...)+1` allocation is therefore non-serialisable even where it *is* inside a transaction
   (§5.6, R6).

### 9.4 Torn documents — the concrete failure mode

`CheckBargashtu.pas:207-244` (identical structure in `CheckDaryaft2U`, `CheckEsterdadU`,
`CheckVosoolU`) writes one logical treasury document as **two sequential transactions**:

```
Begin Transaction
  Update DCheck  Set S_State=1, S_StateName='چک برگشت شده از بانک'  ...   -- "cheque bounced by the bank"
  Insert DCheck2 ( ... )                                                    -- the reversal record
Commit
Set @Tag = @@Identity
Begin Transaction
  Insert Moein ( ... debit line ... )
  Insert Moein ( ... credit line ... )
Commit
```

If the second transaction fails (constraint, deadlock, connection drop), the cheque is marked
bounced and a `DCheck2` reversal exists, but **no ledger entries were written**. The treasury
sub-ledger and the general ledger disagree, silently, with nothing in the schema to detect it.

Then, *outside both transactions*:

```pascal
Dm.DMoein_Make(S_Sanad2.IntValue, S_Date2.Farsi_Date, 'عملیات خزانه مورخ '+ ...);  // :247
Dm.Dmoein_UpdateMab(S_Sanad2.IntValue);                                            // :248
```

`DMoein_Make` (`Dmu.pas:828-838`) upserts the voucher **header**, and `Dmoein_UpdateMab` recomputes
the denormalised `DM_TBed` / `DM_TBes` / `DM_Count`. Both run on the `QS` scratch query — a
*different* connection (§9.5) — after the commit. A crash between the second `Commit` and
`Dmoein_UpdateMab` leaves a voucher whose stored totals do not match its lines. This is why §7.7
migration check 4 (recompute `DM_TBed`/`DM_TBes` from `SUM(M_Bed)`/`SUM(M_Bes)`) is mandatory.

### 9.5 Connection-per-call defeats what transactions there are

Established in §1.6: `Q1`, `Q2` and `QS` have **no design-time `Connection`** (`Dmu.dfm:351-355`,
`629-634`, `1166-1170`). Callers assign the *string*:

```pascal
Q1.Close;
Q1.ConnectionString := Ado.ConnectionString;      // Dmu.pas:1244-1245, and ~40 other sites
```

Assigning `ConnectionString` (rather than `Connection`) makes the dataset open its **own** ADO
connection. Under OLE DB session pooling that may reuse a pooled physical connection, but it is
**never the same session as `Ado`**, and two scratch queries are not guaranteed to share one either.

Therefore:

- A `Begin Transaction` issued on `QS` **does not cover** anything done through `Q1`, `Ado`, or a
  `TADOTable`. The "transactions" in §9.3 protect only the statements inside their own batch.
- The multi-step helpers that alternate between query objects — `Get_NewSanad_DateID`
  (`Dmu.pas:1461-1477` uses `Q1` and calls `New_Sanad` which also uses `Q1`),
  `Get_SanadDateID_Valid` (`Dmu.pas:1494-1521`), `Is_Admin_Or_Valid_Sanad` (`Dmu.pas:983-995`, uses
  `QS`) — run **each statement in its own implicit auto-commit transaction**.
- Two connections from the same workstation can **deadlock each other**: `Ado` holds an update lock
  on `Moein` while `QS`, on its own session, waits to read it. There is no deadlock retry anywhere.

Also note `Q1`'s `Connection` property is sometimes assigned *as well as* `ConnectionString`, in the
same procedure — `Dmu.pas:924-927` (`is_Sarfasl_Last_Deep`) sets `Q1.ConnectionString` and then
`Q1.Connection := Ado`. The later assignment wins, so *that* call does use the shared connection.
The behaviour is therefore **inconsistent from call site to call site**, which is worse than either
choice consistently applied.

### 9.6 Mechanism 3 — application-level soft locks (freeze flags), not concurrency control

These are the real "locks" in the business sense: an administrator freezes a record so ordinary
users cannot touch it. They are ordinary integer columns, checked by the client only.

| Column | Table | 0 / 1 | Checked by | Bypassed by |
|---|---|---|---|---|
| `S_Lock` | `Sarfasl` (chart of accounts) | 1 = frozen | `TDM.Is_Admin_Or_Valid_Daftar` (`Dmu.pas:920-969`) | `Dm.Admin` |
| `S_Lock` | `Sahamdar` (party register, keyed by `S_Card`) | 1 = frozen | `TDM.Is_Admin_Or_Valid_Jari` (`Dmu.pas:971-981`) | `Dm.Admin` |
| `DM_Lock` | `DMoein` (voucher header) | 1 = frozen | `TDM.Is_Admin_Or_Valid_Sanad` (`Dmu.pas:983-995`) | `Dm.Admin` |
| `FM_Lock` | external `Anbar.dbo.FactorMaster` | 2 = posted to a voucher | set at `MakeSanadU.pas:121` (`Set FM_Lock=2, FM_SanadNo=…, FM_SanadDate=…`), written as `2` on creation at `FactorPesteh_U.pas:199-200` | — |

`Is_Admin_Or_Valid_Daftar` is **hierarchical**: it walks Kol → Moein → Tafsil1 → Tafsil2 and returns
false if *any* ancestor has `S_Lock = 1` (`Dmu.pas:929-968`). A missing ancestor row is treated as
**unlocked** (`if Q1.RecordCount=0 then Begin Result:=True; exit; end;`) — fail-open, not fail-closed.

`Is_Admin_Or_Valid_Jari` is also fail-open: an unknown `S_Card` returns `True` (`Dmu.pas:977-978`).
`Is_Admin_Or_Valid_Sanad` is fail-**closed** for a missing voucher (`Dmu.pas:991`, `Result` stays
`Admin` = `False`). Three helpers, two different failure polarities.

UI toggles: `SahamdarU.pas:78` (`B_LockClick`), `RooznamehViewU.pas:420` (`B_LockClick`, gated on
permission key `1139`, `RooznamehViewU.pas:131`).

**None of these is a concurrency lock.** Nothing prevents two users editing the same unfrozen
voucher simultaneously.

### 9.7 Mechanism 4 — the voucher state machine `M_Tx` / `DM_Tx`

The nearest thing to a workflow lock. `M_Tx` on the lines and `DM_Tx` on the header hold the same
value and move together:

| Value | Meaning | Persian on the UI |
|---|---|---|
| `0` | **draft / editable** (`در حالت تحرير`) | — |
| `1` | **confirmed** | `تاييد اسناد` — "confirm vouchers" (`SanadViewU.pas:287`) |
| `2` | **posted / registered** | `ثبت سند` — "register voucher" (`SanadViewU.pas:472`) |

Transitions, all applied to a **range** of voucher numbers `@S1..@S2` at once:

| From→To | Where | Guard |
|---|---|---|
| 0 → 1 | `SanadViewU.pas:296-301` | `DM_Tx=0` **and `DM_TBed = DM_TBes`** — the voucher must balance. **No `Begin Transaction` on this path.** |
| 1 → 0 | `SanadViewU.pas:450-458` | `DM_Tx=1`; inside a transaction |
| 1 → 2 | `SanadViewU.pas:486-494` | `DM_Tx=1`; inside a transaction |
| 2 → 1 | `SanadViewU.pas:525-533` | `DM_Tx=2`; inside a transaction |

Enforcement elsewhere:

- `TDM.Delete_Sanad_moein` (`Dmu.pas:1279-…`) refuses deletion when `Max(M_Tx) > 0` with
  `'سند در حالت تحرير نيست'` — *"the voucher is not in draft state"* (`Dmu.pas:1301-1307`).
- `Get_NewSanad_DateID` only reuses vouchers with `M_Tx = 0` (`Dmu.pas:1469`).
- `Get_SanadDateID_Valid` refuses to append to a voucher with `Get_SanadMaxTX > 0`
  (`Dmu.pas:1502-1506`, `Dmu.pas:1538-1550`).

The state is duplicated on **both** `Moein` and `DMoein` and kept in step by paired `UPDATE`
statements. The 0→1 path updates `Moein` and `DMoein` in two separate statements **with no
transaction around them** (`SanadViewU.pas:296-301`), so a failure between them leaves the lines
confirmed and the header still draft, or vice versa.

### 9.8 Fiscal-year gate

`TDM.Is_New_Sanad_Valid(COID)` (`Dmu.pas:997-1015`) blocks all posting into an archived year:

```pascal
   if Base.Locate('CO_ID', Coid, [loCaseInsensitive]) = false then
     MessageDlg('   سال مالی پیدا نشد  ', ...);              // "fiscal year not found"
   if Base.FieldByName('IsActive').asinteger <> 1  then
     MessageDlg('          سال مالی مورد نظر بایگانی شده است                '+#13#10+
                '   اجازه تغییر در این سال و صدور فاکتور و سند را ندارید    ', ...);
     // "the requested fiscal year has been archived / you may not modify this year or issue invoices and vouchers"
```

Client-side only. Nothing in the database stops a write to an archived year.

### 9.9 Multi-user failure modes — summary

| # | Failure | Root cause | Severity |
|---|---|---|---|
| F1 | Two users get the same voucher / invoice / account number | `MAX+1` outside any transaction, no unique constraint (§5.6) | **high** — silent data corruption |
| F2 | Lost update: user B overwrites user A's edit made 30 seconds earlier | `ctStatic` snapshot + `ltOptimistic` + **no** `OnUpdateError` handler (§9.2) | **high** |
| F3 | Torn document: treasury record committed, ledger lines not | two sequential transactions per logical document (§9.4) | **high** |
| F4 | Voucher header totals disagree with its lines | `Dmoein_UpdateMab` runs after the commit, on a different connection (§9.4, §9.5) | **high** |
| F5 | Partially applied batch: statement 3 of 12 fails, `Commit` still runs | no `SET XACT_ABORT ON`, no `TRY…CATCH`, no `@@ERROR` check (§9.3) | **high** |
| F6 | Orphaned open transaction holding locks until the connection is recycled | unmatched `Begin Transaction` (§9.3 table) | **high** — blocks the whole site |
| F7 | Self-deadlock: `Ado` vs `QS` on the same workstation | connection-per-call (§9.5); no deadlock retry | medium |
| F8 | `M_Tx` and `DM_Tx` diverge | 0→1 transition not wrapped in a transaction (§9.7) | medium |
| F9 | Writes into an archived fiscal year | `Is_New_Sanad_Valid` is client-side only (§9.8) | medium |
| F10 | Frozen (`S_Lock`) records still editable | soft locks are client-side only, and two of the three helpers fail **open** (§9.6) | medium |
| F11 | Stale grid: a report shows data that changed minutes ago | `ctStatic` cursors are never refreshed | low (usability) |
| F12 | Unhandled Delphi exception dialog as the only conflict UX | no error handlers anywhere (§9.2, §9.3) | low (usability), high (data loss) |

### 9.10 Proposed model for the rebuild

| Concern | Proposal |
|---|---|
| Transaction boundary | **one HTTP request = one database transaction**, opened by middleware, committed on `2xx`, rolled back on any error or panic. Never emit `BEGIN`/`COMMIT` as SQL text. |
| Isolation | `READ COMMITTED` by default. `REPEATABLE READ` for trial balances and period reports so a report cannot straddle a concurrent posting. `SERIALIZABLE` for year-end close and carry-forward (§10). |
| Document numbering | counter rows with `UPDATE … RETURNING` inside the request transaction (§5.7) — removes F1 without `SERIALIZABLE`. |
| Lost updates (F2) | **optimistic concurrency with a real version column**: every mutable table gets `version integer NOT NULL DEFAULT 1`; every `UPDATE` carries `AND version = $n` and bumps it; zero rows affected → `409 Conflict` with the current server state, and the React client shows a proper merge/reload dialog. This is the explicit replacement for the missing `OnUpdateError`. |
| Torn documents (F3, F4) | one document = one transaction, always. `DM_TBed`/`DM_TBes`/`DM_Count` become either a **generated/trigger-maintained** column or a view over `SUM(voucher_lines)`; if kept denormalised, add a deferred `CHECK`/trigger asserting equality at commit. |
| Partial batches (F5) | impossible — a Rust transaction rolls back on the first `Err`. |
| Orphaned transactions (F6) | impossible — RAII/`Drop` on the transaction guard rolls back. Add `idle_in_transaction_session_timeout` and `statement_timeout` in PostgreSQL as a backstop. |
| Deadlocks (F7) | one connection per request from a `deadpool`/`bb8` pool; a bounded retry (2–3 attempts with jitter) on SQLSTATE `40001`/`40P01`; a fixed lock-ordering convention documented per aggregate. |
| Voucher state machine (F8) | `vouchers.status` as a PostgreSQL `enum` (`draft`, `confirmed`, `posted`) on the **header only**; lines derive their status through the FK. Transitions in one `UPDATE`, guarded by `WHERE status = <expected>` so the transition is atomic and idempotent. Add `CHECK` that `total_debit = total_credit` before allowing `draft → confirmed`, mirroring `SanadViewU.pas:298`. |
| Fiscal-year gate (F9) | keep the service-layer check **and** back it with a database trigger or an `RLS`/`CHECK` on `fiscal_years.is_active`, so no code path can bypass it. |
| Soft locks (F10) | keep `accounts.is_locked`, `parties.is_locked`, `vouchers.is_locked` as first-class columns; enforce in the service layer **and** in a trigger. Decide fail-open vs fail-closed **once** — recommend fail-**closed** everywhere — and record the change as a behaviour deviation needing approval (§13), since `Dmu.pas:934`/`977` currently fail open. |
| Staleness (F11) | React Query with explicit invalidation on mutation; optional SSE/WebSocket push for the shared voucher list. |
| Long-running user edits | do **not** hold a database lock across a user session. If the business genuinely needs "this voucher is being edited by Ali", implement it as an explicit, expiring **advisory claim** row (`locked_by`, `locked_at`, TTL) — a business feature, visible in the UI, not a database lock. |
| Hardware fingerprinting / licence lock (`LockUnit.pas`) | **dropped entirely.** A Dockerised server-side product does not node-lock to a client's BIOS date. See `docs/08-platform-and-security.md` and §13. |


---

[← 02-08-b-configuration-base-and-registry.md](02-08-b-configuration-base-and-registry.md) | [02-10-a-backup-and-restore.md →](02-10-a-backup-and-restore.md)
