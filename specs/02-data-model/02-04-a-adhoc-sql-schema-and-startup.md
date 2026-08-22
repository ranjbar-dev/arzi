_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 4. Ad-hoc SQL inventory

### 4.0 Scope and method

This section inventories **every SQL statement in the data layer** — the units that own database
access rather than screen behaviour:

| File | SQL present? | Role |
|---|---|---|
| `Dmu.pas` | **yes** — 27 distinct statement groups | the data module's methods |
| `Dmu.dfm` | **yes** — 13 dataset SQL blocks + 27 stored-procedure declarations | design-time datasets |
| `MakeNewU.pas` | **no SQL text** — dataset navigation only | new-fiscal-year creation (§10.4) |
| `Backup_U.pas` | **no SQL text** — ADO catalog API + Absolute Database calls | manual `.abs` export (§10.2) |
| `INI.pas` | **none** | ini file + the `53269` obfuscation (§1.3, §8.1) |
| `InFile.pas` | **none** | file-import dialog; reads an *ini-shaped* `.GGS` file (§10.6) |
| `LockUnit.pas` | **none** | registry + machine fingerprint (§8.5, §9.1) |

Statements already quoted in §2 or §3 are **cross-referenced, not repeated**. Screen units
(`SanadEditU.pas`, `CheckDaryaftU.pas`, `AnbarFactorU.pas`, …) carry far more SQL than the data
layer does — that is the defining structural fact of this codebase — and their statements are
documented in the domain specs (`03-accounting-core.md`, `05-inventory.md`, `06-treasury.md`,
`07-parties-and-shareholders.md`); only those reached *through* `TDM` are repeated here.

**Three properties hold across the whole inventory and are stated once:**

1. **Almost every statement is built by string concatenation from `inttostr(...)` and
   `QuotedStr(...)`.** Numeric interpolation is safe by construction (`inttostr` of an `integer`),
   but `QuotedStr` on operator text is not, and one parameter — `IDList` in
   `Get_NewSanad_DateID` — is a **raw comma-separated string spliced into an `IN (…)` clause**
   (§4.4.3). That is the only genuine injection surface in the data module.
2. **Multi-statement batches are the norm.** A single `TADOQuery.SQL` routinely holds
   `DECLARE … SET … IF EXISTS … UPDATE … ELSE INSERT`, executed as one `ExecSQL`. §9.3 explains
   that this is the *only* transaction mechanism in the system — the batch is atomic per statement,
   not per batch, and there is no `BEGIN TRANSACTION` anywhere.
3. **`Q1`, `Q2`, `QS`, `QX` are shared, reused dataset instances on the data module.** Any method
   that calls another method which touches `Q1` clobbers its own cursor. §9.5 covers the
   consequences.

---

### 4.1 Schema-mutating SQL in `Dmu.pas` — the closest thing to DDL in the repository

#### 4.1.1 `TDm.CreateFieldInTable` — idempotent `ALTER TABLE ADD`

```pascal
    UpdateQ.SQL.Add('IF COL_LENGTH('+QuotedStr(TableName)+', '+QuotedStr(FieldName)+') IS NULL ');
    UpdateQ.SQL.Add('ALTER TABLE '+TableName+' ADD ['+FieldName+'] '+FieldType+'  ');
    UpdateQ.ExecSQL;
```
`Dmu.pas:326-328`

**Intent.** A hand-rolled migration primitive: add a column if it does not already exist. This is
the *only* DDL-emitting code in the application. **Every call site is commented out**
(`Dmu.pas:249-270`), so it does nothing at runtime — but the commented calls are the **single best
evidence in the repository of real column types**, because they were written against the live
schema:

```pascal
//     CreateFieldInTable('Dcheck' , 'S_Zssn' , 'int' );
//     CreateFieldInTable('Dcheck' , 'S_ZCR' , 'Varchar(50)' );
//     CreateFieldInTable('Dcheck' , 'S_ZName' , 'Varchar(100)' );
//     CreateFieldInTable('Dcheck2' , 'S_Besssn' , 'int' );
//     CreateFieldInTable('Base' , 'Co_egh' , 'varchar(20)' );
//     CreateFieldInTable('Base' , 'Co_post' , 'varchar(20)' );
//     CreateFieldInTable('Base' , 'Co_address' , 'varchar(100)' );
//     CreateFieldInTable('Base' , 'Co_tel' , 'varchar(20)' );
//     CreateFieldInTable('Base' , 'Co_fax' , 'varchar(20)' );
//     CreateFieldInTable('Base' , 'ARM' , 'Image' );
//     CreateFieldInTable('Sarfasl' , 'FullName' , 'varchar(200)' );
//     CreateFieldInTable('Sarfasl' , 'S_Child' , 'int' );
//     CreateFieldInTable('Anbar_Jens' , 'AJ_Alarm' , 'int' );
//     CreateFieldInTable('Anbar_Jens' , 'AJ_Manfi' , 'int not null default 1 ' );
//     CreateFieldInTable('Anbar_FactorD' , 'AFD_Vahed2' , 'varchar(20)' );
//     CreateFieldInTable('Anbar_FactorD' , 'AFD_Vahed3' , 'varchar(20)' );
//     CreateFieldInTable('Anbar_Jens' , 'AJ_Vahed2' , 'varchar(20)' );
//     CreateFieldInTable('Anbar_Jens' , 'AJ_Vahed3' , 'varchar(20)' );
```
`Dmu.pas:249-270`

