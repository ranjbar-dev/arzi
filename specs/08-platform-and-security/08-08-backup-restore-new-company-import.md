_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 8. Backup / restore / new company / import

### 8.1 Automatic nightly SQL backup (`TMain.DoBackup`)

`Mainu.pas:393-414`, invoked unconditionally from `Reload` (`Mainu.pas:894`) — i.e.
on **every login**, before the licence check.

| Step | Behaviour | Line |
|---|---|---|
| Guard | `if DM.Backup > 0 then exit;` then `Dm.Backup := 1` — a **process-lifetime** flag, so at most one backup per app run | `Mainu.pas:396-397` |
| Filename | `Base.BackupDir` + today's Jalali date as `YYYYMMDD` (obtained by feeding the Gregorian `Date()` into the hidden `ED1: TFullDate` and slicing `Farsi_Date`) + `.bak` | `Mainu.pas:398-401` |
| Precondition | Client-side `if not FileExists(S)` — checks the **client's** filesystem for a path that will be interpreted by the **SQL Server** | `Mainu.pas:402` |
| Execution | Uppercases the path, then builds a batch: `xp_fileexist` into `@isExists`, and `if @isExists = 0` run `Backup Database <DefaultDatabase> To Disk = '<path>'` | `Mainu.pas:403-412` |

⛔ Problems to fix, not port: no error handling (`ExecSQL` exceptions are unhandled);
`QuotedStr` on a path that came from a user-editable settings field is the only
injection defence; the client-side `FileExists` is meaningless for a remote server;
no retention, rotation, verification, or notification; the backup runs *before* the
licence check so an unlicensed launch still writes one; and every user with permission
1110 or none at all triggers it.

### 8.2 Manual backup to an Absolute Database archive (`Backup_U.pas`)

Form `BackupForm`, ribbon `ايجاد پشتيبان` (perm 1110, `Mainu.pas:947`).

`init` (`Backup_U.pas:41-52`): default target = `Base.BackupDir` + Jalali `YYYYMMDD` +
`.ABS`. The user may edit the path freely (`F_P: TMyEdit`).

`BitBtn1Click` (`Backup_U.pas:128-181`):

1. Create a new ABS database at `F_P.Text`, password `'Mohsen'+'68411'+'211'`,
   `PageSize = 8192` (`:140-146`). ⛔ Hard-coded archive password.
2. `DM.ADO.GetTableNames(Tables, False)` (`:152`).
3. For every table **not** prefixed `temp_` (`:157`), call `Create_Table`.
4. Show `پایان عملیات` ("operation finished") and `پشتیبان گیری انجام شد`
   ("backup completed"), then close (`:171-180`).

`Create_Table` (`Backup_U.pas:59-126`) recreates each table in the ABS file by mapping
field types (`:81-103`), then copies **row by row through the client**
(`Table1.First` … `RecordCount` loop, `:109-121`).

Two data-fidelity bugs to be aware of:

- **The `ARM` field (company logo) is silently skipped** (`Backup_U.pas:116`), so a
  restore loses every logo.
- Field types not in the mapped sets fall into an empty `else` (`:100-103`) and are
  **dropped without warning**. `ftFMTBcd` is coerced to `varchar(25)`; `ftString` loses
  one character of declared size (`DataSize-1`, `:93`); `ftWideString` is doubled
  (`DataSize*2`, `:98`).
- It iterates `For I := 1 to Table1.RecordCount` without advancing on a cursor that may
  not be fully materialised — a classic ADO client-cursor hazard on large tables.

**There is no restore feature.** No unit reads an `.ABS` archive back. Recovery is
manual, out-of-band, and undocumented.

### 8.3 New company / new fiscal year (`MakeNewU.pas`)

Form `MakeNew`, ribbon button `B_Add` — which is **disabled by default** and reachable
only through the Ctrl+Alt drag gesture described in §1.6 (`Mainu.pas:961`,
`Mainu.pas:501-532`).

