_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 10. Backup, restore, new-year creation and import

Six flows, in the order an operator meets them.

### 10.1 Automatic daily backup — `TMain.DoBackup`

**Trigger:** runs unconditionally during login, from the licence-check path (`Mainu.pas:894`).
Not a menu item; the operator never sees it.

`Mainu.pas:393-414`:

```pascal
procedure TMain.DoBackup;
var s:String;
begin
      if DM.Backup >0  then exit;                      // once per process
      Dm.Backup := 1;
      Ed1.SetToDate(Date());
      S:= Dm.Base.FieldByName('BackupDir').AsString
                     + Copy(Ed1.Farsi_Date,1,4) + Copy(Ed1.Farsi_Date,6,2) + Copy(Ed1.Farsi_Date,9,2)
                     + '.bak' ;
      if not FileExists(S) then
      Begin
          S:=UpperCase(S);
          Dm.Q1.Close;
          Dm.Q1.ConnectionString := Dm.Ado.ConnectionString;
          Dm.Q1.SQL.Clear;
          Dm.Q1.SQL.Add('DECLARE @isExists INT');
          Dm.Q1.SQL.Add(' exec master.dbo.xp_fileexist '+ QuotedStr(S) +', @isExists OUTPUT');
          S:= 'Backup Database '+Dm.Ado.DefaultDatabase +' To Disk =' + QuotedStr(S);
          Dm.Q1.SQL.Add('  if @isExists=0 '+S );
          Dm.Q1.ExecSQL;
      End;
end;
```

Steps and preconditions:

| # | Step | Precondition | Failure mode |
|---|---|---|---|
| 1 | in-process guard `DM.Backup > 0` | `Dm.Backup` initialised to `0` at `Dmu.pas:722` | none |
| 2 | build the filename `<Base.BackupDir><YYYYMMDD>.bak` from the **Jalali** date with the slashes stripped | `Base` must be open and positioned; `BackupDir` must end with a path separator — **nothing enforces this**, so a `BackupDir` of `D:\bak` produces `D:\bak14030527.bak` | silent wrong path |
| 3 | client-side `FileExists(S)` | the path must be reachable **from the workstation** | if the path is a server-local path (`E:\...`), `FileExists` is false every time |
| 4 | server-side `xp_fileexist` | the SQL Server **service account** must be able to see the path; `xp_fileexist` is an undocumented extended procedure requiring elevated rights | permission error → whole `ExecSQL` fails |
| 5 | `BACKUP DATABASE <DefaultDatabase> TO DISK = '<path>'` if `@isExists = 0` | the service account needs write permission on the directory | error surfaces as an unhandled Delphi exception at login |

Notes:

- **The path is evaluated twice against two different filesystems** (step 3 client, step 4 server).
  They only agree when `BackupDir` is a UNC path visible to both, or the workstation *is* the server.
- **One backup per Jalali day, full only.** No differential, no log backup, no retention policy, no
  verification, no compression, no checksum, no off-site copy. Files accumulate forever.
- The filename derives from the **client's** Jalali date via the third-party control's
  `Farsi_Date` — not from `DM.Current_Date` (§6.2). A workstation with a wrong clock writes a
  wrong-dated backup or skips the day.
- `Dm.Ado.DefaultDatabase` is interpolated straight into the statement with no bracket quoting.
- `xp_fileexist` returns `0` when the file is missing **or** when permission is denied, so a
  permissions problem silently triggers a redundant backup rather than reporting.

### 10.2 Manual export to Absolute Database — `Backup_U.pas`

**Trigger:** the "Backup" toolbar button, gated on permission key `1110`
(`Mainu.pas:947`: `B_Backup.Enabled := Dm.IsEnabel(Dm.userId, 1110)`), handler `Mainu.pas:372-375`.

This is **not** a database backup. It is a row-by-row export of every table into a single
Absolute Database (`.abs`) file — the only place ABS is still used (§1.1).

`Backup_U.pas:41-52` — `init` proposes the filename:

```pascal
D1.SetToDate(Date());
F_P.Text := Dm.Base.FieldByName('BackupDir').asstring +
    Copy( D1.Farsi_Date, 1,4 )+Copy( D1.Farsi_Date, 6,2 )+Copy( D1.Farsi_Date, 9,2 )+'.ABS';
```

Same `<BackupDir><YYYYMMDD>` convention as §10.1, extension `.ABS`. The operator may edit it.

`Backup_U.pas:128-181` — `BitBtn1Click` runs the export:

1. Create the ABS database (`Backup_U.pas:140-146`):
   ```pascal
   Db.Password := 'Mohsen'+ inttostr(68411)+inttostr(211);   //  → 'Mohsen68411211'
   Db.DatabaseFileName := F_P.Text;
   Db.DatabaseName := F_P.Text;
   db.PageSize := 4096 * 2 ;
   Db.CreateDatabase;
   Db.Open;
   ```
   **The container password is a compile-time constant assembled from three literals** — the same
   for every installation, exactly like the ini key `53269` (§1.3). Treat every `.abs` backup in
   the field as unencrypted.
   `CreateDatabase` on an existing path is destructive: re-running on the same day overwrites.
2. Enumerate tables with `DM.ADO.GetTableNames(Tables, False)` (`Backup_U.pas:152`) — i.e. whatever
   the **current catalog** exposes, so the table list is discovered, never declared. Tables whose
   name starts with `temp_` are skipped (`Backup_U.pas:157`).
