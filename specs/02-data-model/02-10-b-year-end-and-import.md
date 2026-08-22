_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

### 10.5 Year-end close and carry-forward — `EnteghalU.pas` (`انتقال` = *transfer*)

The real year-end routine. It generates a **closing voucher** (`سند اختتامیه`) in the old year and
an **opening voucher** (`سند افتتاحیه`) in the new one.

**Preconditions, checked in order** (`EnteghalU.pas:85-220`) — every one aborts with a Persian
message:

| # | Check | SQL / logic | Message | English |
|---|---|---|---|---|
| 1 | the **next** fiscal year must already exist | `Select * From Base Where co_id=<CO_ID+1>` (`:93-95`) | `سال مالی آینده را ایجاد کنید` | "create the next fiscal year" |
| 2 | **every voucher in the current year must be finalised** | `Select * From moein Where M_tx <2 and M_coid=<CO_ID>` must return 0 rows (`:103-105`) | `برای بستن سال مالی جدید باید تمام اسناد قطعی شده باشند` | "to close the fiscal year all vouchers must be finalised" — i.e. `M_Tx = 2` (§9.7) |
| 3 | closing voucher number supplied and **not already used in the old year** | `Select * From moein Where M_Sanad=<Sanad1> and M_coid=<CO_ID>` (`:119-121`) | `سند اختتامیه تکراری است` | "duplicate closing voucher" |
| 4 | opening voucher number supplied and **not already used in the new year** | `Select * From moein Where M_Sanad=<Sanad2> and M_coid=<CO_ID+1>` (`:136-138`) | `سند افتتاحیه تکراری است` | "duplicate opening voucher" |
| 5 | closing date within the **old** year's `Base.FromDate…ToDate` | string comparison (`:153-158`) | `تاریخ سند اختتامیه در درنج مجاز نمیباشد` | "the closing-voucher date is out of the permitted range" (`درنج` is a typo for `رنج`/range) |
| 6 | opening date within the **new** year's bounds | `Select * From Base Where co_id=<CO_ID+1>` (`:172-177`) | `تاریخ سند افتتاحیه در درنج مجاز نمیباشد` | "the opening-voucher date is out of the permitted range" |
| 7 | both narrations non-empty | `:183-196` | `شرح بستن حسابهای سال جاری / آینده را وارد کنید` | "enter the narration for closing this / next year's accounts" |
| 8 | the **closing account** (`کد اختتامیه`) chosen via the `Taraf` picker | `:199-207` | `کد اختتامیه را مشخص کنید` | "specify the closing account code" |
| 9 | the **opening account** (`کد افتتاحیه`) chosen | `:210-218` | `کد افتتاحیه را مشخص کنید` | "specify the opening account code" |
| 10 | there must be something left to transfer | the balance query (below) must return rows (`:227-231`) | `حسابها قبلا انتقال پیدا کرده اند` | "the accounts have already been transferred" |

**The balance query** (`EnteghalU.dfm:330-349`, parameter `:COID`) — the definition of what carries
forward:

```sql
Select 123456789 as Code, M_Ko, M_Mo, M_Ta1, M_Ta2
     , Sum(M_Bes-M_Bed) As M_Bes
     , Sum(M_Bed-M_Bes) As M_Bed
  into #R
  From Moein
  Where M_Coid=:COID and M_kind=1
  Group By M_Ko, M_Mo, M_Ta1, M_Ta2
  Order By M_Ko, M_Mo, M_Ta1, M_Ta2

Update #R Set M_Bed=0 Where M_Bed <0
Update #R Set M_Bes=0 Where M_Bes <0
Update #R Set Code = ( Select S_SSN from Sarfasl Where S_Ko=M_Ko and S_Mo=M_mo and S_Ta1=M_ta1 and S_Ta2=M_Ta2 )
Delete #R Where M_Bed=0 and M_Bes = 0

Select * From #R Order By M_ko, M_mo, M_ta1, M_ta2
```

Semantics: net balance per **full 4-segment account code**, restricted to `M_kind = 1` (ordinary
ledger lines); the two `Sum` expressions are mirror images, the negative one is zeroed, so exactly
one of `M_Bed`/`M_Bes` survives per account and it is the **net** balance on its natural side.
Zero-balance accounts are dropped. `Code` is then resolved to the `Sarfasl.S_SSN`; the placeholder
`123456789` remains if the account code has no `Sarfasl` row — a **silent data-integrity hole**
that would insert `M_Code = 123456789`.

