_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 4. Card Jari (running account statement)

**Unit:** `CardJariU.pas` / `CardJariU.dfm` (452 / 6 551 lines — the `.dfm` is large because of an
embedded design-time bitmap on `S_Aks`, not because of report layouts).
**Form caption:** `      فرم خلاصه اطلاعات جاری اشخاص...` — "summary form of persons' current-account
information" (`CardJariU.dfm:5`).
**Launched from** `Mainu.pas:445-448` (`TMain.B_CardJariClick` → `CardJariF.init`), toolbar button
`B_CardJari` captioned `خلاصه` / `کارت` on two lines (`Mainu.dfm:816-832`). **Reachable.**

### 4.0 What it actually is — correct the name first

Despite "Card Jari" being the Persian term for a running-account statement, **this screen is not a
statement and produces no report.** It is a **party dashboard**: identity block, photo, one summary
line per current-account code the party owns, and two buttons that hand off to the ledger screens of
§3. There is no `TfrxReport`, no `TfrxDBDataset`, no `ShowReport` call anywhere in the unit. The
running-balance statement that users call "the card" is `DMoein` (§3.2) or `TMoein` (§3.3), reached
from here.

Proposed English name: **`party_account_summary`** — not `card_jari`, not `statement`.

### 4.1 Filters and inputs

There is no filter panel and **no date range at all**. The whole screen is scoped to one party and
one fiscal year, and the figures are always inception-to-date for that year.

| Control | Type | Persian caption | Default | Validation | Effect |
|---|---|---|---|---|---|
| `S_Card: TEditInt` | int, `IntLength = 8`, thousands separator `,` (`.dfm:5892-5906`) | `شماره عضویت` — "membership number" (`.dfm:5798`) | `_Jari` argument, else `0` (`:257`) | `if _card <= 0 then Exit` (`:282`) — silent | the party |
| `sSpeedButton1` | `?` picker (`.dfm:5834-5840`) | — | — | `if Sahamdar.Tag = 0 then exit` (`:440`) | opens `Sahamdar.init2`, writes the chosen card back and re-runs (`:437-443`) |
| `COID: TDBLookupComboBox` | fiscal year (`.dfm:5922-5932`) | `سال مالی` (`.dfm:5830`) | `DM.CO_ID`, or the `_Coid` argument (`:263-264`) | none | `M_Coid` on every figure |
| `GridFontSize: TsUpDown` | int | — | INI `G1FontSize`, default 8 (`:252`) | — | grid font |

`init(_Jari: integer = 0; _Coid: integer = 0)` (`:233-269`). Both parameters are always defaulted in
practice — the only call site is `Mainu.pas:447` with no arguments.

`COID.ListSource = DS2` → `Q2` = **`Select * From Base Order By CO_ID`** (`.dfm:6540-6541`). Note this
is the *plain* year list — Card Jari has **no synthetic `CO_ID = 0` "all fiscal periods" row**, unlike
`DKolU`/`DMoein`/`TMoein` (§3.1). It nonetheless passes `COID.KeyValue` straight into those screens
(`:390`, `:429`), so the all-years mode is reachable only by changing the year inside the child form.

**Recalculation triggers.** `S_Card.OnChange = S_CardChange` → `ClearForm` only (`:445-448`) — typing
blanks the screen. `S_Card.OnExit = S_CardExit` → `LoadSahamdar` (`:450-453`) — leaving the field
loads. `COID.OnCloseUp = S_CardChange` (`.dfm:5931`) → **clears but does not reload**; changing the
fiscal year empties the grid and the user must re-focus and leave `S_Card` to get numbers back.
`DS2.OnDataChange = DS2DataChange` (`.dfm:6547`) → also `S_CardChange` (`:192-196`), with
`S_CardExit` commented out on the next line — the same "clear, don't reload" behaviour, fired whenever
the `Base` cursor moves.

### 4.2 The account-set query — `QList`

Which accounts belong to a party is **not** stored per party. It is derived from a template table
`SahamdarConfig` by substituting the party's card number into the first free code segment.

Verbatim (`CardJariU.dfm:6503-6527`, parameter `Jari` at `.dfm:6495-6501`):

```sql
Declare @Jari int Set @Jari=:Jari

   if OBJECT_ID('tempdb..#R') is not null Drop Table #R

   Select  -Min(SC_Rem) As SC_Rem , SC_K, SC_M, SC_T as SC_T1, 0 As SC_T2, 0 as SC_Found
   into #R
   From SahamdarConfig
   Group By SC_K, SC_M, SC_T

   Update #R Set SC_T2=@Jari Where SC_T2=0 and SC_T1>0
   Update #R Set SC_T1=@Jari Where SC_T1=0

   Update #R Set SC_Found = ( Select Count(*) From Sarfasl Where SC_K=S_Ko and SC_M=S_Mo and SC_T1=S_Ta1 and SC_T2=S_Ta2 )
   Delete #R Where SC_found=0

   Select * From #R Order By SC_Rem, SC_K, SC_M, SC_T1, SC_T2
```

