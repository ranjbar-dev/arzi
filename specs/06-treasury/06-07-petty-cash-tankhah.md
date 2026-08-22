_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 7. Petty cash (Tankhah)

### 7.1 What actually exists

One document type: the **petty-cash expense claim** (`TankhahMaster` + `TankhahDetail`), entered on
`TankhahEdit` / `TankhahEditAddu` and listed on `TankhahList`. That is the whole module.

**None of the following exists anywhere in the codebase:**

- fund setup / imprest float definition — no table, no screen, no float amount;
- a custodian register — the custodian is just whichever `Sarfasl` leaf account the operator picks
  as `TM_Code`, and nothing links a person to a fund;
- an advance / cash-issue document — money going *out* to the custodian is not modelled;
- a replenishment document — nothing tops a fund back up;
- a balance formula, a running balance field, or any query that computes one;
- any status, approval step or closing action on a claim.

Grep confirms it: the only occurrences of `Tankhah` in the repository are the three units, their
`.dfm`s, and `Mainu.pas:289`'s `uses` clause.

### 7.2 The claim document

A claim is structurally identical to the issued-cheque batch (§3.2), with `Check`→`Tankhah`,
`CM_`→`TM_`, `CD_`→`TD_`, `M_Id = 26`→`41`, and two columns dropped:

> On date D, custodian C is reimbursed / relieved of total M, made up of N expense lines, each line
> charging an expense account.

Header (`TankhahMaster`), fields as in §1.6. Lines (`TankhahDetail`): expense account + amount +
description. Nothing else.

The header total `TM_Mab` is **not entered** — `Set_Sum` (`TankhahEdit.pas:232-236`) recomputes it
from the grid footer every time a line is added, edited or deleted, and the save writes that value.
The line count `TM_Count` is likewise `CD1.RecordCount` at save time (`TankhahEdit.pas:415`).

### 7.3 The one posting

`M_Id = 41`, `M_Link = TankhahMaster.TM_SSN`, `M_Kind = 1`, `M_Tx = 0`
(`TankhahEdit.pas:462-482`):

| Side | Account | Amount | Narration |
|---|---|---|---|
| **Debit**, one line per `TankhahDetail` row | `TD_Bed` — the expense account | `TD_Mab` | `TD_Desc` verbatim (`TankhahEdit.pas:471`) |
| **Credit**, one single line | `TM_Code` — the petty-cash custodian's account | `TM_Mab` (the footer sum) | `TM_Desc + ' تعداد ' + N + '  نفر '` — "…, count N persons" (`TankhahEdit.pas:480`) |

Note the credit-line narration says "N **persons**" while the lines are expense accounts, not
people — copy-paste residue from `CheckEditU.pas:502`, where it was correct.

Idempotency: `Delete moein Where M_Coid=<CO_ID> and M_Id=41 and M_Link=<TM_SSN>`
(`TankhahEdit.pas:453-456`) — note the source comment `// هوشمندانه !!!` ("clever!!!"), which marks
the deliberate omission of `M_Sanad` from the predicate so that lines left on a *previously*
allocated voucher are also removed. The same comment and the same trick appear at
`CheckEditU.pas:477-478`.

Account-hierarchy denormalisation is the **post-hoc UPDATE** variant (§8.4):
`Update Moein Set M_Ko=sarfasl.S_Ko, … from sarfasl Where S_SSN=Moein.M_Code and
Moein.M_Coid=<CO_ID> and Moein.M_Sanad=<voucher>` (`TankhahEdit.pas:484-489`) — which, as with the
cheque batch, rewrites those columns on **every** line of the shared voucher.

Finally `Dm.Dmoein_UpdateMab(_OldSanad)` (if editing) and
`Dm.DMoein_Make(TM_Sanad, TM_Date, TM_Desc)` (`TankhahEdit.pas:493-494`) — note that unlike every
cheque screen, the voucher header description is the **document's own description**, not
`'عملیات خزانه'`.

### 7.4 Voucher band

Petty cash allocates from a **different** id band than the rest of treasury:

```pascal
TM_Sanad.IntValue := Dm.Get_NewSanad_DateID( TM_Date.Farsi_Date, '41,42,43,44,45,46,47,48,49' );
```
(`TankhahEdit.pas:388`)

Only `41` is ever written, so effectively: **all petty-cash claims dated the same Jalali day share
one voucher, separate from the day's treasury voucher.** Ids 42-49 are reserved and unused.

### 7.5 The balance formula

