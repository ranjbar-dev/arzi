_Part of [02-data-model](../02-data-model.md) — [index](00-index.md)_

## 8. Configuration and settings storage

Settings live in **five** places. There is no single source of truth and no schema for any of them.

| # | Store | Scope | Written by | Contains |
|---|---|---|---|---|
| 1 | `D:\BACKUP\arzi.ini` (via `TPropSaveFile`) | per **workstation** | `TMyIni` → `DM.PS` | connection string, licence, window geometry, per-screen UI prefs, last user + last fiscal year |
| 2 | `Tanzim` table | **global** (whole DB) | `TDM.Set_paramstr` | print/report layout parameters, IDs 1001–1015 |
| 3 | `Base` table | per **fiscal year** | `TanzimU.pas`, `MakeNewU.pas` | organisation letterhead, fiscal period bounds, account-code widths, backup directory, the two "system account" pointers |
| 4 | `Anbar_Config` table | per **warehouse** | `AnbarTanzimU.pas` | default posting accounts and VAT rate for inventory |
| 5 | Windows registry `HKLM\Software\<PrgName>` and `HKLM\HARDWARE\DESCRIPTION\System` | per **machine** | never (read-only in practice) | licence fingerprint inputs |

Cross-references: §1.3 covers connection-string resolution and the obfuscation scheme; §5 covers
`Base.NO_Ko/NO_Mo/NO_Ta1/NO_Ta2`; `docs/08-platform-and-security.md` covers `Password` and
`Pass_Config` (permissions) — those are **not** repeated here.

### 8.1 Store 1 — the ini file

#### 8.1.1 Mechanics

Two layers, and the outer one is a **third-party component whose source is not in this repository**:

- `PS : TPropSaveFile` (`Dmu.dfm:1171-1175`) — the actual file reader/writer. Design-time
  `FileName = 'D:\1.ini'`, overwritten at `Dmu.pas:713` with
  `ChangeFilePath(ChangeFileExt(ParamStr(0), '.ini'), 'D:\BACKUP')` → **`D:\BACKUP\arzi.ini`**.
  The directory is **hard-coded**; there is no fallback if `D:` does not exist.
- `MyINI : TMyIni` (`INI.pas:7-33`) — a thin façade that adds the obfuscated `*Encript*` variants
  and delegates everything else:

```pascal
procedure TMyIni.WriteString(Const Section:String; Const Ident:String; Const Value :String );
begin
   Dm.PS.WriteString(Section, Ident, Value );
   DM.PS.SaveFile( DM.PS.FileName );        // INI.pas:171 — rewrites the WHOLE file on every write
end;
```

Consequences:

1. **Every single setting write rewrites the entire file** (`INI.pas:168-172`). A form close that
   persists `Left`, `Top`, `Width`, `Height` performs **four** full-file writes.
2. If `D:\BACKUP` is a **shared** network path (which the name and the `Base.BackupDir` setting
   suggest), two workstations closing forms simultaneously will clobber each other's geometry, and
   a partially written file can lose `CS2` — bricking the installation until the connection string
   is re-entered.
3. `TMyIni.Create` (`INI.pas:238-250`) computes a fallback path
   (`ChangeFileExt(ParamStr(0),'X.ini')`, then `'D:\Backup\Hesab.ini'`) and stores it in the private
   `IniFile` field — which is **never read anywhere**. The two lines that would have used it are
   commented out (`INI.pas:247-248`). Dead code; the real path always comes from `Dmu.pas:713`.

Typed accessors (`INI.pas:199-232`): `WriteInteger`/`ReadInteger` are `Int64` and go through
`StrToInt64` **with no exception guard** — a corrupted numeric value raises `EConvertError` at
startup. `WriteBool`/`ReadBool` store `'0'`/`'1'`.

The obfuscated variants (`WriteEncriptString`, `ReadEncriptString`, `WriteEncriptInteger`,
`ReadEncriptInteger`, `INI.pas:174-232`) wrap the value in `Encrypt`/`Decrypt` with the default key
`53269` — see §1.3. `ReadEncriptString` returns the **default** when the key is absent
(`INI.pas:186-191`), so a missing `CS2` silently yields an empty connection string rather than an
error.

The repo copies `arzi.ini1` and `arzi.local.ini` are captured examples of this file.
`arzi.local.ini` contains a stray malformed line consisting of a single `[` between `CS31` and
`[GetPass]`; `TPropSaveFile` tolerates it (§1.3).

#### 8.1.2 Complete ini key inventory

**`[Base]` — application-level**