Reading:

1. Distinct `(SC_K, SC_M, SC_T)` triples from `SahamdarConfig` — the templates.
2. **Card-number injection:** if the template already carries a Tafsil1 (`SC_T1 > 0`), the card number
   goes into Tafsil2; otherwise it goes into Tafsil1. So a party's accounts are
   `Kol / Moein / <template Tafsil1> / <card>` or `Kol / Moein / <card> / 0`.
3. **Existence filter:** templates that resolve to no `Sarfasl` row are dropped. This is why a party
   with no accounts yields an empty grid and no error.
4. **Sort:** `SC_Rem` is negated in the projection (`-Min(SC_Rem)`) so that ascending
   `Order By SC_Rem` puts `SC_Rem = 1` rows first. `SC_Rem` is the "counts toward the final balance"
   flag (see §4.3); it doubles as the display-order key.

The `Jari` parameter is declared **`ftWideString`, `Size = 4`, design-time value `'1481'`**
(`.dfm:6496-6501`) with `Prepared = True` (`.dfm:6502`), while the assigned value is an integer
(`:364`). Whether ADO re-derives the size on prepare or truncates card numbers of five digits or more
is **not determinable from source** — see §9.

### 4.3 The final balance — `DM.Jari_Rem`

`مانده نهایی` ("final balance", label at `.dfm:5809`) in `S_Rem` comes from a **separate** query on
the data module, not from the grid. `Dm.Jari_Rem` (`Dmu.pas:93`, `Dmu.dfm:8641-8692`), parameters
`Jari` and `Sal`, set at `CardJariU.pas:351-352` from `S_Card.IntValue` and **`COID.KeyValue`** (the
comment `//Dm.CO_ID` records that it used to be the global year).

Verbatim (`Dmu.dfm:8659-8688`):

```sql
Declare @Jari int Set @Jari=:Jari
Declare @Sal int Set @Sal=:Sal

   if OBJECT_ID('tempdb..#R') is not null Drop Table #R

   Select  SC_K, SC_M, SC_T as SC_T1, 0 As SC_T2  ,  Cast(0 as Bigint) As Bed, Cast(0 as Bigint) As Bes,  Cast(0 as Bigint) As Rem
   into #R
   From SahamdarConfig
   Where SC_Rem = 1

   Update #R Set SC_T2=@Jari Where SC_T2=0 and SC_T1>0
   Update #R Set SC_T1=@Jari Where SC_T1=0

   Update #R Set Bes = isnull( ( Select Sum(M_Bes) From Moein Where M_Ko=#R.SC_K and M_Mo=#R.SC_M and M_Ta1=#R.SC_T1 and M_Ta2=#R.SC_T2 and M_Coid=@Sal) , 0)
   Update #R Set Bed = isNull( ( Select Sum(M_Bed) From Moein Where M_Ko=#R.SC_K and M_Mo=#R.SC_M and M_Ta1=#R.SC_T1 and M_Ta2=#R.SC_T2 and M_Coid=@Sal) , 0)
   Update #R Set Rem = Bes-Bed
   Delete #R where Bed=0 and Bes=0

   Select isnull( Sum(Rem) , 0) as Remind From #R
```

Semantics and traps:

- **Only templates flagged `SC_Rem = 1` are included** — the final balance is deliberately a *subset*
  of the grid, so `S_Rem` will not equal the grid footer sums. That is by design, but nothing on the
  screen says so.
- **No `GROUP BY`.** Unlike `QList`, this query does not deduplicate `(SC_K, SC_M, SC_T)`. If
  `SahamdarConfig` holds two rows with the same triple both flagged `SC_Rem = 1`, the amount is
  **counted twice**. `QList` guards against this with `Group By`; `Jari_Rem` does not.
- **No `M_kind` filter.** `M_Kind = 2` journal-summary lines are summed together with `M_Kind = 1`
  detail lines — the same class of bug as §3.3(b). Summary lines sit at `M_Mo = 0` so they only
  collide when a template has `SC_M = 0`, but nothing prevents that.
- **No `M_Tx` filter** — drafts included.
- `Rem = Bes - Bed`, credit-positive, consistent with the ledgers.
- Sign display: `S_Rem.IntValue := _Bed` (`:358`) where `_Bed` actually holds `Remind`; the label
  `T_Rem` shows `بدهکار` ("debtor") **only when the value is negative** (`:360`), otherwise blank —
  so a credit balance is unlabelled. `S_Rem` is `ReadOnly`, `IntSplitter = ','`, `IntLength = 17`
  (`.dfm:5907-5921`).