There isn't one. The custodian's petty-cash balance is whatever the general ledger reports for the
`TM_Code` account: sum of debits minus credits over that `Sarfasl` leaf, obtained through the normal
ledger/trial-balance reports in `docs/03-accounting-core.md`. Because *advances to the custodian*
are not a treasury document, they must be entered as manual vouchers or as issued-cheque batches
(`CheckEditU`, where the custodian would be a payee line). The petty-cash module only ever **credits**
the custodian; nothing in it ever debits them.

Practical reading of the accounting shape:

```
custodian account balance
  = (money given to the custodian — recorded elsewhere: manual voucher, cheque batch, or DFish)
  − Σ TankhahMaster.TM_Mab for that custodian
```

Nothing computes, displays or validates this. There is no check that a claim exceeds the float, no
warning at a low balance, and no reconciliation screen.

### 7.6 Screens and permissions

**`TankhahEdit` (`TTankhahEditF`)** — claim editor, three modes:

| Mode | Entry point | Behaviour |
|---|---|---|
| New | `New` (`:170-187`) | blank form, all buttons enabled, printing disabled |
| Edit | `Edit(SSN)` (`:188-205`) | `LoadSSN` then editable; the draft-state check runs at save time, not at open time |
| View | `View(SSN)` (`:238-253`) | panel disabled, only the two print buttons, gated on permission `2114` |

`ClearForm` restores the custodian account code from the INI (`TankhahEdit.pas:518`), and the
`MenuItem1` popup writes the current code back as the default (`:165-168`). Unlike `CheckEditU`
there is **no** `N1Click` / covering-letter memory, because there is no `TM_Tittle`.

**`TankhahEditAddu` (`TTankhahEditAddF`)** — the single-line dialog: expense account picker
(`TD_Code`, `TD_FullName`), amount `TD_Mab`, description `TD_Desc`. Three validations, §9.6 rules
62-64. Returns `Tag = 1` on OK.

**`TankhahList` (`TTankhahListF`)** — `Select * From TankhahMaster Where TM_Coid=:Coid Order By
TM_Date` (`TankhahList.dfm:448`). **Year-scoped and date-ordered**, unlike the cheque and deposit
lists (§3.4). Opens positioned on the last row (`Q1.Last`, `TankhahList.pas:104`).

| Button | Permission | Guard | Action |
|---|---|---|---|
| `B_New` | 2121 | fiscal year open | `TankhahEditF.New` |
| `B_Edit` | 2122 | fiscal year open; `Max(DM_TX) = 0` on `TM_Sanad` | `TankhahEditF.Edit(TM_SSN)` |
| `B_View` | 2123 | none | `TankhahEditF.View(TM_SSN)` |
| `B_Delete` | *(none — always enabled)* | fiscal year open; `Max(DM_TX) = 0`; two-line confirmation | deletes header + lines + `M_Id=41` postings in one transaction, then `Dmoein_UpdateMab` (`:152-162`) |
| `B_Print1` / `B_Print2` | 2124 | none | loads the claim into the editor and fires its report |
| `B_ViewSanad` | 1121 | `TM_Sanad > 0` | opens the voucher read-only in `SanadEditF.View` |

Note the petty-cash permission band is **2121-2124**, distinct from received cheques (2102-2109),
issued cheques (2111-2114, 2125) and treasury printing (2114). `B_Delete` has **no permission key at
all** — anyone who can open the list can delete a claim, provided its voucher is still in draft.

### 7.7 Known defects

1. **`TankhahList.B_NewClick` locates the wrong record.** `TankhahList.pas:219` reads
   `Q1.Locate('TM_SSN', checkeditF.Tag, …)` — `checkeditF` is the **issued-cheque** editor, not
   `TankhahEditF`. After saving a new claim the grid jumps to whatever id the cheque editor happened
   to hold, or nowhere. Cosmetic, but it is a live cross-module bug.
2. **The delete confirmation compares against the literal `6`** rather than `mrYes`
   (`TankhahList.pas:145`).
3. **`TD_BedName` is populated from the *full* account path** (`TankhahEditAddF.TD_FullName`,
   `TankhahEdit.pas:262`) while the header's `TM_CodeName` is populated from the **last segment
   only** (`Taraf.Get_LastName`, `TankhahEdit.pas:211`). The two denormalised name columns in the
   same document mean different things.
4. **No draft-state check when opening the editor for edit** — only on save (`TankhahEdit.pas:334-348`)
   and in the list handler (`TankhahList.pas:190-199`). A user reaching `Edit` another way would
   fill in the whole form before being told.
5. **Deleting a claim does not check `S_LinkPrg`-style provenance** because petty-cash claims have
   no source-module linkage at all — they can only be created by hand.


---

[← 6. Deposit slips (Fish)](06-06-deposit-slips-fish.md) | [index](00-index.md) | [8. Accounting integration →](06-08-accounting-integration.md)
