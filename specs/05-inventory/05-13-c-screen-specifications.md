_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 13.14 Invoice print — `FactorPrint3U`

**Purpose:** print up to four different form layouts of the same invoice in one pass, on A4 or A5.

| Control | Persian caption | Behaviour |
|---|---|---|
| `FactorNo` | | invoice number, pre-filled by `init(FN)` |
| `C1` … `C4` | *(four form checkboxes)* | select which report pages print; each state persisted to INI (`:165-168, 183-186`). At least one must be checked, else `یک فرم برای چاپ انتخاب کنید` ("select a form to print") |
| `A4orA5` | `چاپ روی کاغذ A4` / `چاپ روی کاغذ A5` | a **toggle button whose caption is its next state or its current state — ambiguous**; `Tag` 1 = A4 portrait, 2 = A5 landscape (`:51-58, 122-133`) |
| `B_Config` | | opens `TanzimChap` (print settings) |
| `B_Print` / `B_Exit` | | |

**Background images.** Each of the four pages loads
`<exedir>\images\BACK_1.png` … `BACK_4.png` as a page background if the file exists
(`:147-154`). **The printed form design is therefore partly a set of PNG files on disk**, not in
the binary. The rebuild must obtain those four images.

**Data the print needs — three datasets:**

`Q1` (`FactorPrint3U.dfm:3238-3258`) — the header, joined for display:

```sql
Select Anbar_Factor.* , Sarfasl.*
   , Base.Co_Name
   , Type = Case AF_Type When 1 then 'رسید انبار' when 2 then 'فاکتورفروش کالاو خدمات'
                         when 3 then 'برگشت از خرید' when 4 then 'برگشت از فروش' end
   , Emza = ( Select T_Str From Tanzim Where T_ID = 1012 )
 From Anbar_Factor
Left Join Base On Base.CO_ID=@CoID
Left Join Sarfasl On Sarfasl.S_SSN = Anbar_Factor.AF_Customer
Where AF_Factor=@Factor and AF_Coid=@Coid
```

(`Emza` = `امضا`, the signature block, from settings key 1012. A **fourth** copy of the type-label
mapping.)

`Q2` — the lines: `Select * From Anbar_FactorD Where AFD_Factor=:Factor And AFD_CoID=:Sal`.

`Q3` — the settlement instruments, the same `DFish` ∪ `DCheck` union as §9.2 but keyed
`S_LinkPRG=1 and S_LinkSSN=@Factor`, with `_ID` 22/21 (the §9.6 discriminator) and columns
`S_BesCR, S_Date, S_StateName, S_SSN, S_FishNo|S_CheckNo, S_Mab, S_Desc, S_DateS`.
**So the printed invoice shows how it was paid.**

Runtime-injected: `Tot3`/`Tot4` = `'جمع : ' + Str2String(AF_Total) + ' ريال '`;
`T1` = `Dm.Get_Jari_Code(AF_Customer)` — the buyer's current-account code.

`FormClose` closes `DM.SP_AnbarPrintFactor` (`:172`) — **a dataset this unit never opens**. Left
over from an earlier implementation that used the stored procedure; see §12.

---

### 13.15 **Unreachable** — invoice print variant 1, `FactorPrintU`

`FactorPrint.init(FN)` (`FactorPrintU.pas:310-315`) exists and is never called. The unit has a
`RP1GetValue` callback (`:317+`) supplying report variables, and 280 lines of `B_PrintClick`
(`:60-281`) — the largest print handler in the module. **Superseded by `FactorPrint3U`.**
Do not port; check with the business whether any of its four layouts are still wanted before
discarding it (§14).

---

### 13.16 Subsystem-B document list and posting — `SodoorSanadU`

**Purpose:** browse `FactorMaster` and post/un-post its vouchers (§10.2).

| Control | Purpose |
|---|---|
| `Taeid0` / radio group | document-type filter, `sPanel3` (`SodoorSanadU.dfm:110-165`): `همه موارد` all / `حواله انبار و برگشت` issue+return `(13,22)` / `رسید انبار و خرید پسته` receipt+pistachio `(12,14)` / `اول دوره و جابجایی` opening+transfer `(11,16,26)` / `تولید` production `(15,25)` |
| `G1` | grid over `FactorMaster LEFT JOIN FactorKind`, with `Mab1`/`Mab2` = deposit-slip and cheque sums |

| Button | Action | `.pas` |
|---|---|---|
| `B_Sodoor` | `صدور سند` post → routes by `FM_ID` into `MakeSanadF.init11/12/13/22`, else `' Not implemented yet. '` | `:167-208` |
| `B_Delete` | un-post — §5.3.2 | `:211-270` |
| `B_ViewFactor` | print: `FM_ID ∈ {15,25}` → `Print_Anbar15F.print_Tolid`; `{16,26}` → `Print_Anbar16F.print_Jabejaei`; **anything else does nothing at all** | `:272-285` |
| `B_PrintList` | FastReport list print | `:157-165` |
| *(settle)* | `FM_ID = 22` only, else `فقط برای فاکتورهای فروش فعال است` → `TasfiehFactorF.Init_Pesteh(FM_SSN)` | `:149-154` |

> **The list query joins treasury without an `S_LinkPRG` filter** (`SodoorSanadU.dfm:526-528`) —
> §10.4.

---

### 13.17 Voucher preview — `MakeSanadU`

