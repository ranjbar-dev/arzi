_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 11. Concurrency and multi-user behaviour

### 11.1 What locking exists

Only **advisory, application-level, manually-toggled** locks. There is no pessimistic
row locking, no optimistic-concurrency token, no version column, and no `rowversion`.

| Lock | Column | Semantics | Set / cleared at | Read at |
|---|---|---|---|---|
| Document lock | `DMoein.DM_Lock` | `1` = locked | `SanadViewU.pas:605-619` (lock) and `:621-635` (unlock) — a two-item popup on `B_Lock`; both are plain `Q1.Edit; Q1.Post` | `Dmu.pas:983-995` `Is_Admin_Or_Valid_Sanad` |
| Account lock | `Sarfasl.S_Lock` | `1` = locked | `SNewu.pas:358-365` — toggles with `Update Sarfasl Set S_Lock = <L> Where S_SSN = <R>` | `Dmu.pas:921-966` `Is_Admin_Or_Valid_Daftar` |
| Current-account lock | `Sahamdar.S_Lock` | `1` = locked | `SahamdarU.pas` (grid toggle) | `Dmu.pas:968-981` `Is_Admin_Or_Valid_Jari` |

All three predicates begin `Result := Admin; if Dm.Admin then Exit;` — **a supervisor
ignores every lock** (`Dmu.pas:923-924`, `:970-971`, `:985-986`).

`Is_Admin_Or_Valid_Daftar` walks the account hierarchy from Kol down to Tafzil-2,
returning `False` at the **first** level whose `S_Lock = 1` (`Dmu.pas:926-965`). A missing
level returns `True` — i.e. **absence of a record is treated as "not locked"**.

Grid renderers colour locked rows (`SanadViewU.pas:557-566`, `RooznamehViewU.pas:220-229`,
`SNewu.pas:422-431`, `SahamdarU.pas:310-320`, `:344-354`).

### 11.2 Document state machine (`TX`) as a soft workflow lock

`Moein.M_Tx` / `DMoein.DM_Tx`: `0` = draft (`درحال تحرير`), `1` = approved
(`تاييد شده`), `2` = permanently posted (`ثبت شده`). Transition buttons are visible
only at the right state **and** with the right permission (`SanadViewU.pas:744-757`).
`TDM.Moein_Tx` reads `max(M_TX)` for a document (`Dmu.pas:1166-1178`).

⚠️ This is a **UI-visibility** guard, not a transactional guard. Two users can both see
a `TX = 0` document, both click Approve, and both `Update` will succeed.

### 11.3 ADO cursor and lock configuration

Every dataset in `Dmu.dfm` and the form units uses `CursorType = ctStatic` with
`LockType = ltBatchOptimistic` where a lock type is specified at all
(9 datasets in `Dmu.dfm`, plus `NewFinalu.dfm` ×3, `SanadMoeinu.dfm` ×2, and others).

`ltBatchOptimistic` on a `ctStatic` (client-side snapshot) cursor means:

- The client holds a **stale snapshot** for the life of the form.
- `Post` issues an `UPDATE … WHERE <key> = …` with **no concurrency predicate on the
  changed columns** in the default Delphi configuration.
- **Last writer wins, silently.** No `EDatabaseError`, no "record has changed" prompt.

### 11.4 What happens when two users edit the same document

1. Both open the document browser; both cursors snapshot the same rows.
2. User A edits and posts. The row is updated.
3. User B's grid still shows the old values (nothing refreshes it — there is no
   notification channel, no polling, no `Refresh` on focus).
4. User B edits and posts. **A's changes are overwritten with no warning.**
5. `DMoein.DM_CUser` / `DM_CDate` now record only B (`Dmu.pas:831`). A's edit has vanished
   with no trace anywhere.

For header/line documents this is worse: `DMoein_Make` (`Dmu.pas:815-839`) recomputes
`DM_TBed`, `DM_TBes`, `DM_Count` by aggregating `Moein` in a **separate statement** from
the `Insert`/`Update`, with **no explicit transaction**. A concurrent line insert between
the aggregate and the update leaves the header totals inconsistent with the lines.
`Dmoein_UpdateMab` (`Dmu.pas:841-857`) has the same shape and additionally deletes the
header when `DM_Count = 0` — which will delete a header another user is mid-way through
populating.

### 11.5 Known gaps (all of them)

| Gap | Evidence |
|---|---|
| No optimistic-concurrency token anywhere | no `rowversion`/`timestamp` column in any `Dmu.dfm` dataset |
| No explicit transactions around multi-statement operations | `Dmu.pas:815-857` (`DMoein_Make`, `Dmoein_UpdateMab`), `Admin.pas:192-214` (permission save), `MakeNewU.pas:104-126` (new fiscal year) |
| Client-side `max(x)+1` id allocation — racy | `Admin.pas:172-177` (`UserCode`), `Dmu.pas:1242-1251` (`New_Sanad`), `Dmu.pas:1253-1262` (`New_AnbarFactor`) |
| Locks are advisory and manual; nothing auto-locks on open | `SanadViewU.pas:605-635` |
| Supervisor bypasses every lock silently | `Dmu.pas:923`, `:970`, `:985` |
| No stale-data detection or refresh | no `Refresh` on activate anywhere in the browsers |
| No session concept — the same account can be used concurrently from any number of machines | `GetPassu.pas:94-97` sets globals only |
| No idle timeout, no logout | the only exit is `Application.Terminate` |
| Single shared SQL login for all users — the DB cannot distinguish who did what | `Dmu.pas:726-727` (`CS2`) |
| Extensive SQL built by string concatenation | `Dmu.pas:1557-1558`, `Dmu.pas:929-960`, `GetPassu.pas:72`, `Mainu.pas:408-411`, `FGetCodeU.pas:74-79`, and ~200 more sites |
| `DM.Backup` guard is per-process, not cross-process — N clients can start N simultaneous `BACKUP DATABASE` statements | `Mainu.pas:396-397` |
| `Dmoein_UpdateMab` deletes empty headers unconditionally | `Dmu.pas:855` |

⛔ In the rebuild: PostgreSQL transactions with explicit isolation, DB-generated
identities, a version/`xmin`-based optimistic-concurrency check that surfaces a real
409 to the UI, and per-user database roles or (at minimum) per-user application
sessions with request-scoped identity.

---


---

Prev: [10. Shared dialog / frame catalogue](08-10-shared-dialog-frame-catalogue.md) · Next: [12. What `test.dpr` / `testmainU.pas` is](08-12-what-test-dpr-testmainu-pas-is.md)
