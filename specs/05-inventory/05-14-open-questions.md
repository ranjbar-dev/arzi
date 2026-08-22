_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 14. Open questions

Each row states what is unknown, why it matters, and **the exact query, file or action that
answers it**. Ordered by impact on the rebuild.

### 14.1 Blocking — the rebuild cannot be specified without these

| # | Question | Why it matters | How to answer |
|---|---|---|---|
| **Q1** | **What does `Anbar_AddToFactor` do?** Specifically: which `M_ID` values does it write, what are the debit/credit rules per `@Type`, and what does it do with `@Customer = 0`? | The entire accounting integration of subsystem A (§10.1) is inference. If it writes any `M_ID` other than `1`, invoice voucher lines **double on every re-save** (§10.1.3). | `SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_AddToFactor'));` |
| **Q2** | **Does `Anbar_CardJensi` emit an opening-balance row, and what is its `ORDER BY`?** | The stock card's running balance starts at `R := 0` and accumulates in result order (§11.1.3). Without an opening row the card is wrong for any window that is not the whole year; and if the order is `AFD_SSN`, editing an old invoice silently reorders every card. | `SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_CardJensi'));` |
| **Q3** | **What precision does `Anbar_Mandeh` return for `Remi`?** | The year-opening generator reads it with `.AsInteger` (§6.4), permanently destroying fractional stock at every year boundary. Whether that is already happening depends on the SP. | `SELECT OBJECT_DEFINITION(OBJECT_ID('Anbar_Mandeh'));` then `SELECT AFD_Code, SUM(...) FROM Anbar_FactorD ... HAVING SUM(...) <> FLOOR(SUM(...))` to count affected items |
| **Q4** | **Is there a unique constraint on `(AF_COID, AF_Factor)`, and an insert trigger on `FactorMaster`?** | §5.4's concurrency analysis assumes no constraint. The `@@Identity` read at `FactorPesteh_U.pas:203` is only correct if there is no trigger. | `SELECT * FROM sys.indexes WHERE object_id=OBJECT_ID('Anbar_Factor'); SELECT * FROM sys.triggers WHERE parent_id=OBJECT_ID('Anbar.dbo.FactorMaster');` |
| **Q5** | **Full DDL for `Anbar_Jens`, `Anbar_Config`, `Anbar_Vahed`, `Anbar_Factor`, `Anbar_FactorD`, `Kinds`.** | Nullability, defaults, precisions and collations are inferred throughout §1 and §3. | the `sys.columns` query at §12.7 item 6 |

### 14.2 High — affect data migration correctness

| # | Question | Why it matters | How to answer |
|---|---|---|---|
| **Q6** | **Do any `FM_ID ∈ {11,12}` documents carry a non-zero `FM_Kasr` or `FM_Maliat`?** | If yes, §10.2.5's unbalanced vouchers are live in production and the ledger is wrong by `2·Kasr − Maliat` per document. If no, the defect is latent and the fix is safe. | `SELECT COUNT(*), SUM(FM_Kasr), SUM(FM_Maliat) FROM Anbar.dbo.FactorMaster WHERE FM_ID IN (11,12) AND (FM_Kasr<>0 OR FM_Maliat<>0);` |
| **Q7** | **How many orphaned `Moein` lines with `M_Id IN (31,34)` exist?** | The §5.3 defect. `M_Id=34` orphans are permanent — no code path can remove them. | `SELECT m.M_Sanad, m.M_Id, COUNT(*), SUM(m.M_Bed), SUM(m.M_Bes) FROM Moein m LEFT JOIN Anbar.dbo.FactorMaster f ON (m.M_Id=34 AND f.FM_Factor=m.M_Link AND f.FM_Coid=m.M_Coid) OR (m.M_Id=31 AND f.FM_SSN=m.M_Link) WHERE m.M_Id IN (31,34) AND (f.FM_SSN IS NULL OR f.FM_SanadNo<>m.M_Sanad) GROUP BY m.M_Sanad, m.M_Id;` |
| **Q8** | **How many invoices have `AF_Customer = 0`?** | The broken guard at `AnbarFactorU.pas:579-583` (§4.2.2). These invoices have no counterparty and their voucher lines point at account id 0. | `SELECT COUNT(*) FROM Anbar_Factor WHERE AF_Customer=0 OR AF_Customer IS NULL;` |
| **Q9** | **Are the twenty dead header columns (`AF_Sel2-5`, `AF_Mab1-5`, `AF_Desc1-5`, `AF_Date1-5`) ever non-null?** | §3.1.1 proposes dropping them. If historic rows carry data, that data has meaning nobody in the code remembers. | `SELECT COUNT(*) FROM Anbar_Factor WHERE COALESCE(AF_Sel2,0)+COALESCE(AF_Sel3,0)+… <> 0 OR COALESCE(AF_Desc1,'')<>'' OR …;` |
| **Q10** | **Do any `Anbar_Jens` rows have `AJ_VahedC` null or 0?** | Such items cannot be re-saved from the editor and may crash it (§2.2.3). | `SELECT COUNT(*) FROM Anbar_Jens WHERE AJ_VahedC IS NULL OR AJ_VahedC=0;` |
| **Q11** | **Do any items have code `0` or a negative `AJ_Phi`?** | §2.1, §2.2.1. Item `0` is uneditable from the UI. | `SELECT * FROM Anbar_Jens WHERE AJ_Code<=0 OR AJ_Phi<0;` |
| **Q12** | **Do any `DFish` / `DCheck` rows have `S_LinkPRG = 0` with a plausible invoice number in `S_LinkSSN`?** | §9.6 shows the linking strategy changed from "match by voucher + counterparty" to explicit link columns. Pre-migration rows may be unlinked. | `SELECT S_LinkPRG, COUNT(*) FROM DFish GROUP BY S_LinkPRG; -- and the same for DCheck` |
| **Q13** | **How many items have negative stock today?** | Determines whether a non-negative constraint can be applied at migration (§5.2.3). | run the `Anbar_MandehU.Q1` SQL of §5.1.2 for each `COID` and count `R2 < 0` |
| **Q14** | **Are `AFD_Customer` values currently consistent with `AF_Customer`?** | `Anbar_Amalkard` repairs them table-wide on every run (§13.10). The drift window tells you whether anything reads the stale value. | `SELECT COUNT(*) FROM Anbar_FactorD d JOIN Anbar_Factor f ON f.AF_Coid=d.AFD_Coid AND f.AF_Factor=d.AFD_Factor WHERE d.AFD_Customer<>f.AF_Customer;` |

### 14.3 Medium — affect scope and behaviour decisions

| # | Question | Why it matters | How to answer |
|---|---|---|---|
| **Q15** | **What are the actual rows of `Kinds`?** | §8.1's seven grades come from a **source comment**, not from data. | `SELECT * FROM Kinds ORDER BY K_id;` |
| **Q16** | **What are the actual rows of `Anbar_Vahed`?** | There is no maintenance screen; the table is populated only by direct SQL (§1.3). | `SELECT * FROM Anbar_Vahed ORDER BY AV_Name;` |
| **Q17** | **What are the actual rows of `Anbar.dbo.FactorKind`?** | The authoritative document-type list (§3.2). The ten values in this document are recovered from `if` branches. | `USE Anbar; SELECT * FROM FactorKind ORDER BY FK_ID;` |
| **Q18** | **What columns does `Anbar.dbo.Anbar` have?** | Subsystem B's posting-account table (§10.2). Only `A_Code`, `A_Aval`, `A_Kharid`, `A_Foroosh`, `A_BForoosh`, `A_Kasr`, `A_Maliat` are observed; `A_BKharid` may or may not exist. | `USE Anbar; SELECT name FROM sys.columns WHERE object_id=OBJECT_ID('Anbar');` |
| **Q19** | **Is `Moadian` ever populated, and by what?** | §10.4 — the tax e-invoicing integration is read-only from `arzi`. If nothing populates it, the `Send` column is always 0 and `Anbar_Jens.SSTID` is dead data. | `SELECT COUNT(*), MIN(M_id), MAX(M_id) FROM Moadian;` and ask the business which tool submits invoices |
| **Q20** | **How is `Tools.TFullDate` storing and converting Jalali dates?** | The source is **not in the repository** (`Tools.pas` is absent; only `Lib.inc` ships). The *stored* format is verifiable (`varchar(10)` `YYYY/MM/DD`) but the conversion algorithm, the leap-year rule and the behaviour of `Farsi_day := 31` on a 30-day month (§11.2.1) are not. | Decompile/inspect the compiled `Tools` unit, or obtain its source; **or** run the application and observe. Do not guess the leap rule — Jalali has competing 33-year and 2820-year rules and they disagree. |
| **Q21** | **What is the declared width of `Tools.TEditInt.IntValue`?** | If 32-bit, pistachio line totals above 2 147 483 647 rial silently wrap (§8.2.4). | same as Q20; or enter a >2.1 G rial lot in a test system |
| **Q22** | **What do `NR_P3`…`NR_P12` and `NR_Vazn3`…`NR_Vazn5` mean?** | Ten lab attributes and three deduction buckets on the weighbridge table that `arzi` never reads (§8.3.1). If subsystem B is absorbed they must be understood. | Obtain the weighbridge application's source, or its data dictionary; `SELECT TOP 100 * FROM Rppc_Solution.dbo.NewRamz` and infer from values |
| **Q23** | **Do the four `images\BACK_1..4.png` files exist on the deployed clients, and what do they look like?** | The printed invoice design is partly those PNGs (§13.14). They are not in the repository. | look in the deployment directory |
| **Q24** | **Is `FactorPrintU`'s layout still wanted?** | An entire unreachable print unit with four layouts (§13.15). | ask the business; compare with `FactorPrint3U`'s four forms |
| **Q25** | **Can one payment instrument settle several invoices?** | `DFish.S_LinkSSN` / `DCheck.S_LinkSSN` are scalar, so today it cannot (§9.7). Whether that is a business rule or an implementation limit changes the target schema. | ask the business |
| **Q26** | **Should production (15/25) and transfer (16/26) post to the ledger?** | Today they post nothing (§3.2.4, §10.5). Production consuming materials with no accounting entry is a real gap, not a design. | ask the business/accountant |

### 14.4 Low — nice to resolve, not blocking

| # | Question | How to answer |
|---|---|---|
| **Q27** | Does `TEditInt.Inttext` include the thousands separator (`IntSplitter = ','`)? It is interpolated straight into SQL at `AnbarFactorU.pas:620-621` and `Anbar_Amalkard.pas:178`. Since those queries evidently work, it must not — but confirm. | inspect the compiled `Tools` unit |
| **Q28** | Is `TEditDecimal.FloatValue` `Double` or `Extended`? Determines which quantities hit the truncation hazard of §7.3.4. | same |
| **Q29** | What is settings key `1012` (`Emza`, the signature block) and `1015` (the official-invoice footnote)? | `SELECT T_ID, T_Str FROM Tanzim WHERE T_ID IN (1012,1015);` |
| **Q30** | What is `FM_ID = 21`? It is referenced only as an unhandled case. | Q17's `FactorKind` query |
| **Q31** | Does `Anbar_AjnasView` join `Anbar_Vahed`, and what exactly is `AJ_PhiS`? | Q4's `OBJECT_DEFINITION` |
| **Q32** | What are `Sumr` and `ssn` in the `Anbar_CardJensi` result set? | Q2's `OBJECT_DEFINITION` |

### 14.5 Things this document states plainly rather than guessing

Recorded here so no later reader mistakes silence for omission:

- **The physical storage format of Jalali dates is `varchar(10)` `YYYY/MM/DD` and that *is*
  verifiable** — from the parameter declarations (`Anbar_MandehU.dfm:1602-1611`,
  `Dmu.dfm:453-496`) and from the string comparisons that depend on it. What is **not** verifiable
  is the conversion algorithm inside `Tools.TFullDate`, because that unit's source is not in the
  repository. Q20.
- **The debit/credit rules for subsystem A invoices are not derivable from source.** §10.1.2 is
  labelled as inference throughout. Q1.
- **`Anbar_Config`'s six account columns are written by one screen and read by nothing in the
  repository.** The inference that `Anbar_AddToFactor` consumes them is strong but is an
  inference. Q1.
- **The seven pistachio grades come from a source comment** (`FactorPesteh_U.pas:133`), not from
  the `Kinds` table. Q15.
- **`NR_Vazn3`/`NR_Vazn4`/`NR_Vazn5` are matched to moisture/blanks/other *by position only*.**
  The weighbridge application owns them and its source is unavailable. Q22.
- **Subsystem B's stock derivation does not exist in this repository at all** (§5.1.5). If it is
  absorbed, its rules must be recovered from the other application or re-specified.
- **No Persian string in this document is a guess.** Where a string could not be recovered it is
  not quoted. All Persian text here was decoded from Windows-1256 source or from `#NNNN` decimal
  escapes in `.dfm` files.


---

[← 13. Screen specifications (part c)](05-13-c-screen-specifications.md) | [index](00-index.md) | [15. PROPOSED IMPROVEMENTS (needs user approval) →](05-15-proposed-improvements.md)
