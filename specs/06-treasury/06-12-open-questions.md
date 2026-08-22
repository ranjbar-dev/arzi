_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 12. Open questions

Ordered by how much they block the rebuild. Each needs an answer from the customer or from the live
database before the corresponding table or endpoint can be designed.

### Blocking — cannot design the schema without an answer

1. **~~Does the `BN_*` bank table exist~~ — RESOLVED: no.** A schema-only dump
   (`Full_Script_14050527.sql`, SQL Server, 39 `CREATE TABLE` statements, 0 rows) was reviewed
   table-by-table (`02-data-model/02-12-a.md` §12.5, `02-12-b.md` §12.13). No `BN_*` table, or
   anything with those column names, exists anywhere in it. `BankTanzim`'s grid describes a table
   this database does not have. §11's bank modelling is entirely new — confirmed, not inferred.

2. **Partially resolved.** The dump confirms the *physical type and length* of every date column
   named here: `DCheck.S_Date varchar(10)` / `S_DateS varchar(50)`, `DFish.S_Date varchar(10)` /
   `S_DateS varchar(50)` (see item 3), `CM_Date varchar(10)`, and — a new finding —
   `TankhahMaster.TM_Date varchar(50)`, **not `varchar(10)` like every other business-date column**.
   That's a real, previously-undocumented inconsistency worth its own line: either `TM_Date` holds
   something other than a plain `YYYY/MM/DD` string, or it was declared wide "just in case" and
   never tightened. **Still open:** whether the *values* are consistently zero-padded — this dump
   has no rows (0 `INSERT` statements), so `SELECT DISTINCT LEN(...)` cannot be run yet. See
   `02-data-model/02-12-a.md` §12.1 for the now-known server-side conversion algorithm
   (`dbo.Farsi_Date`), which bears directly on this question.

3. **RESOLVED — yes, `DFish.S_DateS` physically exists.** Full `DFish` column list from the dump:
   `S_SSN, S_COID, S_State, S_StateName, S_FishNo, S_Sanad, S_Date varchar(10), S_DateS varchar(50),
   S_Mab, S_Desc, S_BesSSN varchar(200), S_BesCR, S_BesName, S_BankSsn, S_BankCR, S_BankName,
   S_UserID, S_LinkPRG, S_linkSSN`. It is the same `varchar(50)` shape as `DCheck.S_DateS` and
   `TCheck.S_DateS` — a consistent (if oversized) pattern across all three cheque/fish-adjacent
   tables, not a one-off. What it's *populated with*, and by what, still needs data — the dump
   proves the column is real, not what's in it or who else writes it.

4. **RESOLVED — `DCheck2.S_BedSSN` is `varchar(200)`.** Full `DCheck2` column list:
   `S_SSN, S_Link int NOT NULL, S_COID, S_Sanad, S_Date varchar(10), S_Mab, S_State, S_StateName,
   S_BedSSN varchar(200), S_BesSSN int, S_Desc, S_UserID`. **Confirmed asymmetric**: `S_BedSSN` is
   text, `S_BesSSN` right beside it is a real `int`, in the same table, same apparent role
   (counterparty SSN reference). This is a genuine schema wart, not a Delphi-metadata artefact —
   the FK-shape target for `S_BedSSN` should be modelled as needing a `CAST`/cleanup step during
   migration, not assumed to already be clean numeric text.

5. **Still open — needs data.** No rows in this dump; `S_State=3`'s historical presence in `DCheck`
   or `DCheck2` cannot be checked without a populated database.