### 4.4 The per-account rows — `ADD_Code`

`LoadSahamdar` walks the `QList` result and calls `ADD_Code(SC_K, SC_M, SC_T1, SC_T2)` once per row
(`:367-372`). `ADD_Code` (`:124-164`) does **two** round trips per account — so the screen issues
`2 × <number of accounts> + 3` queries per party:

```pascal
// 1) resolve the account
Q1.SQL.Add('Select * From Sarfasl Where S_ko='+_K+' and S_Mo='+_M+' and S_Ta1='+_T1+' and S_Ta2='+_T2);
Q1.Open;
if Q1.RecordCount=0 then Exit;          // silently skips
...
Vt1.FieldValues['S_R']    := Q1.FieldValues['M_R'];
Vt1.FieldValues['S_Name'] := Q1.FieldValues['LineName'];

// 2) year-to-date turnover
Q1.SQL.Add(' Select isnull(Sum(M_Bed),0) As Bed, isnull(Sum(M_Bes),0) As Bes  From moein ');
Q1.SQL.Add('  Where M_ko='+_K+' and M_Mo='+_M+'  and M_Ta1='+_T1+' and M_ta2='+_T2+
           '  and M_COID='+ inttostr(COID.KeyValue) );   //inttostr(DM.CO_ID) );
Q1.Open;

_Bed := Q1.FieldValues['Bed'];  _Bes := Q1.FieldValues['Bes'];
Vt1.FieldValues['G_Bed'] := _Bed;   Vt1.FieldValues['G_Bes'] := _Bes;
Vt1.FieldValues['R_Bed'] := 0;      Vt1.FieldValues['R_Bes'] := 0;
if _Bed>_Bes then Vt1.FieldValues['R_Bed'] := _Bed-_Bes;
if _Bes>_Bed then Vt1.FieldValues['R_Bes'] := _Bes-_Bed;
```
(`CardJariU.pas:127-160`)

- **No `M_kind` filter, no `M_Tx` filter, no date range.** The turnover figures are the whole fiscal
  year, all voucher states, **and both `M_Kind` universes**. Where `SahamdarConfig` yields an account
  with `SC_M = 0` these totals will include journal-summary lines.
- The debit/credit balance pair uses the **same clamped-split convention as the 4-column trial
  balance** (§2.1): exactly one of `R_Bed`/`R_Bes` is non-zero, both zero when the account nets flat.
  Written here as two strict inequalities rather than clamping, but identical in effect.
- `S_R` is taken from **`Sarfasl.M_R`**, and `S_Name` from **`Sarfasl.LineName`**. Per
  `02-data-model.md` §4.1.3, `M_R` maintenance was disabled (the `Update sarfasl set M_R = Dbo.Make_R(…)`
  statement at `Dmu.pas:274-296` is commented out), so **the `کد حساب` column shows a stale or blank
  code** for any account created since. `LineName` is not in `02-data-model.md`'s `Sarfasl` inventory
  at all — see §9.
- **`Vt1.Append` at `:163` is a bug-shaped quirk.** Each `ADD_Code` ends by putting the in-memory table
  into insert mode with an empty buffer. The next call's `Vt1.Append` at `:136` cancels it (nothing
  was modified), but after the *last* account the dataset is left in `dsInsert`, so **the grid shows a
  trailing blank row** and `G1.RecalculateSummaryResults(True)` (`:375`) runs over it.

### 4.5 Output columns

`VT1` is a `TVirtualTable` (`.dfm:6309-6411`) — a purely in-memory dataset. Grid `G1`
(`.dfm:20-116`), `dgMultiSelect` enabled, `FixedColText.ShowCheckbox = True`:

| # | Field | Persian caption | English | Format |
|---|---|---|---|---|
| 1 | `S_R` | `کد حساب` (`.dfm:72`) | `account_code` | `varchar(20)`, from `Sarfasl.M_R` — **stale** |
| 2 | `S_Name` | `نام حساب` (`.dfm:80`) | `account_name` | from `Sarfasl.LineName` |
| 3 | `G_Bed` | `گردش بدهکار` (`.dfm:88`) | `ytd_debit` | `bigint`, `'###,###'` — zero renders empty |
| 4 | `G_Bes` | `گردش بستانکار` (`.dfm:96`) | `ytd_credit` | `bigint`, `'###,###'` |
| 5 | `R_Bed` | `مانده بدهکار` (`.dfm:104`) | `balance_debit` | `bigint`, `'###,###'` |
| 6 | `R_Bes` | `مانده بستانکار` (`.dfm:112`) | `balance_credit` | `bigint`, `'###,###'` |