Modal, opened by `SodoorSanadU`. Caption is set per document type
(`صدور سند موجودی اول دوره` / `صدور سند خرید مواد و کالا` / `صدور سند برگشت از فروش` /
`صدور سند فروش`).

| Control | Persian label | Editable |
|---|---|---|
| `DM_Coid` | fiscal year | no |
| `DM_Date` | voucher date | no — taken from `FM_Date` |
| `DM_Sanad` | voucher number | no — allocated by `Get_NewSanad_DateID` |
| `_Factor` | source document number | no |
| `DM_Desc` | narration | **yes** — defaults to `' عملیات انبار مورخ ' + FM_Date` ("warehouse operations dated …"); this is the focused control on open |
| `G1` over `VSanad` | the proposed voucher lines: `M_CR`, `M_Code`, `M_Name`, `M_Bed`, `M_Bes`, `Article` | display only |

Buttons: `B_Ok` (writes, §10.2), `B_Exit` (closes). `FormResize` (`:170-184`) recomputes control
widths manually — a hand-rolled layout the React version replaces with flexbox.

**The grid is the operator's only chance to see the entry before it posts, and there is no balance
indicator on it** (§10.2.5).

---

### 13.18 Production and transfer prints — `Print_Anbar15`, `Print_Anbar16`

Both are read-only preview forms: a master grid over `FactorMaster`, a detail grid over
`FactorDetail`, `B_Ok` → `RP1.ShowReport(True)`, `B_Exit` → `Close`, `Q1BeforeDelete` → `abort`.

| | `Print_Anbar15` (production) | `Print_Anbar16` (transfer) |
|---|---|---|
| Entry | `print_Tolid(FM_SSN)` | `print_Jabejaei(FM_SSN)` |
| Master set | **all** `FM_ID = 15` for the same `(FM_Coid, FM_Date)` — a day's production run | the single 26 (out) document of the pair |
| Detail set | `FD_FMSSN in (<15's SSN>, <25's SSN>)` — **both sides in one grid**, refreshed by `DS1DataChange` | `FD_FMSSN = <26's SSN>` — **out side only** |
| Pairing | `FM_Link` resolved as a `FM_Factor` number | same |
| Dead code | `ID1 := 15; ID2 := 25` assigned, never read (`:175`) | `_A1`/`_A2` warehouse ids captured for the report |
| Fragility | falls back to `SSN2 := SSN1` if the pair is missing, showing one side twice (`:141`) | no `RecordCount > 1` check on the ambiguous `FM_Factor` lookup (§3.2.3) |

`Print_Anbar15` also declares 23 anonymous persistent fields (`AutoIncField1`, `IntegerField1` …
`StringField6`, `:65-87`) belonging to a fourth dataset that is never populated — dead weight.

---

### 13.19 Pistachio screens

- `FactorPesteh_U` — §8.3, §8.4.1. Five of nine buttons dead; the "reverse voucher" button has an
  empty handler.
- `PestehD_U` — §8.2. Unreachable; but its formula is the specification.
- `Kharid_U` — §8.0.1. Unreachable and has no save handler.
- `Kharid_BU` — §8.5. Unreachable; `Kh4_Del` clears the wrong slot.
- `Lab` / `Ghabz` / `Get_Serial` — §8.6. Unreachable and mostly commented out.

---

### 13.20 Cross-cutting UI conventions to reproduce (or deliberately drop)

| Convention | Where | Rebuild note |
|---|---|---|
| **Window geometry and grid column widths persisted to an INI file per form name** | every screen: `FormActivate` reads `Left/Top/Width/Height` and `G1C<n>`, `FormClose` writes them | Replace with per-user layout preferences in the database or `localStorage`. It is genuinely used and operators will miss it. |
| **Grid font size adjustable per screen** | `AnbarListU` `GridFontSize`, `DM.GridFontSize` | Keep as a user preference |
| **Enter and ↓ advance to the next control; ↑ goes back** | `AnbarTanzimU.pas:124-137` | Reproduce — this is how the data-entry operators work |
| **RTL throughout** (`BiDiMode = bdRightToLeft`) | every form | `dir="rtl"` |
| **Red label = mandatory field** | `PestehD_U.pas:131-138`, `AnbarCalaAddU.dfm` | Replace with a real required-field indicator plus validation |
| **A read-only box that is clickable** | `AnbarFactorAddU` `Phi1` | Replace with a button |
| **Silent `Exit` on validation failure** | `AnbarCardJensiU` ×4, `AnbarReportKharidU` ×3, `AnbarFactorAddU` `Total = 0` | Always show a message |
| **`ShowModal` everywhere; no non-modal workflow** | all | Modals are fine for the editors; the list screens should be routes |
| **Client-side grid footers (`RecalculateSummaryResults`)** | `AnbarFactorU`, `AnbarListU`, `TasfiehFactor`, `MakeSanadU`, `AnbarReportU` | Compute server-side; today's totals reflect only the loaded page |
| **`Top N` row limiting with no paging** | `AnbarListU` | Real pagination |
| **Excel automation via OLE** | `Anbar_Amalkard` | Server-side XLSX |
| **Report layouts in FastReport `.fr3` blobs inside `.dfm`, plus PNG backgrounds on disk** | all print units | The print layouts must be re-authored; they are not portable |


---

[← 13. Screen specifications (part b)](05-13-b-screen-specifications.md) | [index](00-index.md) | [14. Open questions →](05-14-open-questions.md)
