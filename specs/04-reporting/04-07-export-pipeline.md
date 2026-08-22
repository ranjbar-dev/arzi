_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 7. Export pipeline

There are **three unrelated export mechanisms**, only two of which are live, and they produce
different things.

| Mechanism | Where | Live? | Output |
|---|---|---|---|
| FastReport export filters on the data module | `Dmu.dfm:8693-8770` | yes, **only via the preview's Export button** | XLS / JPEG / CSV of the *rendered report* |
| Direct Excel OLE automation | `ToExcelDaraeiU`, `MoeinZipU`, `Anbar_Amalkard` | yes | a worksheet built cell by cell from a *result set* |
| `Rp1.Export(DM.XLX)` programmatic call | `ToExcelU.pas:309` | **no — unit is dead** | — |

### 7.1 FastReport export filters (`XLX`, `JPG`, `CSV`)

Three filter components sit on the data module and are never referenced by name in live code
(`grep` finds `DM.XLX` only in the dead `ToExcelU.pas:306-309` and in two lines of `MoeinZipU` that
are commented out or inert):

**`XLX: TfrxXLSExport`** (`Dmu.dfm:8693-8709`)
```
FileName = 'D:\TEMP.XLX'          UseFileCache = True        ShowProgress = True
OverwritePrompt = False           DataOnly = False           ExportEMF = True
OpenExcelAfterExport = True       AsText = False             Background = True
FastExport = True                 PageBreaks = True          EmptyLines = True
SuppressPageHeadersFooters = False
```

**`JPG: TfrxJPEGExport`** (`Dmu.dfm:8715-8722`)
```
UseFileCache = True   ShowProgress = True   OverwritePrompt = True   DataOnly = False
```

**`CSV: TfrxCSVExport`** (`Dmu.dfm:8757-8770`)
```
UseFileCache = True   ShowProgress = True   OverwritePrompt = False   DataOnly = False
Separator = ','       OEMCodepage = False   UTF8 = False
OpenAfterExport = False   NoSysSymbols = True   ForcedQuotes = False
```

**How they are reached.** FastReport export filters register themselves globally on construction, so
every filter instantiated anywhere in the application appears in the **Export** menu of *every*
preview window. Since `PreviewOptions.Buttons` includes `pbExport` and `pbExportQuick` on all 58
reports (§6.2), **every report in the system can be exported to XLS, JPEG and CSV** even though no
line of code performs an export. This is the actual, undocumented export surface.

Properties that matter for the rebuild:

- **`DataOnly = False`** on all three: the export carries the page furniture — letterhead, column
  headers repeated per page, footers — not a clean data table. An XLS produced this way has merged
  cells, blank spacer rows and repeated headers, and is awkward to re-import.
- **`PageBreaks = True` and `EmptyLines = True`** on the XLS filter compound that: page breaks become
  worksheet page breaks and empty layout rows become empty worksheet rows.
- **`CSV.UTF8 = False` and `OEMCodepage = False`** — the CSV is written in the **ANSI code page**,
  i.e. Windows-1256 on a Persian system. Any consumer that assumes UTF-8 gets mojibake. This is the
  same encoding trap that makes the source tree itself hard to read.
- **`CSV.ForcedQuotes = False` with `Separator = ','`** — a description field containing a comma
  breaks the row. `Article` is `varchar(250)` free text; commas in it are certain.
- **`XLX.FileName = 'D:\TEMP.XLX'`** and `OverwritePrompt = False`: the default target is a
  hard-coded path on a `D:` drive with no confirmation. `MoeinZipU.FormClose:508-509` explicitly
  *resets* it to `'D:\TMP.XLS'` on every close — dead housekeeping for a code path that no longer
  exists, and it silently re-arms the hard-coded default for the next preview export.
- **`OpenExcelAfterExport = True`** launches Excel on the workstation.
- `.XLX` is not a real extension; the filter writes BIFF (`.xls`).

**Recommendation:** in the rebuild, exports are per-report endpoints returning a clean tabular
`text/csv` (UTF-8, RFC 4180 quoting) or `.xlsx`, plus a separate PDF of the rendered layout. Do not
port "export the rendered page".

### 7.2 `ToExcelDaraeiU` — the tax-authority voucher export

**The only purpose-built, live, file-producing export in the system.**

