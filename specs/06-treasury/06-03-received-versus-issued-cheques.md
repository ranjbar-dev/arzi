_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 3. Received versus issued cheques

### 3.1 There is no discriminator column — they are different tables

The system does **not** distinguish received from issued cheques with a flag. They are two entirely
separate subsystems that share nothing but the word "Check" in their unit names:

| | Received cheque | Issued cheque |
|---|---|---|
| Tables | `DCheck` + `DCheck2` | `CheckMaster` + `CheckDetail` |
| Column prefix | `S_*` | `CM_*` / `CD_*` |
| Entry screen | `CheckDaryaftU` (`TCheckDaryaftF`) | `CheckEditU` (`TCheckEditF`) + `CheckEditAddU` |
| List screen | `CheckListDU` (`TCheckListDF`) | `CheckListU` (`TCheckListF`) |
| Menu entry | `Mainu.pas:524` → `CheckListDF.init` | `Mainu.pas:611` → `CheckListF.init` |
| Report title | `لیست چکهای سررسید شده…` etc. (`CheckListDU.pas:261-267`) | `لیست چکهای صادره` "list of issued cheques" (`CheckListU.pas:151`) |
| Lifecycle | 5-state machine (§2) | **none** |
| Event log | `DCheck2` | none |
| `M_Id` | 21 / 22 / 23 / 24 | 26 |
| Cardinality | one cheque per row | one **batch** header + N payee lines |
| Permission keys | 2102-2109 | 2111-2114, 2125 |

The "D" in `DCheck` / `CheckListDU` stands for *Daryaft* (received). `CheckListU` without the D is
the issued side. This is the only naming signal, and it is easy to misread.

### 3.2 What an "issued cheque" actually is here

`CheckMaster` is **not** one cheque. It is a *bank payment list* — the covering document a company
hands its bank saying "debit our account and pay these N people these N amounts". Evidence:

**Confirmed by the DDL** (`Full_Script_14050527.sql`, schema-only, no rows — see
`02-data-model/02-12-a.md` §12.5): `CheckMaster`'s full column list is `CM_SSN, CM_Coid, CM_No,
CM_Sanad, CM_Date, CM_Mab, CM_Desc, CM_Tittle, CM_Code, CM_CodeCR, CM_CodeName, CM_Count,
CM_UserID` — exactly one bank-account reference (`CM_Code`) and one amount/count pair per header
row, exactly as the "one posting, N payees" reading predicts. `CheckDetail`'s full column list is
`CD_SSN, CD_Coid, CD_CMSSN, CD_Bed, CD_BedCR, CD_BedName, CD_Mab, CD_Desc, CD_BankNo, CD_Jari` —
`CD_CMSSN` is the master-detail link back to `CheckMaster.CM_SSN`, unenforced (no `FOREIGN KEY`, no
index on either side — confirmed absent from the whole schema). **Neither table has a primary key
at all** — `CM_SSN` and `CD_SSN` are `IDENTITY` columns with no `PRIMARY KEY` constraint, matching
the pattern that nothing in this legacy schema enforces uniqueness by default. This is corroborating
schema-shape evidence for "batch", not a row-count proof — the dump has zero data, so it cannot say
whether `CM_Count` is usually 1 (batch-of-one in practice) or usually >1 in this business's actual
usage. `06-treasury/06-12-open-questions.md` Q7 / `11-open-decisions.md` A9 track the remaining,
data-only half of this question.

- The header has exactly one bank account (`CM_Code`) and the posting credits it once for the total
  (`CheckEditU.pas:500-504`).
- Each detail line has a payee account (`CD_Bed`), an amount (`CD_Mab`), a free-text bank account
  number (`CD_BankNo`) and a second banking reference (`CD_Jari`) — the fields a bank needs to make
  a transfer, not the fields a cheque needs.
- The credit-line narration is `<desc> + ' تعداد ' + N + '  نفر '` — "…, count N **persons**"
  (`CheckEditU.pas:502`).
- `CM_Tittle` is a three-line covering-letter body rendered by report `RP2`
  (`CheckEditU.pas:340-348`), and `CheckEditAddU.dfm:236` carries the caption
  `ذخیره کد بانک به عنوان پیش فرض` "save the bank code as default".
- **Individual issued cheques carry no number.** There is no cheque number on a `CheckDetail` line
  at all; the only number is the free-text `CM_No` on the header.

So the module is closer to a payroll/payables disbursement run than to cheque issuance. If the
business really does write physical cheques from it, the cheque numbers are not captured anywhere.

### 3.3 Where the handling differs

| Aspect | Received (`DCheck`) | Issued (`CheckMaster`) |
|---|---|---|
| **Direction of the posting** | Debit an asset (notes receivable), credit the counterparty | Debit the counterparties, credit the bank |
| **Number of postings over the document's life** | up to 3 (receipt, then one per transition) | exactly 1, rewritten in place on edit |
| **Voucher line count** | always 2 | `CM_Count + 1` |
| **Due date** | `S_DateS`, drives lists and aging (§5) | **absent** — an issued cheque has no due date field of any kind |
| **State** | `S_State` 1-5 | none; the document is either saved or deleted |
| **Bank identity** | not recorded (only the internal notes-receivable account) | `CM_Code` = the internal bank account; the payee's external account number is `CD_BankNo` |
| **Edit lock** | `S_State = 1` **and** `S_COID = Dm.CO_ID` **and** voucher in draft | voucher in draft only (`Max(DM_TX) = 0`, `CheckListU.pas:218-227`) — **no fiscal-year check and no state check** |
| **Delete** | dead code, no-op (§2.3 T3) | works: deletes header, lines and `M_Id=26` postings in one batch, then `Dmoein_UpdateMab` (`CheckListU.pas:185-193`) |
| **Delete guard** | `QCheckBeforeDelete` `Abort` on the dataset (§9.3) | none — `CheckMaster` is not one of the five guarded datasets |
| **Line editing** | n/a | in-memory `TVirtualTable CD1`; lines are deleted and re-inserted wholesale on save (`CheckEditU.pas:447`) |
| **Read-only "View" mode** | not available | `TCheckEditF.View(SSN)` disables the whole panel and enables only printing (`CheckEditU.pas:258-273`) |
| **Printing** | one report, `RP1`, list-level (`CheckListDU.pas:256-272`) | two reports, `RP1` (payment list) and `RP2` (covering letter with `CM_Tittle`), plus a dead `RP1_Old` (`CheckEditU.pas:59`) and a list-level `RP3` |
| **Linkage to source documents** | `S_linkPrg` / `S_LinkSSN`; can be created from an invoice via `New_From_PRg` | none — always entered by hand |
| **Fiscal-year filter on the list** | none (`Select * From DCheck`, `CheckListDU.pas:323`) — **the list shows every year at once** | `Where CM_Coid=:Coid` (`CheckListU.dfm:517`) — current year only |

### 3.4 Two consequences worth flagging for the rebuild

1. **The received-cheque list is not year-scoped.** `CheckListDU.ReopenQ1` builds
   `Select * From DCheck Where 1=1 …` with no `S_COID` predicate (`CheckListDU.pas:323-329`). The
   grid therefore shows cheques from every fiscal year in the database, and the year guards
   (§9.2 rules 15, 17, 19, 24) exist precisely to stop the operator acting on the wrong year's rows
   from that mixed list. The issued-cheque list has the opposite design.
2. **The two subsystems can post to the same voucher.** Both call `Get_NewSanad_DateID` with the
   `'21…29'` id band (`CheckDaryaftU.pas:269`, `CheckEditU.pas:409`), so a day's received cheques and
   its issued-cheque batches share one `M_Sanad`. Deleting an issued batch calls
   `Dmoein_UpdateMab` on that shared voucher (`CheckListU.pas:193`), which is correct, but editing a
   batch runs the blanket `Update Moein Set M_Ko=… Where M_Sanad=<shared voucher>`
   (`CheckEditU.pas:508-511`) across the received cheques' lines too (§8.4).

### 3.5 Notes payable

Despite `Asnad Pardakhtani` (notes payable) appearing in the glossary and `BN_AsnadCode`
(`کد حساب اسناد پرداختنی`, "notes-payable account code") appearing on the inert `BankTanzim` form
(`BankTanzim.dfm:174`), **no notes-payable subsystem exists**. Issued cheques are expensed straight
against the bank account on the day the batch is saved; nothing tracks an outstanding own-cheque
until it clears. There is no issued-cheque equivalent of `Vosool`, `Bargasht` or `Esterdad`.


---

[← 2. The cheque state machine](06-02-cheque-state-machine.md) | [index](00-index.md) | [4. Endorsement / transfer to a third party →](06-04-endorsement-transfer-third-party.md)