| Key | Type | Default | Encrypted | Effect | Evidence |
|---|---|---|---|---|---|
| `Program` | string | `'Green Gold'` | no | vendor branding; **rewritten unconditionally on every launch** | `Dmu.pas:715` |
| `Programer` | string | `'Mohsen Ranjbar'` | no | vendor branding, rewritten every launch | `Dmu.pas:716` |
| `Mobile` | string | `'09131912805'` | no | vendor contact, rewritten every launch | `Dmu.pas:717` |
| `Contact` | string | `'MohsenRanjbar.1350@Gmail.com'` | no | vendor contact, rewritten every launch. **Note:** the code writes `Contact`, but `arzi.ini1` contains `Contct=` — an older misspelling that is never cleaned up, so both keys accumulate. | `Dmu.pas:718` vs `arzi.ini1:5` |
| `GridFontSize` | integer | `8` | no | font size for every data grid in the application | `Dmu.pas:720` |
| `CS1` | string flag | `''` | no | `'1'` ⇒ read the connection string from `CS2`; anything else ⇒ show the OLE DB connection editor. Set to `'1'` after the first successful connect. | `Dmu.pas:724-732`, `735-736` |
| `CS2` | string | `''` | **yes** | the **SQL Server connection string including the `sa` password** (§1.3). Not a licence key. | `Dmu.pas:726-727`, `737` |
| `CS3` | integer-as-text | — | no | licence / activation number, e.g. `262360341`. Validated against the machine fingerprint (`Mainu.pas:879`, §9.1). | `arzi.ini1:7`, `arzi.local.ini` |
| `CS31` | integer-as-text | — | no | **dead** — present in `arzi.local.ini` (`108763797`) but never read or written by any unit | grep: no `.pas` reference |

**`[GetPass]` — the login dialog, doubling as session persistence**

| Key | Type | Effect |
|---|---|---|
| `ID` | integer | last successfully logged-in **user id** — pre-selects the user on the next launch |
| `COID` | integer | last selected **fiscal year** (`1403` in `arzi.ini1`, `1399` in `arzi.local.ini`) — pre-selects the year |
| `Left`,`Top`,`Width`,`Height` | integer | window geometry |

**`[<FormName>]` — one section per form, ~40 sections observed**

The pattern is uniform: `init`/`FormActivate` reads, `FormClose` writes
(e.g. `TanzimChapu.pas:88-104`, `BankTanzim.pas:49-62`, `MakeNewU.pas:65-89`).

| Key | Type | Count of writers | Effect |
|---|---|---|---|
| `Left`, `Top`, `Width`, `Height` | integer | 130 each | window geometry, per form |
| `G1FontSize` | integer | 23 | per-grid font-size override (grid `G1`) |
| `M_RL` / `MRL` | string/integer | 6 / 2 | sort/display mode for the account tree. `arzi.local.ini` shows `[ListSarfasl] M_RL=1` and `[Sarfasl_Select] MRL=M_R` — **two different spellings holding two different value domains** (a flag vs. a column name) |
| `C1`,`C2`,`C3`,`C4` | integer(bool) | 4 each | `[FactorPrint]` — which of four invoice copies to print |
| `F0`..`F7` | integer | — | `[AnbarFactor]` — grid column widths (pixels) |
| `A4orA5` | integer | 2 | paper-size selection for a report |
| `D1`, `D2` | string | 1 each | `[AnbarReport_F]` — last-used report **date range**, persisted as Jalali strings (`AnbarReportU.pas:132-133`, read back `:176-177`). Note this stores a *business* value in a UI-preferences file. |
| `L1`, `L2`, `L3` | integer | 1 each | layout/label positions on a print form |
| `G1C0`, `G1C1` | integer | 1 each | grid column widths |
| `F` | — | 1 | undetermined single-letter key |

There is **no ini key that changes business behaviour** other than `D1`/`D2` (a default filter) —
everything else is connection, licence, or UI state. That is a useful property for the rebuild: the
ini file does **not** need a data migration, only the connection string and the licence.

### 8.2 Store 2 — the `Tanzim` table (global key/value settings)

`Tanzim` is a `TADOTable` on the main connection (`Dmu.dfm:744-750`, `TableName = 'Tanzim'`).
Schema, inferred from every field reference:

| Column | Type (inferred) | Meaning | Proposed |
|---|---|---|---|
| `T_ID` | `int` (logical PK) | the setting's numeric key | `settings.key` |
| `T_Str` | `varchar` | the setting's **value** (always a string, even for booleans) | `settings.value` |
| `T_Int` | `int` (written as the string `'0'`) | unused — set to `0` on creation and never read | drop |
| `T_Desc` | `varchar` | Persian label, seeded identically to the initial `T_Str` | `settings.label_fa` |

Accessors (`Dmu.pas:468-508`):

