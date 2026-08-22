_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 8. Accounting integration

Read `docs/03-accounting-core.md` first: `Moein` holds voucher lines and is the system of record;
`DMoein` holds headers whose `DM_Mab` totals are drift-prone caches; the voucher state machine is
`0 → 1 → 2` on `M_Tx`; debit/credit balancing is enforced only on the `0 → 1` transition. Treasury
never validates its own balance — it relies entirely on that later check.

### 8.1 The link mechanism: `M_Id` + `M_Link`

Every treasury posting stamps two columns on each `Moein` row:

- **`M_Id`** — a constant identifying *which treasury event* produced the line (the "source module"
  in glossary terms; `M_Id > 0` also makes the line immutable from the voucher editor, see
  `docs/03-accounting-core.md` §3.4).
- **`M_Link`** — the primary key of the source row.

Idempotency is achieved by deleting on `(M_Id, M_Link)` and re-inserting, not by updating.

**`M_Id` registry for treasury:**

| `M_Id` | Event | `M_Link` points at | Assigned at |
|---|---|---|---|
| 21 | Cheque received | `DCheck.S_SSN` | `CheckDaryaftU.pas:333` |
| 22 | Cheque deposited to bank | `DCheck2.S_SSN` | `CheckDaryaft2U.pas:209, 221` |
| 22 | Cheque bounced back from bank | `DCheck2.S_SSN` | `CheckBargashtu.pas:229, 241` |
| 23 | Cheque collected / cleared | `DCheck2.S_SSN` | `CheckVosoolU.pas:241, 253` |
| 24 | Cheque returned to issuer | `DCheck2.S_SSN` | `CheckEsterdadU.pas:208, 220` |
| 25 | Bank deposit slip | `DFish.S_SSN` | `FISHDaryaftU.pas:469, 477` |
| 26 | Issued-cheque payment batch | `CheckMaster.CM_SSN` | `CheckEditU.pas:486` |
| 41 | Petty-cash expense claim | `TankhahMaster.TM_SSN` | `TankhahEdit.pas:463` |

**Trap 1 — `M_Link` changes meaning between id 21 and ids 22/23/24.** For a *receipt* it is the
cheque id; for every *subsequent* cheque event it is the event id in `DCheck2`. Any query that tries
to find "all postings for cheque X" must therefore union `(M_Id=21 AND M_Link=X)` with
`(M_Id IN (22,23,24) AND M_Link IN (SELECT S_SSN FROM DCheck2 WHERE S_Link=X))`. No code in the
repository does this — nothing ever reconstructs a cheque's full posting history.

**Trap 2 — deposit and bounce share `M_Id = 22`,** so they are distinguishable only via `DCheck2`.

**Trap 3 — the comment block at the top of `FishListD.pas:1-4` is wrong.** It claims:

```
///  Moein.M_ID = 21   daryaft check mojood dar sandoogh ya bank .
///  Moein.M_ID = 22 bargast check be saheb hesab .
///  Moein.M_ID = 23 vosool check .
/// Moein.M_ID = 24   varize fish.
```

In the live code 22 is *deposit-to-bank + bounce*, 24 is *return to issuer*, and the deposit slip is
25, not 24. Do not port that comment.

### 8.2 Voucher allocation — one shared voucher per Jalali day

Every treasury screen calls `Dm.Get_NewSanad_DateID(<jalali date>, '21,22,23,24,25,26,27,28,29')`
(`CheckDaryaftU.pas:269`, `CheckDaryaft2U.pas:177`, `CheckVosoolU.pas:174`,
`CheckBargashtu.pas:198`, `CheckEsterdadU.pas:176`, `FISHDaryaftU.pas:375`, `CheckEditU.pas:409`).
Petty cash uses a different band: `'41,42,43,44,45,46,47,48,49'` (`TankhahEdit.pas:388`).

`Get_NewSanad_DateID` (`Dmu.pas:1461-1477`):

```sql
Select isnull( Max(M_Sanad) , 0 ) as S
  From Moein
  Where M_Tx=0 and M_Coid=<CO_ID>
  and M_ID in( <IDList> )
  and M_Date=<F_Date>
```

If that returns `> 0` it reuses that voucher number; otherwise it falls back to `New_Sanad` (a fresh
number). Consequences:

- **All treasury documents dated the same Jalali day land in one voucher.** A receipt, a deposit, a
  collection and an issued-cheque batch on `1403/05/12` share one `M_Sanad`.
- **Petty cash gets its own daily voucher**, because the id band is disjoint.
- **Once a day's voucher leaves draft (`M_Tx > 0`), the next document on that date opens a *new*
  voucher** — the `M_Tx=0` predicate excludes it. A single date can therefore accumulate several
  vouchers over time.
- Editing an existing document **reallocates** its voucher number from its date rather than keeping
  the original. The old voucher's cached total is then repaired with `Dm.Dmoein_UpdateMab(_OldSanad)`
  and the new one built with `Dm.DMoein_Make(...)`.