**Launched from** `Mainu.pas:417-420` (`TMain.EBCClick` → `ToExcelDaraei.init`), menu item `EBC`
captioned `ایجاد دفاتر الکترونیکی دارایی` — "create the tax authority's electronic books"
(`Mainu.dfm:10664-10667`). Reachable. Form caption `ذخیره سند در فایل اکسل` (`ToExcelDaraeiU.dfm:5`).

It replaced the dead `ToExcelU` (whose form creation is commented out in `arzi.dpr`, §1.13).

#### Parameter form

| Control | Type | Persian caption | Default | Validation |
|---|---|---|---|---|
| `_D1: TEditDate` | Jalali | `از تاریخ` (`.dfm:64`) | today, `Farsi_day := 1` (`:197-198`) | `Farsi_Valid` → `تاریخ را وارد کنید` (`:62-67`) |
| `_D2: TEditDate` | Jalali | `تا تاریخ` (`.dfm:53`) | today, `Farsi_day := 31` (`:199-200`) | `Farsi_Valid` (`:68-73`); `_D1 > _D2` → `رنج تاریخ را درست وارد کنید` (`:74-79`) |
| `F_Out: TEdit` + `SP_Out` (`...`) | file path | `نام فایل خروجی` (`.dfm:42`) | empty | non-empty → `نام فایل خروجی را وارد کنید` (`:81-87`); `TSaveDialog SD1` fills it (`:204-209`) |
| `Is_ReWrite: TsCheckBox` | bool | `فایل خروجی رو نویسی شود` — "overwrite the output file" (`.dfm:100`) | unchecked | if unchecked and the file exists → `فایل موجود است اجازه رونویسی را ندارید` (`:90-96`) |
| `Is_Open: TsCheckBox` | bool | `بعد از ذخیره فایل خروجی باز شود` — "open the output file after saving" (`.dfm:87`) | unchecked | none |

**Fiscal year is `DM.CO_ID` only** (`:102`) — no picker.

#### Exact SQL (verbatim, `ToExcelDaraeiU.dfm:184-216`)

```sql
Declare @D1 varchar(10) Set @D1 = :D1
Declare @D2 varchar(10) Set @D2 = :D2
Declare @Coid int Set @Coid=:Coid

if Object_ID('Tempdb..#R') is not null Drop Table #R

Select M_Sanad, M_Date
   , M_Ko As K, Space(100) as K_Name
   , M_Mo as M , Space(100) As M_Name
   , Article,M_Bed, M_Bes
into #R

from moein
Where (M_Date >= @D1) and (M_Date <= @D2) and (M_Coid=@Coid) and (M_kind=1)

update #R Set K_Name = ( Select S_Name From sarfasl Where S_KO=K and S_Mo=0)
update #R Set M_Name = ( Select S_Name From sarfasl Where S_KO=K and S_Mo=M and S_Ta1=0)
update #R Set Article =  REPLACE(Article, char(13)+char(10), ' ' )
update #R Set Article =  REPLACE(Article, '  ', ' ' )
update #R Set Article =  Left(Article, 200 )

Select * From #R
Order By M_date, M_Sanad
```

Notes:

- **`M_kind = 1` is filtered** — the tax export sees subsidiary detail only, not journal summaries.
  Correct.
- **`M_Tx` is not filtered** — **draft vouchers are exported to the tax authority.** This is the most
  consequential instance of the missing state filter in the whole system and needs an explicit
  decision (§9, §10).
- Only **Kol and Moein** are exported; `M_Ta1`/`M_Ta2` are dropped entirely, so analytic detail does
  not reach the file.
- `Space(100)` pre-allocates the name columns so the subsequent `UPDATE` has room — a `varchar(100)`
  by construction. A name longer than 100 characters is truncated silently.
- Text cleaning: CRLF → space, then **a single pass** of double-space → single space (`'  '` → `' '`),
  then truncate to 200 characters. One pass does not collapse runs of three or more spaces; and the
  200-character truncation is applied *after* cleaning, so it can cut mid-word. `Article` is
  `varchar(250)`, so up to 50 characters are lost.
- Sort `M_date, M_Sanad` — same key, and same missing third tie-break, as the ledgers (§3.1.c).

#### The export itself (`B_SaveClick:113-178`)

Direct COM automation of Excel, **not** a FastReport export:

```pascal
myExcel := CreateOleObject('Excel.application');
MyExcel.displayAlerts := False;
myExcel.caption := 'Green Gold System';
myExcel.visible := False;
Workbook := myexcel.workbooks.add;
sheet := workbook.worksheets.add;
Sheet.Name := 'صورت حساب الکترونیکی';
...
Sheet.Range['A2'].CopyFromRecordset(Q1.Recordset);
...
Workbook.Saveas( F_Out.Text , 51);
```

- **Requires Microsoft Excel installed on the workstation.** No Excel, no export.
- Sheet name `صورت حساب الکترونیکی` — "electronic statement".
- All worksheets except the first are deleted (`:125-127`), *after* one was added — so the surviving
  sheet is the newly added one.
- **`CopyFromRecordset`** bulk-writes the whole ADO recordset starting at `A2` — fast, and it means
  the column order is exactly the `SELECT` order.
- `SaveAs(…, 51)` — `xlOpenXMLWorkbook`, i.e. **`.xlsx`**. The file extension the user typed is not
  checked against that.
- Progress is shown through `WaitF.initForm('ایجاد صورتحساب', 1, 5)` with three `waitF.Next` steps
  (`:123-158`) — a five-step bar advanced three times.
- On completion: if `Is_Open` is unchecked, message `فایل  <path>  ایجاد شد` ("file … created") and
  `MyExcel.quit`; otherwise Excel is made visible and **left running** (`:171-177`). Either way the
  form closes.
- **No `try…finally`.** Any exception between `CreateOleObject` and `quit` leaves an invisible
  `EXCEL.EXE` process holding the workbook. `displayAlerts := False` is never restored.

#### Exported layout

Header row 1, data from row 2. Nine columns, Persian headers written cell by cell (`:131-139`), with
explicit widths (`:141-149`) and formatting applied to `A1:I<n+1>` (`:160-165`):

| Col | Cell | Persian header | English | Width | Source |
|---|---|---|---|---|---|
| A | `A1` | `ردیف` | row number | 5 | *(header only — `M_Sanad` lands here, see below)* |
| B | `B1` | `تاریخ` | date | 9 | `M_Date` |
| C | `C1` | `کل` | Kol code | 5 | `K` |
| D | `D1` | `نام کل` | Kol name | 20 | `K_Name` |
| E | `E1` | `معین` | Moein code | 5 | `M` |
| F | `F1` | `نام معین` | Moein name | 20 | `M_Name` |
| G | `G1` | `شرح` | description | 80 | `Article` |
| H | `H1` | `مبلغ بدهکار` | debit amount | 18 | `M_Bed` |
| I | `I1` | `مبلغ بستانکار` | credit amount | 18 | `M_Bes` |

**Header/data mismatch.** The `SELECT` order is `M_Sanad, M_Date, K, K_Name, M, M_Name, Article,
M_Bed, M_Bes` — nine columns whose *first* is the **voucher number**, but column A is headed
`ردیف` ("row number"). The export therefore labels voucher numbers as row numbers. Every other
column lines up. This is a real defect in a file submitted to the tax authority; verify against the
authority's expected template before porting (§9).

Formatting: whole range `Tahoma` 10 pt; header row bold with background `$CAE4FF` (a pale blue).
No number formatting is applied, so amounts export as raw integers — which is what a re-importable
file should do, and is better than the FastReport path.

**Writes: none** to the application database.

### 7.3 `MoeinZipU` — grid-to-Excel, no file

`B_Save1Click` (`MoeinZipU.pas:382-427`) is live and does something different again: it copies the
**on-screen grid** into a new Excel workbook cell by cell, then just makes Excel visible.

```pascal
myExcel := CreateOleObject('Excel.Application');
WorkBook := myExcel.Workbooks.Add;
 while WorkBook.Worksheets.Count > 1 do WorkBook.Worksheets[WorkBook.Worksheets.Count].Delete;
Sheet := WorkBook.Worksheets[1];
Range := Sheet.range['A1:'+char(G1.Columns.Count+96)+ inttostr(Q1.RecordCount+1) ];
Range.font.name := 'Tahoma';  Range.font.size := 10;  Range.Borders.LineStyle := 1;
Range := Sheet.range['A1:'+char(G1.Columns.Count+96)+ '1' ];
Range.interior.color := $00CAE4FF ;
For C:=1 to G1.Columns.Count Do
    Sheet.Cells[1,C].value := G1.Columns[C-1].Title.Caption ;
Q1.First;
for R := 1 to Q1.RecordCount do
Begin
  if R mod 2 = 0 then
  begin
    Range := Sheet.range['A'+inttostr(R+1)+':'+char(G1.Columns.Count+96)+inttostr(R+1) ];
    Range.interior.color :=  $00CFEFD2;
  end;
  for C := 1 to G1.Columns.Count do
    Sheet.cells[R+1,C].value := Q1.FieldByName( G1.Columns[C-1].displayName).asstring;
  Q1.Next;
End;
myExcel.visible := true;
```

