_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 1. Connection and runtime topology

### 1.1 The three data stores

| # | Legacy object | Type | Purpose | Proposed replacement |
|---|---|---|---|---|
| 1 | `DM.Ado` (`TADOConnection`) | MS SQL Server, OLE DB `SQLOLEDB.1` | **The** application database. Everything accounting / inventory / treasury / party / security lives here. | PostgreSQL, single database, connection pool in the Rust backend |
| 2 | `DM.ADO_RPPCSOLUTION` (`TADOConnection`) | MS SQL Server on a *different* host | Read/write access to an external legacy system, catalog `Rppc_Solution` on host `Pesteh`. Used only by the pistachio-receipt serial lookup. | External integration / import job; **not** part of the core schema |
| 3 | Absolute Database (`ABSMain`, `*.abs`) | Embedded single-file DB | Two uses only: (a) `Backup_U.pas` backup/restore container, (b) `AnbarFactorU.pas` in-memory scratch table for invoice line editing. **Not** a persistent store of business data. | (a) replaced by `pg_dump`/logical backup; (b) replaced by client-side React state |

The `Dmu.pas` unit declares the ABS unit in `uses` (`Dmu.pas:7`) but its ABS connection object is
commented out — see `Dmu.pas:784-789`:

```pascal
// open ABADO
//   AbsAdo.Close;
//   S:= ChangeFileExt(Paramstr(0) , '.ABS' ) ;
//   AbsAdo.DatabaseFileName :=  S;
//   ABSADO.DatabaseName := 'ABS' ;
//   ABSADO.Open;
```

So at runtime **there is no application-level ABS database** in the data module. ABS survives only
in the backup unit and one form-local scratch table (`AnbarFactorU.pas:38` — `CDS2: TABSTable`).

### 1.2 Connection strings

The design-time connection string baked into the `.dfm` is a developer machine's:

`Dmu.dfm:6-16`
```
object Ado: TADOConnection
  ConnectionString =
    'Provider=SQLOLEDB.1;Integrated Security=SSPI;Persist Security Info=False;'+
    'User ID=sa;Initial Catalog=Arzi89;Data Source=MOHSEN-RANJBAR\SQLEXPRESS;'+
    'Use Procedure for Prepare=1;Auto Translate=True;Packet Size=4096;'+
    'Workstation ID=RANJBAR;Use Encryption for Data=False;'+
    'Tag with column collation when possible=False;'
  LoginPrompt = False
  Provider = 'SQLOLEDB.1'
end
```

`Dmu.dfm:1125-1136`
```
object ADO_RPPCSOLUTION: TADOConnection
  ConnectionString =
    'Provider=SQLOLEDB.1;Persist Security Info=False;User ID=sa;'+
    'Initial Catalog=Rppc_Solution;Data Source=Pesteh;...'
end
```

Notes:
- Default catalog name is **`Arzi89`** — "arzi, year 89 (1389 Jalali)". The catalog is a plain
  database name, not per-year: see §1.4.
- `User ID=sa` with `Integrated Security=SSPI` simultaneously present is contradictory but harmless;
  SSPI wins for the primary connection, SQL auth for `Rppc_Solution`.
- `LoginPrompt = False` — no interactive credential prompt; credentials must come from the string.
- Three additional datasets carry their own **hard-copied** connection strings instead of using the
  `Connection` property: `SahamdarConfig` (`Dmu.dfm:8607-8613`), `Jari_Rem` (`Dmu.dfm:8640-8646`).
  These are stale duplicates of the design-time string and are overwritten at runtime for
  `Jari_Rem` (`Dmu.pas:751-752`) but **not** for `SahamdarConfig` — a latent bug: `SahamdarConfig`
  will try to reach `MOHSEN-RANJBAR\SQLEXPRESS`. Callers work around it by assigning
  `ConnectionString` themselves before use.

### 1.3 Runtime connection-string resolution and obfuscation

`Dmu.pas:707-740` (`TDM.DataModuleCreate`) is the whole bootstrap:

1. The settings file path is computed as the executable's name with extension `.ini`, **relocated to
   the hard-coded directory `D:\BACKUP`** (`Dmu.pas:711`):
   ```pascal
   S:= ChangeFilePath( ChangeFileExt( ParamStr(0) , '.ini' ), 'D:\BACKUP') ;
   PS.FileName := S;
   ```
   For `arzi.exe` this yields `D:\BACKUP\arzi.ini`. The repo copies (`arzi.ini1`, `arzi.local.ini`)
   are captured examples of that file.