`DMoein_Make` descriptions used by treasury: `'عملیات خزانه'` "treasury operations"
(`CheckDaryaftU.pas:361`); `'عملیات خزانه مورخ ' + <date>` "treasury operations dated D"
(`CheckDaryaft2U.pas:227`, `CheckBargashtu.pas:247`, `CheckEsterdadU.pas:226`,
`FISHDaryaftU.pas:488`); the document's own `CM_Desc` / `TM_Desc` for the two batch screens
(`CheckEditU.pas:516`, `TankhahEdit.pas:494`).

### 8.3 Treasury event → voucher posting

Amounts are always in rials. "Bes"/"Bed" in the *column* names of `DCheck` describe the **receipt**
posting only; later events reuse those accounts on the opposite sides, so read the table, not the
column names.

| # | Event | Screen | `M_Id` | `M_Link` | Debit account | Credit account | Amount source | Narration (`Article`) |
|---|---|---|---|---|---|---|---|---|
| 1 | **Cheque received** | `CheckDaryaftU` | 21 | `DCheck.S_SSN` | `DCheck.S_BedSSN` — notes receivable on hand, default `Sandoogh_K-Sandoogh_M-<payer Ta1>` | `DCheck.S_BesSSN` — the payer | `DCheck.S_Mab` | `'بابت دریافت چک شماره '+S_CheckNo+' مبلغ '+dbo.Noto3(S_Mab)+' سررسید '+S_DateS+' توسط '+S_BesName` (`CheckDaryaftU.pas:324-325`) |
| 2 | **Cheque deposited to bank** | `CheckDaryaft2U` | 22 | `DCheck2.S_SSN` | notes in course of collection, `Jaryan_K-Jaryan_M-<payer Ta1>` (operator-overridable) | the account from step 1's debit side (`DCheck.S_BedSSN`) | `DCheck.S_Mab` | `'بابت واگذاری به بانک چک شماره '+…` (`CheckDaryaft2U.pas:181-182`) |
| 3 | **Cheque bounced** | `CheckBargashtu` | 22 | `DCheck2.S_SSN` | notes on hand, `Sandoogh_K-Sandoogh_M-<payer Ta1>` | notes in collection, `Jaryan_K-Jaryan_M-<payer Ta1>` | `DCheck.S_Mab` | `' بابت برگشت از بانک چک شماره '+…+' به بانک'` (`CheckBargashtu.pas:201-202`) |
| 4 | **Cheque collected** | `CheckVosoolU` | 23 | `DCheck2.S_SSN` | **the bank account the operator picks** (`S_Bank`, remembered in the INI) | notes in collection, `Jaryan_K-Jaryan_M-<payer Ta1>` | `DCheck.S_Mab` | `' بابت وصول چک شماره '+no+' '+payer+' '+note` (`CheckVosoolU.pas:216`) |
| 5 | **Cheque returned to issuer** | `CheckEsterdadU` | 24 | `DCheck2.S_SSN` | `DCheck.S_BesSSN` — the payer | `DCheck.S_BedSSN` — notes receivable on hand | `DCheck.S_Mab` | `'بابت استرداد چک شماره '+…+' به '+payer` (`CheckEsterdadU.pas:180-181`) |
| 6 | **Deposit slip / incoming transfer** | `FISHDaryaftU` | 25 | `DFish.S_SSN` | `DFish.S_BankSSN` — the receiving bank account | `DFish.S_BesSSN` — the payer | `DFish.S_Mab` | debit line: `'بابت '+<method>+' شماره '+S_FishNo+' توسط '+payerName+' '+S_Desc`; credit line: same but `' به '+bankName` (`FISHDaryaftU.pas:461-464`) — note the two strings are **swapped relative to their sides** (see §8.5) |
| 7 | **Issued-cheque batch** | `CheckEditU` | 26 | `CheckMaster.CM_SSN` | one line **per** `CheckDetail` row: `CD_Bed` (the payee), amount `CD_Mab` | one single line: `CM_Code` (the bank account), amount `CM_Mab` | line: `CD_Mab`; credit: `CM_Mab` = grid footer sum | per payee line: `CD_Desc` verbatim; credit line: `CM_Desc+' تعداد '+<n>+'  نفر '` "…, count N persons" (`CheckEditU.pas:494, 502`) |
| 8 | **Petty-cash expense claim** | `TankhahEdit` | 41 | `TankhahMaster.TM_SSN` | one line **per** `TankhahDetail` row: `TD_Bed` (the expense account), amount `TD_Mab` | one single line: `TM_Code` (the petty-cash custodian account), amount `TM_Mab` | line: `TD_Mab`; credit: `TM_Mab` = grid footer sum | per line: `TD_Desc` verbatim; credit line: `TM_Desc+' تعداد '+<n>+'  نفر '` (`TankhahEdit.pas:471, 480`) |

Rows 7 and 8 are the only many-to-one postings; rows 1-6 are always exactly two lines.