**Findings.**

- `Anbar_Jens.AJ_Manfi` is declared `int not null default 1` — i.e. **negative stock is allowed by
  default**, which §11.6 currently models the other way round (`allow_negative_stock boolean NOT
  NULL DEFAULT false`). **Flip the default in §11.6, or confirm against live data (§12.5).** This
  is a concrete correction the ad-hoc inventory produced that §2 did not have.
- `DCheck.S_Zssn`/`S_ZCR`/`S_ZName` (the dead endorsee columns, §2.2/§11.5) were **added by this
  code**, which is why they exist yet are never read.
- `Sarfasl.FullName varchar(200)` and `S_Child int` confirm the types asserted in §2.5.
- `DCheck2.S_Besssn int` — **settles §12.5**: the column is `int`, not `varchar`, despite being
  declared as a `TStringField` on the history grid (`CheckListDU.pas:107`). The grid declaration is
  the error, not the schema.

**Rebuild.** Replaced entirely by versioned migrations (`sqlx migrate` / `refinery`). No
application code ever emits DDL.

#### 4.1.2 `TDM.Update_Sarfasl_Child` — recompute the denormalised child counter

```pascal
     UpdateQ.SQL.Add('Update sarfasl Set S_Child = ( Select Count(*) From sarfasl As D ');
     UpdateQ.SQL.Add('  Where sarfasl.S_Ko=D.S_Ko and D.S_Mo>0 and S_Ta1=0) Where S_Ko>0 and S_Mo=0 ');

     UpdateQ.SQL.Add('Update sarfasl Set S_Child = ( Select Count(*) From sarfasl As D ');
     UpdateQ.SQL.Add('  Where sarfasl.S_Ko=D.S_Ko and Sarfasl.S_MO=D.S_Mo and D.S_Ta1>0 and S_Ta2=0)');
     UpdateQ.SQL.Add('   Where S_Ko>0 and S_Mo>0 and S_Ta1=0 ');

     UpdateQ.SQL.Add('Update sarfasl Set S_Child = ( Select Count(*) From sarfasl As D ');
     UpdateQ.SQL.Add('  Where sarfasl.S_Ko=D.S_Ko and Sarfasl.S_MO=D.S_Mo and Sarfasl.S_Ta1=D.S_Ta1 and D.S_Ta2>0 )');
     UpdateQ.SQL.Add('   Where S_Ko>0 and S_Mo>0 and S_Ta1>0 and S_Ta2=0 ');

     UpdateQ.SQL.Add('Update sarfasl Set S_Child = 0 where S_Ta2>0 ');
     UpdateQ.ExecSQL;
```
`Dmu.pas:303-316`

**Intent.** Rebuild `Sarfasl.S_Child` (§2.5) for the whole table in four passes, one per level:
Kol counts its Moeins, Moein counts its Tafsil1s, Tafsil1 counts its Tafsil2s, and every Tafsil2 is
forced to `0` (a leaf).

**Findings.**

- **The four `UPDATE`s are one batch with no transaction** (§9.3). A failure after the second pass
  leaves the hierarchy's leaf flags inconsistent, and `is_Sarfasl_Last_Deep` (`Dmu.pas:920`) then
  gives wrong answers for the affected subtree.
- It is a **full-table rewrite** with a correlated subquery per row — O(n²) without an index on
  `(S_Ko, S_Mo, S_Ta1, S_Ta2)`, which §12.6 has not yet confirmed exists.
- Note the **table name is lower-cased** (`sarfasl`) here and mixed-cased elsewhere — harmless only
  under a case-insensitive collation (§12.8).
- The server-side `Active_Set` procedure (§3.1) appears to do the same job; **which of the two is
  authoritative is unresolved** (§12.3 item 7).