```pascal
function TDM.Get_paramstr(parameter_Id: integer ): string;
begin
    s := '';
    if parameter_Id = 1001 then  S:= 'فاکتور امضا 1';
    ... one `if` per id ...
    if tanzim.Active= false then tanzim.Open;
    if not tanzim.Locate('T_ID', parameter_Id, [locaseinsensitive]) then
    begin
      tanzim.Append;                                   // lazily create the row on first read
      tanzim.FieldByName('T_ID').AsInteger := parameter_Id;
      Tanzim.FieldByName('T_Str').AsString := S;       // the LABEL becomes the initial VALUE
      Tanzim.FieldByName('T_Int').AsString := '0';
      Tanzim.FieldByName('T_Desc').AsString := S;
      tanzim.Post;
    end;
    Result := Tanzim.FieldByName('T_Str').AsString;
end;

procedure TDM.Set_paramstr(parameter_Id: integer; Parameter_Str: String);
begin
    if tanzim.Active= false then tanzim.Open;
    if Not tanzim.Locate('T_ID', parameter_Id, [locaseinsensitive]) then exit;   // silent no-op
      tanzim.edit;
      Tanzim.FieldByName('T_Str').AsString := Parameter_Str;
      tanzim.Post;
end;
```

Three defects to note:

1. **`Get_paramstr` mutates on read.** A missing setting is *created* by a getter, with the Persian
   label as its value. So an un-configured invoice signature line prints as the literal text
   `فاکتور امضا 1` ("Invoice signature 1") on real customer documents.
2. **`Set_paramstr` is a silent no-op if the row does not exist** (`Dmu.pas:504`). A save can appear
   to succeed and change nothing — unless a `Get_paramstr` happened to create the row first.
   The screens do call the getters in `init` (`TanzimChapu.pas:105-121`), which is what makes it
   work in practice. That ordering dependency is invisible and fragile.
3. The label→value seeding means `T_Desc` and `T_Str` are indistinguishable until first edit.

**Complete setting inventory (all IDs, from `Dmu.pas:472-486`):**

| `T_ID` | Persian default | English | Type | Set from | Read by |
|---|---|---|---|---|---|
| 1001 | فاکتور امضا 1 | Invoice signature line 1 | string | `TanzimChapu.pas:71` | `TanzimChapu.pas:105`, invoice reports |
| 1002 | فاکتور امضا 2 | Invoice signature line 2 | string | `:72` | `:106` |
| 1003 | فاکتور امضا 3 | Invoice signature line 3 | string | `:73` | `:107` |
| 1004 | فاکتور امضا 4 | Invoice signature line 4 | string | `:74` | `:108` |
| 1005 | فاکتور عنوان 1 | Invoice heading line 1 | string | `:75` | `:109` |
| 1006 | فاکتور عنوان 2 | Invoice heading line 2 | string | `:76` | `:110` |
| 1007 | طرف حساب | Counterparty (label/caption on the invoice) | string | `:77` | `:111` |
| 1008 | نمایش مبلغ | **Show amount** column on the printed invoice | bool as `'0'`/`'1'` | `:83` | `:113` |
| 1009 | نمایش تخفیف | **Show discount** column | bool as `'0'`/`'1'` | `:84` | `:114` |
| 1010 | نمایش مالیات | **Show tax** column | bool as `'0'`/`'1'` | `:85` | `:115` |
| 1011 | سند امضا 1 | Voucher signature line 1 | string | `:78` | `:117` |
| 1012 | سند امضا 2 | Voucher signature line 2 | string | `:79` | `:118` |
| 1013 | سند امضا 3 | Voucher signature line 3 | string | `:80` | `:119` |
| 1014 | سند امضا 4 | Voucher signature line 4 | string | `:81` | `:120` |
| 1015 | پانویس فاکتور رسمی | Footer text for the **official** (tax) invoice | multi-line string (`TMemo`) | `:82` | `:121`, `Factorprint2U.pas:129` and appended to the invoice description at `Factorprint2U.pas:102` |

That is the **entire** `Tanzim` inventory — the IDs are a closed set of exactly 1001–1015
(repo-wide grep of `Get_paramstr(`/`Set_paramstr(` literals). All fifteen are presentation
settings; none changes accounting behaviour.

Editing screen: `TanzimChapu.pas` (`TanzimChap` — "print settings", `تنظيم چاپ`). `FactorPrintU.pas:94`
re-opens `Dm.Tanzim` before printing to pick up changes.


---

[← 02-07-money-and-amount-handling.md](02-07-money-and-amount-handling.md) | [02-08-b-configuration-base-and-registry.md →](02-08-b-configuration-base-and-registry.md)
