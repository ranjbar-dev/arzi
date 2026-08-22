_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 4.6 `MakeNewU.pas` — new fiscal year, without a single `INSERT`

`MakeNewU.pas` contains **no SQL text whatsoever**. The new-year row is created through the ADO
dataset API:

```pascal
    T1.Close;
    T1.TableName := 'Base';
    T1.Open;
    if T1.Locate('CO_ID', Trim(COID.Text) , [] ) Then
    Begin
       Application.MessageBox('…','Error');     // "this year already exists"
       T1.Close;
       ActiveControl := COID;
       Exit;
    End;

    // Create Base
    T1.Append;
    For I:=0 to T1.FieldCount-1 Do
        T1.FieldByName( T1.Fields[i].FieldName ).AsString :=  DM.Base.FieldByName( T1.Fields[i].FieldName ).AsString ;
    T1.FieldByName('Co_ID').AsString     := Coid.Text;
    T1.FieldByName('FromDate').AsString  := FromDate.Text;
    T1.FieldByName('Todate').AsString    := ToDate.Text;
    T1.FieldByName('BackupDir').AsString := Backup_Dir.Text;
    T1.FieldByName('Co_Name').AsString   := CoName.Text;
    T1.FieldByName('Co_Sub').AsString    := CoSuB.Text;
    T1.Post;
    T1.Close;
```
`MakeNewU.pas:104-126`

**Intent.** Copy **every column** of the currently open fiscal year's `Base` row into a new row,
then override the six the operator supplied. Fully analysed in §10.4; the SQL-layer findings:

- **Duplicate detection is a client-side `Locate` on an open cursor**, not a `WHERE` and not a
  unique constraint. Two operators creating year 1404 simultaneously both pass the check
  (§5.6, §13.2). The `UNIQUE (year)` of §11.2 closes it.
- **The copy loop is `AsString` for every column**, including `ARM` (an `image`) and the numeric
  code widths. Blob-to-string round-tripping through `AsString` is the reason a new year may lose
  the logo — worth probing before migration.
- **`C1081`/`C1082` are copied verbatim**, so the new year inherits the previous year's system
  accounts — pointing at `Sarfasl.S_SSN` values which, because `Sarfasl` is global (§1.4), remain
  valid. Correct by accident of the global chart of accounts.
- The commented-out block at `MakeNewU.pas:129-…` shows an abandoned attempt to also copy `Sarfasl`
  per year, filtering on a **`Sarfasl.S_COID` column that does not exist** — direct evidence that
  `Sarfasl` was *once* year-scoped and was made global later (§1.4).

**Rebuild.** `POST /api/v1/fiscal-years` — one `INSERT` into `fiscal_years` with the `UNIQUE (year)`
and `EXCLUDE` constraints of §11.2 doing the checking. Nothing is copied, because the letterhead and
code widths no longer live on the year row (§8.6, §13.9).

---

### 4.7 `Backup_U.pas` — no SQL, but a schema walk worth recording

`Backup_U.pas` issues **no SQL statements**. It uses the ADO catalog API and the Absolute Database
component set:

```pascal
    DM.ADO.GetTableNames(Tables,False);          // Backup_U.pas:152 — the schema enumeration
    ...
      for i:=0 to Tables.Count-1 do
        if not(copy(Tables[i], 1,5)='temp_')     // Backup_U.pas:157 — skip scratch tables
```

and then, per table, reflects the live field metadata into an `.abs` table definition:

```pascal
    Table1.TableName := Table_Name ;  Table1.Open;             // source: SQL Server, via ADO
    Table.TableName  := Table_Name;
    if Table.Exists then Table.DeleteTable;                    // destination: Absolute Database
    Table.FieldDefs.Clear;
    For I:=0 to Table1.FieldCount-1 do
      if Table1.Fields[i].DataType in [ ftSmallint, ftInteger, ftWord, ftBoolean, ftBytes,
                                        ftAutoInc, ftLargeint, ftBCD, ftFloat, ftDateTime ] then
        Table.FieldDefs.Add( Table1.Fields[i].FieldName, Table1.Fields[i].DataType, 0, False)
      Else if Table1.Fields[i].DataType in [ ftFMTBcd ] then
        Table.FieldDefs.Add( Table1.Fields[i].FieldName, ftString, 25, False)      // ← lossy
      Else if Table1.Fields[i].DataType in [ ftString ] then
        Table.FieldDefs.Add( Table1.Fields[i].FieldName, Table1.Fields[i].DataType,
                             Table1.Fields[i].DataSize-1, False)
      Else if Table1.Fields[i].DataType in [ ftWideString ] then
        Table.FieldDefs.Add( Table1.Fields[i].FieldName, Table1.Fields[i].DataType,
                             Table1.Fields[i].DataSize*2, False);
    Table.CreateTable;
```
`Backup_U.pas:64-106`

then row-by-row:

```pascal
             FieldName := Table1.Fields[j].FieldName ;
             if FieldName <> 'ARM' then
             Table.FieldByName( FieldName ).Value := Table1.Fields[j].Value;
```
`Backup_U.pas:115-117`

**Findings.**

1. **`ftFMTBcd` → `ftString(25)`.** Every `decimal`/`numeric` column — which is every **quantity**
   in the system (§7.3) — is exported as **text**, and a value needing more than 25 characters is
   truncated. The `.abs` file is therefore **not** a faithful copy. §10.2 records the export as
   lossy; this is the exact mechanism.
2. **The `else` branch is empty** (`Backup_U.pas:100-103`, its body commented out). Any column of a
   type not in the four lists — `image`, `text`, `varbinary`, `uniqueidentifier` — is **silently
   omitted from the destination table**. No error, no log.