`init` (`MakeNewU.pas:62-82`) pre-fills from the **current** `Base` row:
`Co_Name`, `Co_Sub`, `Co_ID + 1`, `FromDate`/`ToDate` each with `Farsi_year + 1`
(`:73-77`), and `BackupDir`.

`sBitBtn1Click` (`MakeNewU.pas:97-154`):

| Step | Behaviour |
|---|---|
| Precondition | `T1.Locate('CO_ID', COID.Text)` — if the id already exists, show `Error` and refocus (`:107-113`) |
| Create | `T1.Append`, then **copy every field of the current `Base` row as a string** (`For I := 0 to T1.FieldCount-1`, `:117-118`) and overwrite `Co_ID`, `FromDate`, `ToDate`, `BackupDir`, `Co_Name`, `Co_Sub` (`:119-124`) |
| Chart of accounts | The block that would clone `Sarfasl` into the new year is **entirely commented out** (`:129-150`) — the new fiscal year starts with **no chart of accounts** |
| Confirm | Message box, close (`:151-152`) |

⛔ Not validated: `COID.Text` is not checked to be an integer (the comment at `:102`
admits this), `FromDate`/`ToDate` are not validated, and `IsActive` is copied verbatim
from the source row rather than being explicitly set. The whole-row string copy will
also carry the source year's `C1081`/`C1082` account SSNs, which point at chart-of-accounts
rows that do not exist in the new year.

### 8.4 Import (`InFile.pas`)

Form `InFileF`, opened from `SanadEditF.Import` — whose only caller is the dead
developer button `Button7` (`Mainu.pas:865`, unreachable because of the `exit` at
`Mainu.pas:863`). So **the import feature is currently unreachable from the UI.**

`init` (`InFile.pas:74-84`): clears the path and count, defaults the direction radio to
`RBed` (debit), sets `Tag := 0`.

File selection, `sSpeedButton1Click` (`InFile.pas:86-113`) — the file is itself an
ini file, read through `TMyIni`:

| Check | Requirement | Failure | Line |
|---|---|---|---|
| Magic | `UpperCase([Base] Program)` = `GREENGOLD` | `فایل ورودی اشتباه است` ("wrong input file") then **`Close`** | `:94-99` |
| Type | `UpperCase([Base] Name)` = `SANAD` | same message then `Close` | `:100-105` |
| Size | `[Base] Size` → row count, shown read-only | — | `:109-111` |

Confirm, `Button1Click` (`InFile.pas:44-51`): requires a chosen file (`infile.Tag <> 0`),
a non-zero count (`count.Tag <> 0`) and a non-empty description; then sets
`Tag := 1` and closes. **All three failures are silent** — no message, the button just
appears not to work.

Inputs the caller receives: `InFile.Text` (path), `Count.Tag` (expected rows),
`Desc.Text` (description to stamp on the generated document), `RBed`/`RBes`
(debit/credit side).

⛔ The magic-string check is not a format validation and the file is read with the same
`TMyIni` that resolves its own fallback path; a missing file silently reads defaults.

### 8.5 Preconditions summary

| Flow | Permission | Other preconditions |
|---|---|---|
| Auto SQL backup | **none** | `DM.Backup = 0`; `Base.BackupDir` set; SQL Server can write the path |
| Manual ABS backup | 1110 | Target path writable by the *client*; `DM.ADO` connected |
| Restore | — | **Feature does not exist** |
| New company/year | Supervisor + the hidden drag gesture | `Co_ID` must not already exist |
| Import | none (unreachable) | File has `Program=GreenGold` and `Name=Sanad` |
| Change fiscal year | none | — (`ChangesU.pas`) |
| Post to a fiscal year | — | `Base.IsActive = 1` (`Dmu.pas:1008-1013`) |

---


---

Prev: [7. Licensing / copy protection](08-07-licensing-copy-protection.md) · Next: [9. `Utility.pas` function reference](08-09-utility-pas-function-reference.md)