3. For each table, `Create_Table` (`Backup_U.pas:59-126`):
   - open the SQL table, mirror its field definitions into an ABS table, `DeleteTable` first if it
     already exists (`:69`);
   - **type mapping** (`:81-103`):

     | Source `TFieldType` | ABS target | Loss |
     |---|---|---|
     | `ftSmallint, ftInteger, ftWord, ftBoolean, ftBytes, ftAutoInc, ftLargeint, ftBCD, ftFloat, ftDateTime` | same type | none |
     | `ftFMTBcd` | **`ftString(25)`** | **lossy** — `decimal(38,3)` quantities become 25-char text (§7.3) |
     | `ftString` | `ftString(DataSize-1)` | none |
     | `ftWideString` | `ftWideString(DataSize*2)` | none |
     | **anything else** (`ftBlob`, `ftMemo`, `ftGraphic`, `ftVariant`, `ftGuid`, `ftTimeStamp`, …) | **silently dropped** — the `else` branch is an empty block with its diagnostics commented out (`:100-103`) | **columns vanish with no warning** |
   - copy rows one at a time with `Append`/`Post`, **skipping the column named `ARM`**
     (`:116-117`) — the organisation logo is deliberately excluded;
   - close everything, **including `DB.Close` at `:124`** — inside the per-table routine, so the
     database is closed after the first table and each subsequent `Create_Table` relies on ABS
     re-opening implicitly. Fragile.
4. Show `'  پشتیبان گیری انجام شد  '` — *"backup completed"* (`Backup_U.pas:178`).

**Fidelity assessment: this is not a usable backup.** It loses BLOB/memo columns, loses the logo,
degrades high-precision decimals to text, records no indexes, no constraints, no identity seeds, no
stored procedures, and copies rows outside any transaction while other users are writing.

### 10.3 Restore — **does not exist**

Repo-wide grep finds **no** restore path: no `RESTORE DATABASE`, no code that reads a `.bak`, and
no code that reads an `.abs` back into SQL Server. `TABSTable` appears in exactly two places
(`Backup_U.pas:14` — the export above; `AnbarFactorU.pas:38` — an unrelated in-memory scratch table
for invoice-line editing, §1.1).

Restoring an arzi installation is therefore a **manual DBA operation** using SQL Server tooling
against the `.bak` from §10.1. The `.abs` files are, at best, a forensic archive.

### 10.4 Creating a new fiscal year — `MakeNewU.pas`

**Trigger:** menu → "new fiscal year". Form `TMakeNew`.

`MakeNewU.pas:62-82` — `init` pre-fills from the **currently selected** `Base` row:

```pascal
Coname.Text  := Dm.Base.FieldByName('Co_Name').AsString ;
CoSuB.Text   := Dm.Base.FieldByName('Co_Sub').AsString ;
COID.Text    := inttostr( Dm.Base.FieldByName('Co_ID').AsInteger +1);      // next year
FromDate.Text := Dm.Base.FieldByName('FromDate').AsString ;
Fromdate.Farsi_year := Fromdate.Farsi_year + 1;                            // shift both bounds +1 year
ToDate.Text  := Dm.Base.FieldByName('ToDate').AsString ;
ToDate.Farsi_year := ToDate.Farsi_year + 1;
Backup_Dir.Text := Dm.Base.FieldByName('BackupDir').AsString ;
```

`MakeNewU.pas:97-154` — `sBitBtn1Click` performs the creation:

| # | Step | Precondition | Evidence |
|---|---|---|---|
| 1 | open `Base` as a plain `TADOTable` | — | `:104-106` |
| 2 | reject a duplicate year: `T1.Locate('CO_ID', Trim(COID.Text))` → `'شماره شناسايي تکراري است'` (*"the identifier is duplicated"*) | the new `CO_ID` must not exist | `:107-113` |
| 3 | `T1.Append`, then **copy every field of the current year's row verbatim**: `For I:=0 to T1.FieldCount-1 Do T1.FieldByName(...).AsString := DM.Base.FieldByName(...).AsString` | — | `:116-118` |
| 4 | overwrite `Co_ID`, `FromDate`, `ToDate`, `BackupDir`, `Co_Name`, `Co_Sub` from the form | — | `:119-124` |
| 5 | `T1.Post` | — | `:125` |
| 6 | message `'سال مالي جديد اضافه شد'` — *"the new fiscal year has been added"* | — | `:151` |

Observations:

- **The whole operation is one `INSERT` into `Base`.** No transactional data is created, copied or
  migrated; no schema objects are created. This confirms §1.4: one physical database, years
  separated only by the `*_COID` stamp.
- Step 3 copies **every** column including `ARM` (logo), `No_Ko…No_Ta2` (code widths), `C1081`,
  `C1082` (system accounts) — and also `IsActive`. Since it is copied from the previous row, a new
  year inherits the previous year's active flag. There is **no code anywhere that sets
  `IsActive`** (§12) — meaning the "archive the year" gate at `Dmu.pas:1008-1014` is driven by a
  column no screen writes. It must be set directly in SQL.
- The two validations the author intended are **not implemented** — the comments
  `// Check For COID is integer` and `// Check Date` (`MakeNewU.pas:102-103`) sit above no code.
  `COID.Text` goes into `Locate` and then into `AsString` unvalidated.
- The per-year copy of the chart of accounts is **commented out** (`MakeNewU.pas:129-150`) — the
  abandoned attempt to make `Sarfasl` year-scoped. Master data stays global.
- No transaction, no lock; two operators can create the same year concurrently (§5.6 R1 applies —
  `Locate` is a client-side scan of a `ctStatic` snapshot).


---

[← 02-09-concurrency-locking-and-transactions.md](02-09-concurrency-locking-and-transactions.md) | [02-10-b-year-end-and-import.md →](02-10-b-year-end-and-import.md)