**Rebuild.** Deleted — `child_count` is derived, or maintained by a trigger in the same transaction
as the insert (§13.6).

#### 4.1.3 The disabled `FullName` / `M_L` / `M_R` maintenance

Nine `UPDATE sarfasl SET FullName = …` statements and the two `Make_L`/`Make_R` updates at
`Dmu.pas:274-296` are **entirely commented out**. Already quoted and analysed in **§3.2** (the
UDFs) and **§2.5** (why the columns are stale in production). Not repeated.

The consequence is worth restating once: `Sarfasl.FullName`, `M_L` and `M_R` are **still read** for
display and `ORDER BY` by live screens, but nothing has updated them since the code was disabled.
Every one of them is stale.

---

### 4.2 Connection and startup SQL

#### 4.2.1 External-catalog probe — `TDM.DataModuleCreate`

```pascal
    Q1.SQL.Add('Declare @Saham varchar(20) Set @Saham=''Saham.Dbo'' ');
    Q1.SQL.Add('Declare @Anbar varchar(20) Set @Anbar=''Anbar.Dbo'' ');
    Q1.SQL.Add('Declare @Basc  varchar(20) Set @Basc=''RPPC_Solution.Dbo'' ');

    Q1.SQL.Add(' if DB_ID(''Saham'') is null Set @Saham='''' ');
    Q1.SQL.Add(' if DB_ID(''Anbar'') is null Set @Anbar='''' ');
    Q1.SQL.Add(' if DB_ID(''Rppc_Solution'') is null Set @Basc='''' ');

    Q1.SQL.Add('   Select @Saham As Saham, @Basc As Basc, @Anbar As Anbar ');
    Q1.Open;
```
`Dmu.pas:765-774`

**Intent.** At startup, ask the server whether the three auxiliary catalogs exist. Each result is
stored in `Anbar_DB` / `Saham_DB` / `Basc_DB` (§1.5) and later **concatenated as a three-part-name
prefix into other queries** — so a missing catalog degrades to an unqualified name rather than
raising.

**Findings.**

- This is the **proof that all three catalogs are expected on the same SQL Server instance** as the
  main database: `DB_ID()` only resolves databases on the local instance. §12.12 asks for
  confirmation; this statement is strong evidence for "yes".
- The fallback is silent: if `Anbar` is missing, `@Anbar` becomes `''` and every query built from it
  targets `dbo.<table>` in the *main* database — which either fails at runtime or, worse, hits a
  same-named local table.
- The prefixes are **spliced into SQL text**, so the catalog names become part of the query string.

**Rebuild.** Configuration, not discovery. Each integration is a declared dependency with its own
connection URL; absence is a startup failure or an explicitly disabled feature flag, never a silent
name change (§8.6 rule 1).

#### 4.2.2 `E_K` and `Password` — the login datasets

```
'Select * From Password'
```
`Dmu.dfm:22` (dataset `E_K`), and the bare `TADOTable` `Password` at `Dmu.dfm:597-603`.

**Intent.** Load the whole user table for the login combo (`GetPassu.pas`). The plaintext
`Password` column comes back with it — see `08-platform-and-security.md` §3.2 and §13.17.

**Finding.** `SELECT *` with no filter. The credential column crosses the wire for **every user**
on every login screen open.

**Rebuild.** `POST /api/v1/auth/login` with a username and a password; the user list is never sent
to an unauthenticated client.

#### 4.2.3 `Base_Q` / `UpdateQ` — the fiscal-year picker

```
'Select *,  LTrim(RTrim(Co_Name)) + '' = '' + LTrim(RTrim(Co_Sub)) As CO_DESC '
'From Base'
'Order By Co_ID'
```
`Dmu.dfm:367-372` (`Base_Q`) — **and the identical text again** at `Dmu.dfm:879-884` (`UpdateQ`).

**Intent.** Populate the year-selection dialog with `"<company name> = <system name>"` per fiscal
year (§1.4).

**Findings.**

- **`UpdateQ` carries this SQL as its design-time text but is used at runtime for the `ALTER
  TABLE` / `S_Child` batches of §4.1** — its `.SQL` is cleared and rebuilt before every use. The
  design-time text is vestigial and misleading.
- The concatenation is the source of the `CO_DESC` label described in §2.3.

**Rebuild.** `GET /api/v1/fiscal-years`; the label is composed in the frontend.

---


---

[← 02-03-c-stored-procedures-functions-and-summary.md](02-03-c-stored-procedures-functions-and-summary.md) | [02-04-b-adhoc-sql-accounting-and-lookups.md →](02-04-b-adhoc-sql-accounting-and-lookups.md)