3. **`ARM` is skipped by name** (`Backup_U.pas:116`), confirming §2.3. It is the only `image`
   column the author knew about; any other blob is dropped by finding 2 without anyone noticing.
4. **`ftString` fields are created one byte narrower** (`DataSize-1`) — correct, since Delphi's
   `DataSize` includes the null terminator, but it means a full-width value survives only if the
   source length was accurately reported.
5. **`if Table.Exists then Table.DeleteTable`** — the destination is destroyed before the source is
   read. A failure mid-export leaves a partial archive that looks complete.
6. The database password is the hard-coded `'Mohsen'+inttostr(68411)+inttostr(211)`
   (`Backup_U.pas:141`) — assembled from integer literals, presumably to defeat a strings dump.
   §10.8 and §13.22 drop the whole mechanism.
7. `GetTableNames` is the **only place the application enumerates the schema**, and it excludes
   `temp_*` — which is the source of §2.2's row 26.

**Rebuild.** Deleted. Backup is `pg_dump`/WAL archiving, owned by operations, never triggered from
a client (§10.8, §13.22).

---

### 4.8 `INI.pas`, `InFile.pas`, `LockUnit.pas` — no SQL at all

Recorded explicitly, because their absence from a data-layer inventory would otherwise look like an
omission.

| File | Contents | Cross-reference |
|---|---|---|
| `INI.pas` | `TMyIni`, a `TIniFile` subclass whose `WriteString`/`WriteInteger` transparently encrypt with `Encrypt(S, Key: Word = 53269)` (`INI.pas:15-16, 163-169, 182, 206`). Values are written **encrypted**, read **decrypted**. | §1.3 (the `CS2` obfuscation), §8.1 (the ini inventory) |
| `InFile.pas` | The closing-entry **import** dialog. It reads the selected `.GGS` file **as an ini file**, not as SQL: `F.ReadString('Base','Program')` must equal `'GREENGOLD'` and `F.ReadString('Base','Name')` must equal `'SANAD'`, then `F.ReadInteger('Base','Size')` gives the row count (`InFile.pas:93-111`). The actual posting is done by the caller (`BastanHesab.pas` → `SanadMoeinu.pas`). | §10.6 |
| `LockUnit.pas` | Registry reads only — `Software\<PrgName>` and the BIOS/hardware fingerprint keys under `HKLM\HARDWARE\DESCRIPTION\System` (`LockUnit.pas:39-59, 100-131`). **Despite its name it does no record locking** — see §9.1. | §8.5, §9.1 |

**Finding on the `.GGS` format.** The interchange file for closing entries is an **ini file with a
`[Base]` header** carrying a magic string (`GREENGOLD`), a document kind (`SANAD`) and a row count.
There is no checksum, no schema version, and — because the count is read but the rows are parsed
elsewhere — no guarantee that `Size` matches the row count actually imported. §10.8 replaces it with
a versioned, checksummed payload validated server-side before commit.

---

### 4.9 Summary of new findings this inventory produced

Items below were **not** derivable from §2 or §3 and update the rest of this document:

| # | Finding | Affects |
|---|---|---|
| 1 | `Anbar_Jens.AJ_Manfi` is `int NOT NULL DEFAULT 1` — negative stock is allowed **by default** | §11.6 default is currently `false`; flip or confirm (§12.5) |
| 2 | `DCheck2.S_Besssn` is `int`, not `varchar` — the `TStringField` grid declaration is the error | closes the §12.5 / §14.10 uncertainty |
| 3 | `AF_Type` 3 **reduces** stock and 4 **increases** it (`Noin − NoOut − NoBin + NoBOut`) | §11.6 `document_type`, §12.9 |
| 4 | `S_LinkSSN` holds the invoice **number**, not `AF_SSN` — the migration must resolve it | §11.5 `source_id`, §13.8 |
| 5 | Party accounts land in **either** `S_Ta1` or `S_Ta2`, per template (`Jari_Rem`) | §13.7 back-fill of `accounts.party_id` |
| 6 | `SahamdarConfig` is a real table (`SC_K, SC_M, SC_T, SC_Kind, SC_Tik, SC_Rem`) missing from §2 and §11 | §2.2 master list, §11.3, §12.5 |
| 7 | `SahamdarConfig.SC_Tik` is a **global scratch flag written by a SELECT dataset** | §9.9 failure modes |
| 8 | Voucher prev/next navigation (`Dmu.pas:872-877`) omits `M_COID` — cross-year leak | §12.12 |
| 9 | `DMoein_Make` truncates the description to 200 chars although the column is 500 | migration probe |
| 10 | `Delete_Moein_ssn` never refreshes the header totals | §7.7 check 4, §13.19 |
| 11 | The `.abs` export coerces every `decimal` to `varchar(25)` and **silently drops** unhandled types | §10.2, §13.22 |
| 12 | `ADO_RPPCSOLUTION`'s connection string, with `User ID=sa`, is committed in plain text | §13.17 |
| 13 | The `.GGS` interchange file is an ini file with a `GREENGOLD`/`SANAD` magic and no checksum | §10.6, §10.8 |
| 14 | `MakeNewU`'s abandoned block references `Sarfasl.S_COID` — `Sarfasl` was once year-scoped | §1.4 |


---

[← 02-04-c-adhoc-sql-design-time-datasets.md](02-04-c-adhoc-sql-design-time-datasets.md) | [02-05-keys-identity-and-document-numbering.md →](02-05-keys-identity-and-document-numbering.md)