2. Vendor branding is written unconditionally on every start (`Dmu.pas:715-719`).
3. `GridFontSize` is read (`Dmu.pas:720`, default `8`).
4. `CS1` is read as a plain string (`Dmu.pas:724`). **`CS1` is a flag, not a key.**
   - If `CS1 = '1'`: `CS2` is read via `MyIni.ReadEncriptString` and assigned directly as the
     connection string (`Dmu.pas:725-728`).
   - Otherwise: `EditConnectionString(Ado)` opens the standard Microsoft OLE DB connection editor
     dialog so the operator types the server/catalog by hand (`Dmu.pas:730-731`).
5. On successful connect, the resolved string is written **back** obfuscated and `CS1` set to `'1'`
   (`Dmu.pas:735-740`), so the first successful manual configuration is remembered.

**`CS2` is the SQL Server connection string, including the `sa` password, not a licence key.**
Confirmed: it is written from `Ado.ConnectionString` at `Dmu.pas:737` and read back into
`Ado.ConnectionString` at `Dmu.pas:727`.

The "encryption" (`INI.pas:43-170`) is the classic Borland `Encrypt`/`Decrypt` pair:
- `InternalEncrypt` (`INI.pas:43-58`) is the LCG stream cipher with constants `C1 = 52845`,
  `C2 = 22719` and a 16-bit seed.
- The seed defaults to the **compile-time constant `Key: Word = 53269`** (`INI.pas:15-16`) and no
  caller ever overrides it (`INI.pas:163-170`, `182`, `193`, `206`, `221`).
- The ciphertext is then Base64-ish encoded by `PostProcess`/`Encode` with the standard
  alphabet `A–Za–z0–9+/` (`INI.pas:59-90`), decoded by `PreProcess`/`Decode` (`INI.pas:91-145`).

This is **obfuscation, not encryption** — the key is a literal in the binary. Every deployment that
ever ran shares it. Treat every `CS2` value in the field as a plaintext credential leak.

Other ini keys observed (`arzi.ini1`, `arzi.local.ini`):

| Key | Meaning | Evidence |
|---|---|---|
| `Base/CS1` | "connection string is configured" flag; `'1'` = use `CS2` | `Dmu.pas:724-732` |
| `Base/CS2` | obfuscated SQL Server connection string | `Dmu.pas:726-737` |
| `Base/CS3` | licence / activation number (integer as text, e.g. `262360341`) | see §8 |
| `Base/CS31` | dead — never read or written by any unit | grep: no `.pas` reference |
| `Base/Program`, `Programer`, `Mobile`, `Contact` | vendor branding, rewritten every launch | `Dmu.pas:715-718` |
| `Base/GridFontSize` | grid font size, default 8 | `Dmu.pas:720` |
| `<FormName>/Left,Top,Width,Height` and per-form extras | window geometry and per-screen UI prefs persisted by `TPropSaveFile` | see §8 |
| `GetPass/ID`, `GetPass/COID` | last logged-in user id and last selected **fiscal year** | see §1.4, §8 |

Note the `arzi.local.ini` file contains a stray malformed line `[` between `CS31` and `[GetPass]`.
`TPropSaveFile` tolerates it. A Rust rewrite must not.

### 1.4 Fiscal-year scoping — `CO_ID` / `COID`

**Confirmed: `CO_ID` is a fiscal-year identifier, not a company/tenant identifier.**

Evidence:
1. `TDM.CO_ID : integer` (`Dmu.pas:113`) is a single global, initialised to `0` at
   `Dmu.pas:743` and set once at login.
2. The `Base` table is looked up **by `CO_ID`** to obtain `FromDate` / `ToDate` — the fiscal period
   bounds (`Dmu.pas:1137-1149`):
   ```pascal
   function TDM.From_Date: String;
   begin
        if not Base.Active then Base.Active := True;
        Base.Locate('CO_ID', inttostr(CO_ID),[locaseinsensitive]);
        Result := Dm.Base.FieldByName('FromDate').AsString;
   end;
   ```
   A tenant identifier would not carry a date range.