**The generated postings.** For each surviving account, one batch of four `INSERT`s wrapped in
`Begin Transaction … Commit` (`EnteghalU.pas:249-276`):

| # | Year | Voucher | Account | Debit | Credit | Narration | Comment in source |
|---|---|---|---|---|---|---|---|
| 1 | `CO_ID` | `Sanad1` (closing) | the account itself | `M_Bes` | `M_Bed` | `Desc1` | `// بستن کد` — "close the code" |
| 2 | `CO_ID` | `Sanad1` | the **closing account** `_Code1` | `M_Bed` | `M_Bes` | `Desc1 + ' کد <code>'` | `// بستن کد به اختتامیه` — "close the code to the closing account" |
| 3 | `CO_ID + 1` | `Sanad2` (opening) | the account itself | `M_Bed` | `M_Bes` | `Desc2` | `// ایجاد کد` — "create the code" |
| 4 | `CO_ID + 1` | `Sanad2` | the **opening account** `_Code2` | `M_Bes` | `M_Bed` | `Desc2 + ' کد <code>'` | `// بستن کد به افتتاحیه` — "close the code to the opening account" |

i.e. rows 1–2 zero every account against the closing account in the old year; rows 3–4 re-establish
every balance against the opening account in the new year. Standard practice.

`_Code1` / `_Code2` are built as a **five-value CSV fragment** spliced into the `VALUES` list
(`EnteghalU.pas:206-207`, `:217-218`), matching the columns `M_Code, M_Ko, M_Mo, M_Ta1, M_Ta2`:

```pascal
_Code1 := inttostr(Taraf.Get_SSn) + ',' + Taraf.EKo.Text+',' + Taraf.EMo.Text + ',0' +
          Taraf.ETa1.Text+',0' + Taraf.ETa2.Text ;
```

The author's own example in the comment is `'14709, 909,22,0,0 '`. Note the stray `'0'` literals
concatenated before `ETa1`/`ETa2` — harmless only because `'0'+'0' = '00'` parses as 0.

Finally (`EnteghalU.pas:279-285`), the two voucher **headers** are created by temporarily mutating
the global fiscal year:

```pascal
Dm.DMoein_Make(Sanad1.IntValue, date1.Farsi_Date, Desc1.Text );
Dm.CO_ID := Dm.CO_ID + 1;
Dm.DMoein_Make(Sanad2.IntValue, date2.Farsi_Date, Desc2.Text );
Dm.CO_ID := Dm.CO_ID - 1;
```

**Defects worth recording:**

1. **One transaction per account, not one per document.** N accounts ⇒ N independent transactions
   (`EnteghalU.pas:249`/`:276` inside the loop). An abort halfway leaves both vouchers
   half-written and unbalanced, with no rollback and no resume. This is the most dangerous
   instance of §9.4 (F3).
2. **The voucher headers are created *after* all lines**, outside every transaction. A failure
   between the loop and `DMoein_Make` leaves orphan lines with no header (§9.4, F4).
3. **`M_User` is hard-coded to `68`** in all four inserts (`EnteghalU.pas:254`, `:258`, `:264`,
   `:272` — the `..., 0,0,68, <desc>` tail maps to `M_id, M_link, M_User, Article`). Carry-forward
   entries are always attributed to user 68, whoever ran them. `M_ID` (source-module code) is `0`,
   so the provenance of the largest journal of the year is lost.
4. **Mutating the global `Dm.CO_ID` and restoring it** (`:281-283`) is not exception-safe; an error
   inside `DMoein_Make` leaves the session pointed at the wrong fiscal year.
5. Precondition 2 requires `M_Tx = 2` (posted) for **all** lines, but the routine then *inserts* new
   lines with `M_Tx = 0` (draft) into the same closed year — so re-running precondition 2
   immediately afterwards would fail. Closing is a one-way, non-idempotent operation guarded only by
   precondition 10.

### 10.6 Closing-entry export / import — `BastanHesab.pas` → `InFile.pas` → `SanadMoeinu.pas`

A separate, file-mediated path, distinct from §10.5. `بستن حساب` = "closing the accounts".

**Export** — `BastanHesab.pas:36-83`:

1. Run stored procedure **`Moein_All`** with `@Coid = DM.CO_ID` (`BastanHesab.dfm:45-58`,
   `BastanHesab.pas:41-43`).