Characteristics and defects:

- **The exported columns are whatever the grid currently shows**, with the grid's `Title.Caption` as
  the header — so the Excel layout follows the user's chosen level and column set. That is genuinely
  useful behaviour and worth keeping as "export current view".
- **It never saves.** No `SaveAs`, no filename prompt; the user is handed an unsaved workbook.
- **`char(G1.Columns.Count + 96)`** builds the last column letter as `'a' + count - 1`. It produces
  a **lower-case** letter (Excel accepts it) and **breaks completely past 26 columns** — column 27
  yields `'{'`. The voucher summary can exceed 26 columns when all analytic levels are on.
- `.asstring` is used for every cell, so **every value lands in Excel as text**, including amounts and
  dates. No number formatting, no sums possible without re-typing the column.
- Alternating row shading `$00CFEFD2` on even rows and header fill `$00CAE4FF` reproduce the grid's
  banding — page furniture in a data file.
- Cell-by-cell COM assignment is one round trip per cell; a 5 000-row × 10-column export is 50 000
  COM calls.
- Same missing `try…finally` and orphaned-Excel-process problem as §7.2.

**`MoeinZipU`'s FastReport XLS export is dead.** `:453-463` — `Rp1.PrepareReport`,
`DM.XLX.FileName := F_Out.Text`, `Rp1.Export(DM.XLX)` and the confirmation message are all commented
out, along with the eight `(Rp1.FindObject('T11'…'M14') as TfrxMemoView).Visible := …` lines that used
to toggle report columns to match the chosen level (`:439-451`). The only surviving `DM.XLX`
references in the unit are the two `FormClose` lines (`:508-509`) that reset the filter's filename.

`Button1Click` (`:108-232`) is a 120-line `(* … *)` block of Excel-OLE sample code — borders, merges,
page setup, clipboard paste — that was never part of the feature. Delete on sight.

### 7.4 `Anbar_Amalkard` — three more Excel exports

`Anbar_Amalkard.pas:259`, `:330`, `:391` each run `XApp := CreateOleObject('Excel.Application')`.
Owned by `05-inventory.md`. Flagged here only because this is the same unit that performs the
unconditional `UPDATE Anbar_FactorD SET AFD_Customer = (…)` with no `WHERE` (§1.11) — anyone
exporting from it also mutates the table.

### 7.5 Image export

`JPG: TfrxJPEGExport` exists on the data module and no code calls it. It is nonetheless reachable
from every preview's Export menu (§7.1), producing a **raster image of the rendered page** at the
preview's resolution. `OverwritePrompt = True` is the only filter of the three that asks before
overwriting. There is no PNG, PDF, RTF or HTML export component anywhere in the project — **no report
can produce a PDF**, which is worth stating plainly because users of the rebuilt system will expect
one.

### 7.6 Summary — which report can export what

| Report | FastReport XLS/CSV/JPG via preview | Purpose-built export |
|---|---|---|
| every report with a preview (all 51 live `ShowReport` sites) | yes | — |
| `PrintN.FastPrint` path (`SanadEditU.pas:830`) | **no** — `Rp2.Print` bypasses the preview | — |
| `MoeinZipU` (`خلاصه اسناد معین`) | yes | grid → unsaved Excel workbook (§7.3) |
| `ToExcelDaraeiU` (`ذخیره سند در فایل اکسل`) | its `RP1` exists but is never shown | **`.xlsx` file, 9 columns** (§7.2) |
| `Anbar_Amalkard` | yes | three Excel workbooks (`05-inventory.md`) |
| everything else | yes | — |


---

[← SS6 Print pipeline (2/2)](04-06-b-print-pipeline.md) | [Index](00-index.md) | [SS8 Rebuild recommendations →](04-08-rebuild-recommendations.md)