Common `Moein` column values for treasury lines: `M_Kind = 1`, `M_Ted = 0` (no quantity),
`M_Tx = 0` (created in draft), `M_Coid = Dm.CO_ID`, `M_User = Dm.userId`,
`M_Code = <Sarfasl.S_SSN>`, and `M_Ko/M_Mo/M_Ta1/M_Ta2` denormalised from that account.

### 8.4 Two different ways the account hierarchy gets denormalised onto the line

- **Set-based** (`CheckDaryaftU.pas:332-336`, `FISHDaryaftU.pas:468-471`): the `INSERT … SELECT`
  does `left join sarfasl as s on S.S_SSN = <the account column>` and copies `S_Ko, S_Mo, S_Ta1,
  S_Ta2` inline. A `left join` means a missing account silently yields NULL levels rather than
  failing.
- **Client-side** (`CheckDaryaft2U.pas:201-210`, `CheckVosoolU.pas:232-242`,
  `CheckBargashtu.pas:221-230`, `CheckEsterdadU.pas:200-209`): the Delphi code calls
  `Dm.Sarfasl_SSN_CODEName(<id>)` to position the shared `Dm.Sarfasl` dataset, then string-pastes
  `Dm.Sarfasl.FieldByName('S_Ko').AsString` etc. straight into the SQL text. If the lookup misses,
  the previous row's values are pasted in — a silent mis-posting.
- **Post-hoc UPDATE** (`CheckEditU.pas:506-511`, `TankhahEdit.pas:484-489`): the batch screens insert
  the lines with `M_Ko/M_Mo/M_Ta1/M_Ta2 = 0` and then run
  `Update Moein Set Moein.M_Ko=sarfasl.S_Ko, … from sarfasl Where S_SSN=Moein.M_Code and
  Moein.M_Coid=<CO_ID> and Moein.M_Sanad=<the voucher>`. **This updates every line of the shared
  daily voucher, not just this document's lines** — harmless as long as the mapping is
  idempotent, but it means saving a cheque batch rewrites hierarchy columns on unrelated treasury
  lines that happen to share the voucher.

The rebuild should do this once, in one place, from a foreign key.

### 8.5 Defects in the accounting integration

1. **The collection posting never builds or refreshes a voucher header.** `CheckVosoolU` is the only
   treasury screen with no `Dm.DMoein_Make` / `Dm.Dmoein_UpdateMab` call
   (`CheckVosoolU.pas:258-261`). A collection into a brand-new voucher number leaves `Moein` lines
   with no `DMoein` header; a collection into an existing voucher leaves `DM_Mab` stale.
2. **`FISHDaryaftU`'s two narration strings are attached to the wrong sides.** `@SBed` is built with
   `' توسط ' + S_BesN` ("by <payer>") and is applied to the **bank debit** line; `@SBes` is built with
   `' به ' + S_BedN` ("to <bank>") and is applied to the **payer credit** line
   (`FISHDaryaftU.pas:461-464` vs `:466-480`). Cosmetic, but it makes the daybook read backwards.
3. **`Dm.Dmoein_UpdateMab(_OldSanad)` is called with the *new* voucher number in `CheckDaryaft2U`**
   (`:228` passes `S_Sanad2.IntValue`, the freshly allocated one) while the other screens pass the
   old one. Inconsistent, and the deposit screen therefore never repairs a voucher it moved lines
   off.
4. **Deletes are partial.** `CheckDaryaftU.Delete_Check` removes `M_Id=21` lines only, leaving every
   `M_Id ∈ {22,23,24}` line and every `DCheck2` row orphaned; and it is unreachable anyway (§2.3 T3).
   `FISHDaryaftU.Delete_Fish` (`:159-188`) and `CheckListU.B_DeleteClick` (`:155-203`) do delete
   their own lines, but neither repairs `DMoein` for a voucher that still holds other documents —
   `CheckListU.pas:193` does call `Dmoein_UpdateMab`, `FISHDaryaftU.Delete_Fish` does not.
5. **`TDM.MakeSanad_FishVariz` (`Dmu.pas:341-360`) is an empty stub.** It loads the `DFish` row, then
   the two commented section headers `/// جستجوی سند و حذف` ("find and delete the voucher") and
   `/// ایجاد سند` ("create the voucher") are followed by nothing. Likewise the commented-out
   `Exec MakeSanad_CheckDaryafti` call at `CheckDaryaftU.pas:353-358`. Both are remnants of an
   abandoned move of posting logic into stored procedures; the inline SQL is what actually runs.
6. **No posting is transactional across its two statements in the Delphi sense.** The screens emit
   `Begin Transaction … Commit` pairs inside a single `ExecSQL` batch, but the `DCheck` state update
   and the `Moein` insert are in *separate* transactions in the same batch
   (`CheckDaryaft2U.pas:187-224`), so a failure in the second leaves the cheque in the new state with
   no accounting entry.


---

[← 7. Petty cash (Tankhah)](06-07-petty-cash-tankhah.md) | [index](00-index.md) | [9. Validation rules →](06-09-validation-rules.md)