2. Create **two** files at fixed paths — `D:\Bed.GGS` (debits) and `D:\Bes.GGS` (credits) — using
   `TMyIni`, i.e. they are **ini files with a `.GGS` extension** (`:45-46`).
3. Write a header: `[Base] Program=GreenGold`, `Name=Sanad`, `Size=1` (`:47-52`).
4. Walk the result set; a row with `M_Bed > '0'` (**string** comparison, `:57`) goes to `Bed.GGS`,
   everything else to `Bes.GGS`. Each becomes a section `Line<n>` with keys
   `Kol`, `Moein`, `Taf1`, `Taf2`, `Mab` (`:60-73`).
5. Rewrite `[Base] Size` with the real counts (`:77-78`).
6. Message: `فایل خروجی در مسیر d:\ ساخته شد` — *"the output file was created in `d:\`"*.

**Preconditions:** `D:\` must exist and be writable. Paths are hard-coded — two concurrent users
overwrite each other's export.

**Defect:** the split test `Sp1.FieldByName('M_Bed').AsString > '0'` compares *strings*. `'10'` is
**not** `> '0'`… actually `'10' > '0'` is true (`'1' > '0'`), but `'0'` itself and any value
starting with `'0'` falls to the credit file, and a leading space or `'-'` sorts below `'0'`.
Because `Str2String`-style formatting is not applied here it happens to work for plain integers,
but it is a string test on a `bigint` and must be replaced with `M_Bed > 0`.

**Import selection** — `InFile.pas:86-113` validates the chosen file before accepting it:

```pascal
F:= TMyIni.Create(OD.FileName);
S:=  UpperCase( F.ReadString('Base', 'Program', '' ));
if S<>'GREENGOLD' then  MessageDlg('فایل ورودی اشتباه است', ...)   // "the input file is wrong"
S:=  UpperCase( F.ReadString('Base', 'Name', '' ));
if S<>'SANAD' then      MessageDlg('فایل ورودی اشتباه است', ...)
N:= F.ReadInteger('Base','Size', 0);
```

The operator also supplies a **narration** (`Desc`) and picks **debit or credit** via the radio
buttons `RBed`/`RBes` (`InFile.pas:20-21`, `:81`). `Button1Click` (`:44-51`) refuses unless a file
is chosen, `Size > 0`, and the narration is non-empty.

Note both `MessageDlg` branches call `Close` but **do not `Exit`** (`InFile.pas:96-105`), so a file
failing the first check still runs the second check and still sets `infile.Tag := 1` at `:108`.
A wrong file is accepted.

**Import execution** — `SanadMoeinu.pas:281-337`:

| # | Step | Precondition |
|---|---|---|
| 1 | refuse if the target voucher is not a draft: `if SanadState > 0` → `'سند در حالت تحرير نيست'` (*"the voucher is not in draft state"*) | `:287-291` |
| 2 | run the `InFileF` dialog; abort if `Tag = 0` | `:294-295` |
| 3 | open `dm.Moein` as a live table | `:296-297` |
| 4 | re-read `[Base] Size` and loop `Line1..LineN`, reading `Kol`, `moein`, `taf1`, `taf2`, `mab` | `:299-305` — note the key is read as `'moein'`/`'taf1'`/`'taf2'` (lower case) but written by `BastanHesab.pas:62-64` as `'Moein'`/`'Taf1'`/`'Taf2'`; ini key lookup is case-insensitive in `TIniFile`, so this works — but it is an undocumented dependency on the third-party `TPropSaveFile` preserving that behaviour |
| 5 | for each line `dm.moein.Append` and set `M_Ko/M_Mo/M_Ta1/M_Ta2`, put `mab` into `M_Bed` **or** `M_Bes` per the radio button, set `Article` to the operator's narration, `M_Sanad` from the screen, `M_date` from the screen, `M_User := Dm.userId`, `M_Kind := 1`, `M_Tx := 0`, `M_ID := 0`, `M_Code := 0`, `M_CoID := Dm.CO_ID` | `:306-330` |
| 6 | `Post` per row, then reopen the summary stored procedure | `:331-334` |

**Defects:**

- **`M_Code := 0`** — the account **id** is never resolved from `M_Ko/M_Mo/M_Ta1/M_Ta2`. Every
  imported line has a null account reference, unlike every other write path (compare
  `FactorPesteh_U.pas:229-230`, which back-fills `M_Code` from `Sarfasl`).
- No validation that the imported account codes exist in `Sarfasl`.
- No check that debits equal credits; the operator imports one file at a time, so an unbalanced
  voucher is the *normal* intermediate state.
- No transaction — N rows, N implicit auto-commits (§9.5).
- `mab` is read as a **string** and assigned via `AsString`; a malformed value raises at `Post`.

### 10.7 Flow summary

| Flow | Entry point | Output | Transactional | Restorable | Permission gate |
|---|---|---|---|---|---|
| Daily auto backup | `Mainu.pas:894` → `:393` | `<BackupDir><YYYYMMDD>.bak` (native SQL Server) | n/a | **yes**, manually via SSMS | none — runs for everyone |
| Manual ABS export | `Mainu.pas:372` → `Backup_U.pas:128` | `<BackupDir><YYYYMMDD>.ABS` | no | **no** | key `1110` (`Mainu.pas:947`) |
| New fiscal year | `MakeNewU.pas:97` | one `Base` row | no | n/a | see `docs/08-platform-and-security.md` |
| Year-end carry-forward | `EnteghalU.pas:85` | closing + opening vouchers | per-account only | no | — |
| Closing-entry export | `BastanHesab.pas:36` | `D:\Bed.GGS`, `D:\Bes.GGS` | n/a | n/a | — |
| Voucher import | `SanadMoeinu.pas:281` | `Moein` rows | no | no | — |

### 10.8 Proposed model

| Legacy flow | Replacement |
|---|---|
| `BACKUP DATABASE … TO DISK` triggered from a client at login | **operator-owned, out-of-band**: a scheduled `pg_dump`/`pg_basebackup` sidecar container plus continuous WAL archiving to object storage. The application performs **no** backups and has no `BackupDir` setting. Add restore-drill documentation and an automated weekly restore test. |
| `.abs` export | **dropped.** For "give me the data" needs, offer authenticated CSV/XLSX export per table through the API, and `pg_dump --data-only` for full extracts. Nothing in the product writes a proprietary container. |
| Backup encryption via the constant `'Mohsen68411211'` | dumps encrypted at rest by the storage layer, keys in the platform's secret manager. |
| No restore | documented `pg_restore` runbook + PITR; recovery objective (RPO/RTO) agreed with the business. |
| New fiscal year = copy one `Base` row | `POST /api/v1/fiscal-years` inserting into `fiscal_years` (`year`, `start_date`, `end_date`, `is_active = true`) inside one transaction, with `UNIQUE (year)`, `CHECK (end_date > start_date)` and an `EXCLUDE USING gist (daterange(start_date, end_date) WITH &&)` constraint so periods cannot overlap. Letterhead and code widths move out of the year row (§8.6), so nothing is copied. |
| `IsActive` set by no code | an explicit `POST /api/v1/fiscal-years/{id}/archive` endpoint, permission-gated and audited. |
| Year-end carry-forward, one transaction per account | **one transaction for the whole close.** Compute the balances with a single `INSERT … SELECT` (the `#R` query becomes a CTE), generate all four line sets in one statement per voucher, create both headers first, and commit once. Run under `SERIALIZABLE` (§9.10). Make it **idempotent**: record `fiscal_years.closed_at` / `closing_voucher_id` / `opening_voucher_id` and refuse to run twice. |
| Placeholder `Code = 123456789` for unmatched accounts | a hard failure listing the offending account codes before anything is written. |
| Hard-coded `M_User = 68` | `created_by` = the authenticated user, always. |
| `D:\Bed.GGS` / `D:\Bes.GGS` ini files | if the file exchange is still needed, a versioned JSON or CSV download/upload through the API with a declared schema, checksum, and a server-side dry-run that reports every unresolved account code before committing. |
| Import writing `M_Code = 0` | resolve the account id server-side from the 4-segment code and **reject** the import if any segment does not match an `accounts` row. |
| Import with no balance check | validate `SUM(debit) = SUM(credit)` per voucher before commit, or explicitly mark the voucher `draft` and block confirmation until it balances (mirroring `SanadViewU.pas:298`). |
| Hard-coded `D:\` and `D:\BACKUP` paths | no host paths anywhere; all file exchange goes through the API and object storage. |


---

[← 02-10-a-backup-and-restore.md](02-10-a-backup-and-restore.md) | [02-11-a-ddl-overview-and-extensions.md →](02-11-a-ddl-overview-and-extensions.md)