6. **Still open — needs data.** Same reason; `S_Zssn`/`S_ZCR`/`S_ZName` population cannot be
   checked without rows. The columns' existence is confirmed (`DCheck.S_Zssn int, S_ZCR
   varchar(50), S_ZName varchar(100)`), which was never in doubt.

7. **Narrowed, not closed — the DDL supports "batch" but the row-count distribution still needs
   data.** `CheckMaster`'s full column list (`CM_SSN, CM_Coid, CM_No, CM_Sanad, CM_Date, CM_Mab,
   CM_Desc, CM_Tittle, CM_Code, CM_CodeCR, CM_CodeName, CM_Count, CM_UserID`) has exactly **one**
   bank-account reference (`CM_Code`) and **one** count/total pair (`CM_Count`/`CM_Mab`) per header
   row, with `CheckDetail` (`CD_SSN, CD_Coid, CD_CMSSN, CD_Bed, CD_BedCR, CD_BedName, CD_Mab,
   CD_Desc, CD_BankNo, CD_Jari`) as an unenforced (no FK, no index — `02-data-model/02-12-a.md`
   §12.6) master-detail child via `CD_CMSSN`. This is structurally a header-with-N-lines shape,
   which is what "batch" predicts and what "one row per physical cheque" does not — a single-cheque
   design would not need a separate detail table with its own count column. **This is corroborating
   schema evidence for the existing "batch" reading, not proof**: the DDL cannot show whether
   `CM_Count` is *usually* 1 (in which case "batch" is technically true but practically always a
   batch-of-one, i.e. behaves like a single cheque anyway) or usually >1 (a real batch in practice).
   `SELECT CM_Count, COUNT(*) FROM CheckMaster GROUP BY CM_Count` from the original question is
   still exactly the query needed, and this dump cannot run it (0 rows). See
   `11-open-decisions.md` A9.

### Business rules that must be decided, not inferred

8. **Should a received cheque's due date be constrained?** Today it is validated not at all (§9.1).
   Must it be ≥ the receipt date? Must it lie inside the fiscal year? (It usually cannot — cheques
   routinely fall due after year end.)

9. **Must cheque numbers be unique?** And unique per what — globally, per drawer, per bank? Today
   there is no uniqueness anywhere in treasury (§9.8). The same question applies to `S_FishNo`,
   `CM_No` and `TM_No`.

10. **Should the bounce path record a reason, a certificate number (گواهی عدم پرداخت) and a bank
    charge?** None of the three exists today (§11.4). All three are normal Iranian practice.

11. **Is the "undo collection" button (`S_DVosool`, permission 2108) a required feature?**
    It was designed, permissioned and left unimplemented (§2.3 T9). Someone specified it.

12. **Should endorsement be built?** See §4.5 — the schema was prepared for it and abandoned.

13. **Should a real petty-cash fund exist** (float amount, custodian, advance and replenishment
    documents, running balance), or is the current "claims only, balance implied by the ledger"
    model what the business wants? (§7.1, §7.5.)

14. **What is the intended relationship between a `DFish` deposit slip and the cheques deposited that
    day?** Today there is none (§6.1). If the bank's paying-in slip actually lists several cheques,
    the rebuild needs a slip→cheque link that the legacy system never had.

15. **Which of the documented defects are load-bearing bugs the business has worked around, and which
    should be fixed on port?** Specifically: the bounce writing `S_State=2` into `DCheck2`
    (§2.1); the collection posting never building its voucher header (§8.5 defect 1); the
    `FISHDaryaftU` narrations being on the wrong sides (§8.5 defect 2); the blanket
    `Update Moein … Where M_Sanad=<shared voucher>` (§8.4). Default per the brief is
    **port-as-is**; each of these produces visibly wrong output, so each needs an explicit decision.

### Lower priority

16. **What are `M_Id` values 27, 28, 29 and 42-49 reserved for?** They appear only inside the
    `Get_NewSanad_DateID` id lists (§8.2) and are never written. Was something planned?

17. **Are permissions 2115-2120 used elsewhere?** The treasury bands are 2102-2109, 2111-2114, 2121-2125
    with a gap. See `docs/08-platform-and-security.md`.

18. **Why do received-cheque and deposit-slip entry share permissions 2102/2103/2104?** (§6.5.) Is
    that intentional, or was `FishListD` copied from `CheckListDU` without changing the keys?

19. **`CheckDaryaftU` computes the default notes-receivable account two different ways** —
    `Dm.SanDoogh_kM + '-' + Ta1` on text change (`:187`) but the hard-coded `'108-1-' + Ta1` from the
    picker button (`:401`). Which is correct? Is `108-1` always equal to `Sandoogh_KM`?

20. **`TankhahEdit`'s report has the customer's name baked into the form resource**
    (`شرکت تعاونی تولید کنندگان پسته رفسنجان`, §11.13). Is `arzi` deployed to more than one
    organisation? If so, that report is wrong for all but one of them.

21. **RESOLVED — yes, `TCheck` is real, with full DDL now captured**: `S_SSN` identity,
    `S_COID, S_State, S_StateName, S_CheckNo, S_Sanad, S_Date varchar(10), S_DateS varchar(50),
    S_Mab, S_Desc, S_BankSSN, S_BankCR, S_BankName, S_BedSSN varchar(200), S_BedCR, S_BedName,
    S_Asnadssn, S_AsnadCR, S_AsnadName, S_UserID`. An extended property on `S_State`, read directly
    from the database schema (no data needed), gives the full code list: `1=check naghdi` (cash
    cheque), `2=check moedi`, `3=bardasht naghdi` (cash withdrawal), `4=bardasht ba kart` (card
    withdrawal), `11=daryaft check` (cheque received), `12=variz ba fish ya naghdi` (deposit by
    slip/cash), `13=variz ba kartkhan` (deposit by card reader). **This is a broader cash/bank
    movement-type table, not a cheque-specific one** — worth re-reading as a general treasury
    transaction log rather than folding it into `cheques` in §11. Whether it's legacy or vestigial
    (i.e. still populated) still needs data.

22. **RESOLVED — yes, `dbo.Noto3` exists; full body captured.** `CAST(@INP AS varchar(20))` then
    four `IF Len(@Re)>N` steps insert a comma every 3 digits from the right. No rounding logic (the
    input is `bigint`, already whole rial — nothing to round). No sign-aware branch: for a negative
    number the leading `-` just shifts the grouping boundary by one character versus the
    same-magnitude positive value — a minor, real formatting quirk, not a crash risk. Comparing this
    against `TDM.inttoStr3` (`Dmu.pas:859-867`) to see if they group identically is now a
    straightforward source-to-source diff.

23. **RESOLVED — yes, `MakeSanad_CheckDaryafti` exists on the server; full body captured.** Guards:
    the cheque must exist and be `S_State = 1` ("در صندوق"); refuses if a draft posting already
    exists under `M_Id=21` for this cheque (must be finalized first); deletes any prior `M_Id=21`
    lines for this `S_SSN`, then inserts exactly 2 `Moein` rows (debit `S_BedSSN`, credit `S_ZSSN`),
    narration built from `dbo.Noto3(S_Mab)` and `S_DateS`. Confirms the "2 postings per cheque
    receipt" claim in `06-03-received-versus-issued-cheques.md` at the database level.


---

[← 11. Screen specifications (part c)](06-11-c-screen-specifications.md) | [index](00-index.md) | [13. PROPOSED IMPROVEMENTS (needs user approval) →](06-13-proposed-improvements.md)
