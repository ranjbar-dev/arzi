_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 6. Print pipeline

Everything printable in this application goes through **FastReport VCL 6.9.6**
(`Version = '6.9.6'` on all 58 `TfrxReport` components). There is no other print path — no
`TPrinter` drawing, no HTML, no PDF library.

### 6.1 How a report is invoked

The pattern is identical in all 51 call sites:

1. Open the feeding `TADOQuery` / `TADOStoredProc`. The report binds to it through a
   `TfrxDBDataset` (`UserName = 'DB1'` almost everywhere) declared in the same `.dfm`.
2. **Push runtime text into named memos by string lookup**:
   `( Rp1.FindObject('T1') as TfrxMemoView ).Text := <value>;`
3. `Rp1.ShowReport(true)` — modal preview window; the user prints from there.

Two variants exist:

- **`FindComponent` instead of `FindObject`** — `KolStateU.pas:106-107`, `BedBes.pas:174`. Both
  happen to work because the memos are directly owned by the report, but it is a different lookup
  path and will return `nil` (and raise on the hard cast) if the object nesting ever changes.
- **Silent direct print, no preview** — `PrintNu.pas:90-91`:
  ```pascal
  RP2.PrepareReport(True);
  Rp2.Print;
  ```
  reached from `SanadEditU.pas:830` (`PrintN.FastPrint`). This is the only path that bypasses the
  preview and goes straight to the default printer.

There is **no error handling** around any of it. `FindObject` returning `nil` produces an access
violation on the `as TfrxMemoView` cast; no report guards against it.

**Data binding is by convention, not by contract.** The memo *names* (`T1`, `T2`, `_Total`, `B3`,
`_Name`, `Co_Name`, `L1`, `_Totals`, `_Total3`, `C1..C6`, `D1..D6`, `S1..S4`) are string literals
duplicated between the `.pas` and the `.dfm`. Renaming a memo in the designer silently breaks the
runtime injection. §11.4 maps them to a structured header object.

### 6.2 Page setup

Read from the `TfrxReportPage` in each `.dfm`. Across the whole application:

| Property | Value | Count |
|---|---|---|
| `PaperSize` | `9` (A4, 210 × 297 mm) | 57 pages |
| `PaperSize` | `256` (custom) | 3 pages |
| `PaperSize` | `11` (A5) | 1 page |
| `Orientation` | `poPortrait` (default, absent) | 39 pages |
| `Orientation` | `poLandscape` | 19 pages |
| `LeftMargin` | `10.0` mm | 60 pages (one page has `10` written as integer) |
| `Columns` | empty / `1` | all — **no multi-column layout anywhere** |
| `PrintOptions.Printer` | `'Default'` | 58 |
| `PrintOptions.PrintOnSheet` | `0` | 58 |
| `PreviewOptions.Zoom` | `1.0` | 58 |

Every accounting report in §2–§4 is **A4 portrait**: `Taraz4Setooni_U.dfm:812-814`,
`Taraz6SetooniU.dfm`, `DKolU.dfm:633-635`, `DMoein.dfm:846`, `KolStateU.dfm:170`, `BedBes.dfm:569`.
The landscape pages belong to the wide inventory and cheque listings.

`PreviewOptions.Buttons` is the full set on every report:
`[pbPrint, pbLoad, pbSave, pbExport, pbZoom, pbFind, pbOutline, pbPageSetup, pbTools, pbEdit,
pbNavigator, pbExportQuick, pbCopy, pbSelection]`. Note **`pbEdit` and `pbLoad`**: any user who can
open any preview can edit the report layout in the FastReport designer and load a different `.fr3`
from disk. That is a live capability, not a theoretical one, and it is ungated.

### 6.3 Bands and pagination

The band vocabulary in use is small:

| Band | Purpose | Example |
|---|---|---|
| `TfrxReportTitle` | printed once at the start | `ToExcelDaraeiU.dfm:269` |
| `TfrxPageHeader` | repeated column strip on every page | `DKolU.dfm:642`, `Taraz4Setooni_U.dfm:821` |
| `TfrxMasterData` | the detail rows | everywhere |
| `TfrxFooter` | grand totals at the end of the data | `DKolU.dfm:1121`, `BedBes.dfm:1003` |
| `TfrxDataPage` | the (empty) data-definition page | every report |

**There are no `TfrxGroupHeader` bands anywhere in the accounting reports.** Every grouping the user
sees — trial-balance levels, ledger sections — is produced by *interleaving pre-aggregated rows in the
result set* and distinguishing them with a level column (`St`, `IsLast`, `S_Mo = 0`) plus row colour.
Consequently:

- **No group subtotals exist.** The only totals are the `Footer1` grand totals.
- **No page-break rule is set on any accounting report.** `StartNewPage`, `PrintOnParent` and the
  break-related band properties are absent; pagination is purely "what fits on A4".
- **Grand totals are protected against double counting by an `IIF` filter inside the `SUM`
  expression**, not by band scoping: `IIF(<DB1."St"> > 1, 0, 1)` (§2.1),
  `IIF(<DB."S_mo"> = 0, …, 0)` (§2.2). Any level column the rebuild introduces must keep this
  property or the totals inflate.

**Row banding** is done per memo with `Highlight.Condition`, not by the band:
`'<line> mod 2 = 1'` with `Highlight.Fill.BackColor = 15794160` on `DKolU.dfm:889-891`,
`DMoein`, `BedBes` and others. `Taraz4Setooni_U` instead computes colours in PascalScript
(`.dfm:748-797`) and `Taraz6SetooniU` in `Bes1OnAfterData` (`.dfm:185-237`).

**Report-side PascalScript** is used in exactly three ways, all of them in §2–§3 reports:
alternating colour by level; the `بس`/`بد` side letter (`DKolU.dfm:606-615`, `DMoein.dfm:817-829`);
and a `GetValue` event handler in `PrintMU.pas:74-120` that builds a composite description string and
mutates memo frames and colours per row while the report renders. That last one is the most
sophisticated thing in the print layer and the hardest to port — see §6.6.

### 6.4 Header, letterhead and footer content

Nothing is stored in the layout except placeholder text; **everything meaningful is injected at
runtime**. The recurring pieces:

| Injected value | Source | Typical target memo |
|---|---|---|
| Organisation name | `DM.RegName` (`Dmu.pas:111`, loaded from `Base.CO_Name`) | `T1`, `Co_Name`, `L1`, `Reg` |
| Fiscal-year caption | `DM.RegSal`, or `COID.Text` from the picker | appended to `T1` |
| Report title | Persian literal in the `.pas` | `T1` / `T2` |
| Period | `'از تاریخ : ' + D1 + CRLF + 'تا تاریخ : ' + D2` | `T6`, `_D1`/`_D2` |
| Page number | the FastReport built-in `'صفحه : [Page#]  از [TotalPages#] '` | appended to `T6` |
| Row number | the built-in `[line#]` | first data column, header `ردیف` |
| Amount in Persian words | `Dm.Str2String(<total>)` | `B3`, `_Totals` |
| Signature block | `Dm.Get_paramstr(1011 \| 1013 \| 1014)` | `_Total`, `_Total3`, `B4`, `S1011..S1014` |

**Note the design-time text is frequently stale.** `DKolU.dfm:661-664` still says
`مشاهده دفتر معین` on the general-ledger report (overwritten at runtime), `KolStateU.dfm:284,301`
carries `گردش بدهکار` in the two memos that become the letterhead and the title, and
`BedBes.dfm:928` has the literal `'T1'`. If any injection is ever skipped, the placeholder prints.
`Taraz6SetooniU.pas:100-101` is exactly that case: the `_T2`/`_T3` assignments are commented out and
the static text prints instead (§2.2).

**Amount in words.** `TDm.Str2String` (`Dmu.pas:604-635`) converts a digit string to Persian words by
chopping three digits at a time off the right and prefixing the scale word:
`هزار` (thousand), `ميليون` (million), `ميليارد` (billion), `تريليارد` (trillion), joined with `' و'`,
with `N23` (`Dmu.pas:…-602`) rendering each 3-digit group. Notes for the rebuild:
- The scale words are spelled with **Arabic yeh `ي`** (`ميليون`, `ميليارد`) while the rest of the UI
  uses Persian yeh `ی`. Preserve the spelling if byte-identical output matters; otherwise normalise.
- `تريليارد` for 10¹² is non-standard Persian (the usual term is `تریلیون`).
- It takes a **string**, not an integer, and slices it with `Copy` — a negative sign or a thousands
  separator in the input produces garbage. Every call site passes an unformatted `AsString`.
- Callers: `RooznamehViewU.pas:170`, `PrintMU.pas:64,92`, `PrintM2U.pas:72`, `PrintNu.pas:86,135`.

**Letterhead image.** One mechanism, in one place: `PrintNu.pas:94-103` loads
`<exe dir>\images\back_Sanad.png` into a `TfrxPictureView` named `Picture1` at form-create time, and
clears the picture first so a missing file yields a blank rather than the design-time image:

```pascal
S:= ExtractFilePath( paramstr(0) ) + 'images\back_Sanad.png';
( Rp2.FindObject('Picture1') as TfrxPictureView).Picture := nil;
if FileExists(S) then
   ( Rp2.FindObject('Picture1') as TfrxPictureView).Picture.LoadFromFile(S);
```