3. `Is_New_Sanad_Valid` (`Dmu.pas:997-1015`) rejects posting when
   `Base.IsActive <> 1`, with the Persian message
   `'سال مالی مورد نظر بایگانی شده است'` — *"the requested **fiscal year** has been archived"* —
   and when the row is missing, `'سال مالی پیدا نشد'` — *"**fiscal year** not found"*. The literal
   word used is سال مالی (`sal-e mali`, fiscal year), never شرکت (`sherkat`, company).
4. The observed values in the wild are years: `arzi.ini1` has `COID=1403`, `arzi.local.ini` has
   `COID=1399`; stored-procedure design-time defaults are `1396`, `1397` (`Dmu.dfm:471`, `549`,
   `739`, `790`).
5. The same `Base` row simultaneously carries the operating entity's letterhead identity
   (`Co_Name`, `Co_Sub`, `Co_Egh`, `Co_Post`, `Co_Address`, `Co_Tel`, `Co_Fax`, `ARM` logo image —
   see the disabled DDL at `Dmu.pas:255-260` and `TanzimU.pas:121-143`). So the year row *doubles*
   as the company header, which is why the prefix reads `CO_`. There is exactly one operating
   entity; there is no tenant dimension anywhere in the schema.
6. `Base_Q` (`Dmu.dfm:362-374`) builds the year picker label by concatenating the two name fields:
   ```sql
   Select *,  LTrim(RTrim(Co_Name)) + '  =  ' +  LTrim(RTrim(Co_Sub))   As CO_DESC
   From Base
   Order By Co_ID
   ```

Scoping is applied by **stamping a `*_COID` column on every transactional table** and filtering on
it in every query — `M_COID` on `Moein`, `DM_Coid` on `DMoein`, `AF_COid` on `Anbar_Factor`,
`AFD_Coid` on `Anbar_FactorD`, `S_COID` on `DCheck`/`DFish`, etc. There is **no** database-level
enforcement; if the application forgets the predicate, years bleed together. Examples of the
predicate: `Dmu.pas:828`, `851`, `1247`, `1258`, `1289`, `1318`, `1469`, `1487`.

**Master data is NOT year-scoped.** `Sarfasl` (chart of accounts) and `Sahamdar` (party register)
have no `*_COID` column: every `Sarfasl` query in `Dmu.pas` filters only on `S_Ko/S_Mo/S_Ta1/S_Ta2`
or `S_SSN` and never on the year (`Dmu.pas:929-964`, `1026-1029`, `1044`, `1393`, `1450`), and
`Update_Sarfasl_Child` (`Dmu.pas:300-318`) updates the whole table unscoped. The commented-out
per-year copy in `MakeNewU.pas:129-150` confirms an abandoned attempt to make them year-scoped.

**Proposed model.** A `fiscal_years` table replaces `Base`; every transactional table gets
`fiscal_year_id integer NOT NULL REFERENCES fiscal_years(id)`; the company-header columns move out
of `Base` into a separate single-row `organization` table (or are carried per year, if the team
wants the letterhead to be able to change year over year — see §13).

### 1.5 Auxiliary catalogs — `Anbar_DB`, `Saham_DB`, `Basc_DB`

`Dmu.pas:106` declares three public strings; `Dmu.pas:757-782` resolves them at startup. They are
**cross-database three-part-name prefixes on the same SQL Server instance**, not connections:

```pascal
Saham_DB := 'Saham.Dbo';
Saham_F  := '\\pesteh\SahamData\';
Anbar_DB := 'Anbar.Dbo';
Basc_DB  := 'Rppc_Solution.Dbo';

Q1.SQL.Add('Declare @Saham varchar(20) Set @Saham=''Saham.Dbo'' ');
Q1.SQL.Add('Declare @Anbar varchar(20) Set @Anbar=''Anbar.Dbo'' ');
Q1.SQL.Add('Declare @Basc  varchar(20) Set @Basc=''RPPC_Solution.Dbo'' ');
Q1.SQL.Add(' if DB_ID(''Saham'')         is null Set @Saham='''' ');
Q1.SQL.Add(' if DB_ID(''Anbar'')         is null Set @Anbar='''' ');
Q1.SQL.Add(' if DB_ID(''Rppc_Solution'') is null Set @Basc='''' ');
Q1.SQL.Add(' Select @Saham As Saham, @Basc As Basc, @Anbar As Anbar ');
```