Hidden fields carried for drill-down only: `S_Ko` (`ftString(20)`), `S_Mo`, `S_Ta1`, `S_Ta2`,
`S_SSN` (`TAutoIncField`, the row ordinal assigned at `:135,137`).

**Footer totals** are grid-level, not report-level (`.dfm:58-63`):

```
FooterRow.FooterVisible = True
FooterRow.FieldFooterDefs.Strings = ('R_Bes=%Sum' 'R_Bed=%Sum' 'G_Bed=%Sum' 'G_Bes=%Sum')
```

so all four amount columns get a `%Sum` footer, recomputed by `G1.RecalculateSummaryResults(True)`
(`:375`). Because `R_Bed`/`R_Bes` are clamped per row, `Σ R_Bed − Σ R_Bes` is the party's true net
position, but neither footer alone means anything. **The footer sums span all accounts; `S_Rem` spans
only `SC_Rem = 1` accounts — the two will normally disagree and there is no note explaining why.**

There is **no grouping, no subtotal level, no sort control, no page break** — a single flat grid,
ordered by `QList`'s `Order By SC_Rem, SC_K, SC_M, SC_T1, SC_T2`.

### 4.6 The identity panel

Two side-by-side group boxes reading two different databases:

- `sGroupBox2` (accounting side) ← `Select * From Sahamdar where S_card=<card>` (`:286`), fields
  `S_Name`, `S_Famil`, `S_Father`, `S_Mobile`, `S_CodeMelli` → boxes captioned
  `نــــــــــام`, `نام خانوادگی`, `نــــام پــدر`, `شماره تماس`, `کـــد ملـــی`
  (`.dfm:5787,5776,5765,5754,5743`). All `ReadOnly = True`.
- `G_Saham` (shareholder side) ← `Select * From <DM.Saham_DB>.NSaham Where N_Card=<card>`
  (`:308-309`) — a **cross-database query** into the separately installed share-register database,
  fields `N_Name`, `N_Famil`, `N_Father`, `N_Mobile`, `N_CodeMelli`. Hidden entirely when
  `DM.Saham_DB` is empty (`:237-244`); the "share system not installed" error that used to fire is
  commented out (`:240-241`).
- **Not-found handling writes into the display fields instead of raising:** `N_Name` is overwritten
  with `جاری در قسمت اشخاص وارد نشده است` ("the current account has not been entered in the persons
  section", `:297`) when the party is missing from `Sahamdar`, and with
  `جاری در برنامه سهام به روز نشده` + `بروزرسانی انجام شود` ("not updated in the share program /
  perform an update", `:321-322`) when missing from `NSaham`. Both `MessageDlg` versions are commented
  out (`:298`, `:323`). Note the *accounting*-side miss writes into the *shareholder*-side box — a
  copy-paste slip that puts the message in the wrong column.
- **Photo:** `S_Aks: TImage` loaded from `DM.Saham_F + <card> + '\certificate_id.jpeg'`, falling back
  to `DM.Saham_F + <card> + '\' + <card> + '_KartMelli.JPG'` (`:329-337`). A raw filesystem path, no
  access control, JPEG only, silently blank if absent. In the rebuild this is object storage behind an
  authorised URL.

### 4.7 Permission gates

Two, applied at different times:

1. `Report1.Enabled := Dm.IsEnabel(Dm.userId, 1123)` in `init` (`:259`). `IsEnabel`
   (`Dmu.pas:1554-1564`) is `Select * From Pass_Config where P_User=<u> and P_ID=<key>` →
   `RecordCount > 0`. So permission key **1123** gates the "مشاهده دفتر" button only; `Report2`
   ("مشاهده تجمیعی") is **ungated** and reaches the same data through `TMoein`. That is a real
   authorisation hole.
2. `Dm.Is_Admin_Or_Valid_Jari(S_Card.IntValue)` inside `LoadSahamdar` (`:342-347`), refusing with
   `مشاهده اطلاعات فقط تو سط مدیر سیستم مجاز است` ("viewing this information is permitted only to the
   system administrator"). The implementation (`Dmu.pas:968-981`) is
   `Select * From Sahamdar Where S_Card=<card>` → `true` if no row, else `S_Lock = 0`. So, as with the
   ledgers (§3.1), the message is wrong: the rule is a per-party lock flag, and an **unknown card is
   permitted**. Critically, this gate sits **after** the identity panel and the photo have already been
   populated (`:291-337`) — a locked party's name, father's name, phone, national ID and photo are all
   on screen before the refusal, and only the *balances* are withheld.


---

[← SS3 General and subsidiary ledgers (3/3)](04-03-c-general-and-subsidiary-ledgers.md) | [Index](00-index.md) | [SS4 Card Jari (2/2) →](04-04-b-card-jari.md)