No other report has a letterhead image; the rest print the organisation name as text. In the rebuild
this becomes a single uploaded asset on the organisation record, applied by the template.

### 6.5 Print settings — `TanzimChapu`

**Launched from** `Mainu.pas:508-511` (`TMain.B_FormClick` → `TanzimChap.init`), button `B_Form`.
Form caption `تنظیمات چاپ فاکتور` — "invoice print settings" (`TanzimChapu.dfm:5`). Reachable.

It is a flat editor over the **`Tanzim` key/value table** (`TADOTable`, `Dmu.dfm:745-751`), reached
through `TDM.Get_paramstr` / `TDM.Set_paramstr` (`Dmu.pas:468-508`). `Tanzim` columns: `T_ID` (int
key), `T_Str` (the value), `T_Int`, `T_Desc`.

| Key | Control | Persian label (`Dmu.pas:472-486`) | Meaning | Consumed by |
|---|---|---|---|---|
| 1001 | `S1001` | `فاکتور امضا 1` | invoice signature 1 | invoice prints (`05`) |
| 1002 | `S1002` | `فاکتور امضا 2` | invoice signature 2 | invoice prints |
| 1003 | `S1003` | `فاکتور امضا 3` | invoice signature 3 | invoice prints |
| 1004 | `S1004` | `فاکتور امضا 4` | invoice signature 4 | invoice prints |
| 1005 | `S1005` | `فاکتور عنوان 1` | invoice heading 1 | invoice prints |
| 1006 | `S1006` | `فاکتور عنوان 2` | invoice heading 2 | invoice prints |
| 1007 | `S1007` | `طرف حساب` | counterparty caption | invoice prints |
| 1008 | `S1008` (bool) | `نمایش مبلغ` | show amount | invoice prints |
| 1009 | `S1009` (bool) | `نمایش تخفیف` | show discount | invoice prints |
| 1010 | `S1010` (bool) | `نمایش مالیات` | show tax | invoice prints |
| 1011 | `S1011` | `سند امضا 1` | voucher signature 1 | `RooznamehViewU:171`, `PrintMU:60`, `PrintM2U:67`, `PrintNu:139` |
| 1012 | `S1012` | `سند امضا 2` | voucher signature 2 | `PrintMU:61` only (commented out in `PrintM2U:68`) |
| 1013 | `S1013` | `سند امضا 3` | voucher signature 3 | `DMoein:401`, `RoyatJU:407`, `PrintMU:62` |
| 1014 | `S1014` | `سند امضا 4` | voucher signature 4 | `Taraz4Setooni_U:174`, `Taraz6SetooniU:102`, `PrintMU:63` |
| 1015 | `Tanzim1015` (memo) | `پانویس فاکتور رسمی` | official-invoice footnote | `Factorprint2U` (`05`) |

Two important behaviours:

- **`Get_paramstr` self-heals.** If `T_ID` is not found it **appends a row** with `T_Str` and
  `T_Desc` set to the Persian label above and `T_Int = '0'` (`Dmu.pas:489-497`). So the first read of
  a key writes it — a *getter that writes*. It also means the shipped default value of every setting
  is its own label text, which is why fresh installations print `سند امضا 4` where a signature block
  should be.
- **`Set_paramstr` silently does nothing** if the key is missing (`Dmu.pas:504`, `if Not … Locate …
  then exit`). Since `Get_paramstr` creates the row, the pairing works — but only if a read precedes
  the write, which `TanzimChap.init:103-119` guarantees.

**What `TanzimChapu` does *not* control:** paper size, orientation, margins, printer, font, copies,
number format, or which of the three voucher templates is used. All of those are baked into the
`.dfm`. The form's own caption calls it *invoice* print settings, and 12 of its 15 keys are indeed
invoice-only; the four voucher signature keys are the only part that reaches the accounting reports.

**Template selection is hard-coded, not configured.** `SanadEditU.pas:521-529` chooses between the
three voucher layouts in source:
```pascal
    PrintN.Print( S_Sanad.IntValue );
//    PrintM.Print( S_Sanad.IntValue );
    ...
    PrintM2.Print( S_Sanad.IntValue );
```
`PrintM` is commented out — **`PrintMU` is dead**, and with it its `RO_OLD` report and its elaborate
`RP2GetValue` script. `PrintM2.Print1` (`PrintM2U.pas:79-93`) has no caller either. The live voucher
prints are `PrintN.Print` (preview), `PrintN.FastPrint` (direct) and `PrintM2.Print`.


---

[← SS5 Date-range and fiscal-year filtering semantics](04-05-date-range-and-fiscal-year-filtering-semantics.md) | [Index](00-index.md) | [SS6 Print pipeline (2/2) →](04-06-b-print-pipeline.md)