Each is set to the empty string if the catalog does not exist on the server, and **every consumer
guards on `Length(...) = 0`** — so all three are optional site-specific integrations:

| Variable | Catalog | Contents used | Consumers |
|---|---|---|---|
| `Anbar_DB` | `Anbar.dbo` | `Cala` (item master), `Anbar` (warehouses), `FactorMaster`, `FactorDetail` | `FactorPesteh_U.pas:137,194,197,206,213,215`; `Print_Anbar15.pas:138,143,179,190`; `Print_Anbar16.pas:105,110,151,173,194,200`; `SanadViewU.pas:362-363,417-418` (renumber/redate posted invoices when a voucher is renumbered); guards at `AnbarReportU.pas:139`, `Mainu.pas:325`, `SodoorSanadU.pas:325`, `FactorPesteh_U.pas:276` |
| `Saham_DB` | `Saham.dbo` | `NSaham` (share register) | `CardJariU.pas:237,304,308` |
| `Basc_DB` | `Rppc_Solution.dbo` | `NewRamz` (purchase-receipt "password"/voucher records) | `FactorPesteh_U.pas:202,211,218`; also read locally via `KharidPeste_List` (`Dmu.dfm:1114-1121`, `Select * From NewRamz`) |
| `Saham_F` | UNC path `\\pesteh\SahamData\` | scanned images: `<card>\certificate_id.jpeg`, `<card>\<card>_KartMelli.JPG` | `CardJariU.pas:329-331`; blanked when `Saham_DB` is empty (`Dmu.pas:780`) |

Note the *same* catalog `Rppc_Solution` is reachable two ways: as `Basc_DB` on the primary
connection (three-part name, same server), and via `ADO_RPPCSOLUTION` pointed at host `Pesteh`.
Whether those are the same physical server depends on deployment — an open question (§12).

**Proposed model.** In PostgreSQL these become **schemas** in the same database
(`inventory`, `shares`, `external_rppc`) or, better, are dropped entirely: the `Anbar.*` tables are
the *external* pistachio-processing system's invoices, not this application's own inventory (which
lives in `Anbar_Jens` / `Anbar_Factor` / `Anbar_FactorD` in the main catalog). Do **not** confuse
`Anbar_DB.FactorMaster` with the main-catalog `Anbar_Factor` — they are different tables in
different systems.

### 1.6 Dataset topology inside `TDM`

`Dmu.dfm` declares 56 top-level components. Stripping the two image lists, the three FastReport
exporters, the grid sorter and the data source, the persistence surface is:

- **1 connection to the app DB** (`Ado`) + **1 to the external system** (`ADO_RPPCSOLUTION`).
- **11 `TADOTable`** direct table handles: `Base`, `Moein`, `Sarfasl`, `Kind_Table`(→`Kinds`),
  `AnbarConfig`(→`Anbar_Config`), `AnbarJens`(→`Anbar_Jens`), `AnbarFactor`(→`Anbar_Factor`),
  `AnbarFactorD`(→`Anbar_FactorD`), `Sahamdar`, `Password`, `Tanzim`, `TCheck`, `DCheck`, `DFish`.
- **14 `TADOStoredProc`** — see §3.
- **~17 `TADOQuery`**, of which `Q1`, `Q2`, `QS`, `QS1`, `Q1ADO`, `UpdateQ` are **scratch query
  objects with no design-time SQL**: every caller clears `SQL`, appends a freshly concatenated
  statement and executes. This is the source of both the ad-hoc SQL inventory in §4 and the
  SQL-injection surface noted in §13.
- **`PS: TPropSaveFile`** — the settings persistence component (`Dmu.dfm:1171-1175`, design-time
  `FileName = 'D:\1.ini'`, overwritten at `Dmu.pas:713`).

Three scratch queries (`Q1`, `Q2`, `QS`) have **no `Connection` assigned at design time**
(`Dmu.dfm:351-355`, `629-634`, `1166-1170`); callers assign `ConnectionString` per call, e.g.
`Dmu.pas:1244-1245`. This means each such call may open a *new* physical connection rather than
reuse the pooled one — relevant to §9 (every statement runs in its own implicit transaction).


---

[02-02-a-table-inventory-overview.md →](02-02-a-table-inventory-overview.md)
