_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 2. The cheque state machine

This is the core of the treasury module. **Only received cheques (`DCheck`) have a state machine.**
Issued/paid cheques are handled by an entirely different pair of tables with no lifecycle at all —
see §3.

### 2.0 Where the state lives

`DCheck.S_State` (int) + `DCheck.S_StateName` (varchar(50), the denormalised Persian label written by
whichever screen last touched the row) — `Dmu.dfm:929-935`.

Every transition also appends an immutable-in-practice audit row to **`DCheck2`**, a history/event
table keyed by `S_Link = DCheck.S_SSN`. The history grid on the cheque list reads it with
`Select * From DCheck2 Where S_Link=<S_SSN> Order By S_SSN` (`CheckListDU.pas:161-165`).
`DCheck2` columns actually written: `S_Link, S_Coid, S_Sanad, S_Date, S_Mab, S_State, S_StateName,
S_BedSSN, S_BesSSN, S_Desc, S_UserID` (`CheckDaryaft2U.pas:192`). The collection screen omits
`S_BesSSN` from that list (`CheckVosoolU.pas:225`).

There is **no `DCheck2` row for the initial receipt** — state 1 is only ever recorded on `DCheck`
itself (`CheckDaryaftU.pas:285-286`). The history of a cheque therefore always starts at its second
event.

### 2.1 The declared state codes

The only place the codes are enumerated is a comment block in the collection screen
(`CheckVosoolU.pas:167-171`), reproduced verbatim:

```
///   Dcheck , Dcheck 2
///     S_State = 1   check dar sandoogh
///     S_State = 2   check dar bank.
///     S_State = 3   check bargeh
///     S_State = 4   check bargast be saheb hesab
///     S_State = 5   check vosool.
```

| Code | Persian label written to `S_StateName` | Translation | Proposed enum | Written by |
|---|---|---|---|---|
| 1 | `چک موعدي در صندوق` | "Dated cheque in the cash box" (in hand) | `InHand` | `CheckDaryaftU.pas:286` |
| 1 | `چک برگشت شده از بانک` | "Cheque bounced back from the bank" | `Bounced` — **shares code 1** | `CheckBargashtu.pas:209` |
| 2 | `چک موعدی در بانک` | "Dated cheque at the bank" (deposited for collection) | `AtBank` | `CheckDaryaft2U.pas:189` |
| 3 | *(never written)* | intended "bounced" per the comment | `Bounced` (unreachable) | — |
| 4 | `‌ چک مسترد شد ` (with surrounding spaces) | "Cheque was returned [to the issuer]" | `ReturnedToIssuer` | `CheckEsterdadU.pas:188` |
| 5 | `چک وصول شد` | "Cheque was collected" | `Cleared` | `CheckVosoolU.pas:222` |

**Finding — state 3 is dead.** No code path anywhere in the repository assigns `S_State=3`. The
bounce screen, which the comment says should produce state 3, instead sets the cheque *back to
state 1* (`CheckBargashtu.pas:209`) so it re-enters the "in hand" pool and can be re-deposited. The
only artefacts of state 3 are (a) the grid row-colour map `$b9daff` (`CheckListDU.pas:216`, inside a
procedure that begins with `Exit;` at line 208, so the colouring never runs at all), (b) the report
title `لیست چکهای برگشت خورده` "list of bounced cheques" (`CheckListDU.pas:264`), and (c) a
`State3: TsSpeedButton` with `Tag = 3` and **no `OnClick` handler** (`CheckListDU.dfm:317-327`;
confirmed by the complete `OnClick` inventory of that form — only `S_Search`, `S_Print`,
`GridFontSize`, `S_Edit`, `S_Delete`, `S_New`, `sBitBtn4`, `S_Bargasht`, `S_Vosool`, `S_BBank`,
`S_Bank`, `Print1` are wired).

**Finding — state 1 is overloaded.** "Never deposited" and "deposited, then bounced" are the same
numeric state and are distinguishable only by the free-text `S_StateName` string or by inspecting
`DCheck2`. The rebuild must split these (see §13).

**Finding — the `DCheck2` audit row for a bounce records the wrong state.** `CheckBargashtu.pas:214`
inserts `S_State = 2` with label `برگشت از بانک` while `CheckBargashtu.pas:209` sets the master row
to `S_State = 1`. The history table and the master row disagree on every bounce.

**Finding — "written off" does not exist.** There is no write-off, no cancellation, no
stale-dated/expired state, and no partial settlement anywhere in the cheque module.

### 2.2 Guards shared by every transition

Every transition button on the cheque list runs the same preamble
(`CheckListDU.pas:343, 375, 407, 439, 494, 535`):

1. `DM.Is_New_Sanad_Valid(Dm.CO_ID)` — the fiscal year must still accept new vouchers; returns
   `False` silently aborts the action.
2. The grid dataset must be open and non-empty.
3. A state precondition (per transition, tabulated below).
4. A fiscal-year precondition (per transition, tabulated below).

Inside each transition screen, `Dm.Get_SanadMaxTX(<voucher no>) > 0` blocks the save with
`جهت ذخیره اطلاعات سند N را در حالت تحریر قرار دهید` — "to save, put voucher N back into draft
state" (`CheckDaryaftU.pas:228-232`). See `docs/03-accounting-core.md` for the `0 → 1 → 2` voucher
state machine; `M_Tx > 0` means the voucher has left draft.

### 2.3 Transition-by-transition

#### T1. Receive a cheque → state 1

- **Trigger**: `S_New` on the cheque list (`CheckListDU.pas:530`, permission `2102`), or
  `CheckDaryaftF.New_From_PRg(prg, factor, bes, date)` called from an invoice-settlement flow
  (`CheckDaryaftU.pas:148-161`), which pre-fills and locks the counterparty and stamps
  `S_LinkPrg` / `S_LinkSSN`.
- **Screen**: `CheckDaryaftU` (`TCheckDaryaftF`).
- **Preconditions**: fiscal year open; no state precondition (this creates the row).
- **Fields required**: counterparty account `S_Bes` (must be a leaf account —
  `Dm.is_Sarfasl_Last_Deep_SSN`), the notes-receivable account `S_Bed` (defaulted to
  `Dm.SanDoogh_kM + '-' + <counterparty's Tafsil-1>`, `CheckDaryaftU.pas:187`), amount `S_Mab > 0`,
  voucher date `S_Date` inside `[Dm.From_Date, Dm.To_Date]`, description `S_Desc` non-blank.
  Cheque number `S_CheckNo` and due date `S_DateS` are **not** validated (see §9).
- **Fields updated**: a new `DCheck` row with `S_State=1`,
  `S_StateName='چک موعدي در صندوق'`, `S_COID=Dm.CO_ID`, `S_UserID=Dm.userId`, `S_CheckNo`,
  `S_Date`, `S_DateS`, `S_Mab`, `S_Desc`, `S_LinkPrg`, `S_LinkSSN`, and the two account triplets
  `S_BesSSN/S_BesCR/S_BesName` and `S_BedSSN/S_BedCR/S_BedName` (`CheckDaryaftU.pas:282-309`).
  The voucher number is (re)allocated by
  `Dm.Get_NewSanad_DateID(S_Date, '21,22,23,24,25,26,27,28,29')` (`CheckDaryaftU.pas:269`), i.e. the
  cheque shares a per-date treasury voucher with every other treasury document of that day.
- **Voucher**: `M_Id = 21`, `M_Link = DCheck.S_SSN`, `M_Kind = 1` (`CheckDaryaftU.pas:329-347`).
  Existing lines are deleted first (`Delete moein where M_link=@ssn and M_id=21`,
  `CheckDaryaftU.pas:327`) so edit is idempotent.
  - **Debit** `S_BedSSN` (notes receivable on hand) for `S_Mab`.
  - **Credit** `S_BesSSN` (the party who handed over the cheque) for `S_Mab`.
  - `Article` = `'بابت دریافت چک شماره ' + S_CheckNo + ' مبلغ ' + dbo.Noto3(S_Mab) + ' سررسید ' +
    S_DateS + ' توسط ' + S_BesName` — "for receipt of cheque no. N, amount <spelled out>, due D,
    from P" (`CheckDaryaftU.pas:324-325`; `dbo.Noto3` is a SQL Server function that spells a number
    in Persian words).
  - Then `Dm.Dmoein_UpdateMab(_OldSanad)` and
    `Dm.DMoein_Make(newSanad, date, 'عملیات خزانه')` — "treasury operations"
    (`CheckDaryaftU.pas:359-361`).

#### T2. Edit a received cheque (state 1 only)

- **Trigger**: `S_Edit` (`CheckListDU.pas:489`, permission `2103`).
- **Preconditions**: `S_State = 1` else `چک در صندوق نیست` "the cheque is not in the cash box"
  (`CheckListDU.pas:499-503`); `S_COID = Dm.CO_ID` exactly — you must be *in* the year the cheque was
  received, else `جهت اصلاح یا حذف چک باید در سال مالی صدور چک قرار داشته باشید`
  (`CheckListDU.pas:506-510`); `Dm.Moein_Tx(S_Sanad) = 0` (voucher still in draft)
  (`CheckListDU.pas:512-517`).
- **Effect**: re-runs T1's save path in update mode; **reallocates the voucher number from the
  date** (`CheckDaryaftU.pas:269`) and deletes+reposts the `M_Id=21` lines. If the counterparty came
  from an invoice (`S_LinkPrg > 0`), `S_Bes` is read-only and the picker button is disabled
  (`CheckDaryaftU.pas:132-133`).

#### T3. Delete a received cheque — **disabled**

- **Trigger**: `S_Delete` (`CheckListDU.pas:434`, permission `2104`).
- The handler validates fiscal year and `S_State = 1`, then executes a bare `Exit;` at
  `CheckListDU.pas:457`. **Everything below — the draft-voucher check, the confirmation dialog and
  the actual `Delete From DCheck` / `Delete From moein` (lines 462-485) — is unreachable dead code.**
  The button appears enabled and does nothing.
- A *working* delete exists but is never called from any UI:
  `TCheckDaryaftF.Delete_Check(_SSN)` (`CheckDaryaftU.pas:411-441`). It checks `Dm.Moein_Tx`, asks
  for confirmation via `GetYes`, then deletes the `DCheck` row and the `M_Id=21` voucher lines. It
  does **not** delete `DCheck2` history rows. Grep for `Delete_Check` returns only its own
  declaration and body.
- Additionally `Q1BeforeDelete` / `Q2BeforeDelete` call `Abort` (`CheckListDU.pas:274-277, 295-298`),
  so grid-level deletion is blocked, and `Dmu`'s shared `QCheckBeforeDelete` guards
  `TCheck/QCheck/QDCheck/DCheck/QDFish` (see §9).

#### T4. Deposit to bank (in hand → at bank) — state 1 → 2

- **Trigger**: `S_Bank` (`CheckListDU.pas:338`, permission `2105`) → `CheckDaryaft2F.New(S_SSN)`.
- **Preconditions**: `S_State <= 1`, else
  `جهت واگذاری چک به بانک باید چک در صندوق موجود باشد` — "to hand the cheque to the bank it must be
  in the cash box" (`CheckListDU.pas:348-353`). Note the test is `_C > 1`, so a state of 0 or
  negative would also pass. `S_COID <= Dm.CO_ID` (`CheckListDU.pas:355-360`) — you may deposit in a
  *later* year than receipt, but not an earlier one.
- **Fields required**: the "notes in course of collection" account `S_Bed`, defaulted to
  `Jaryan_K-Jaryan_M-<counterparty Tafsil-1>` (`CheckDaryaft2U.pas:123-128`) and required to be a
  leaf (`CheckDaryaft2U.pas:149-152`); transfer date `S_Date2` in range; optional note `S_Desc2`.
  A new voucher number `S_Sanad2` is allocated from `S_Date2` if zero
  (`CheckDaryaft2U.pas:177`).
- **Fields updated on `DCheck`**: `S_State=2`, `S_StateName='چک موعدی در بانک'` and *nothing else* —
  the deposit date, the target bank and the collection account are recorded **only** on the
  `DCheck2` history row (`CheckDaryaft2U.pas:189-195`). There is no `deposited_at` or `bank_id`
  column on the cheque.
- **`DCheck2` row**: `S_State=2`, label `چک موعدی در بانک`, `S_BedSSN = <collection account>`,
  `S_BesSSN = <old S_BedSSN>`, `S_Desc = 'انتقال چک به بانک ' + S_Desc2` ("transfer of cheque to the
  bank").
- **Voucher**: `M_Id = 22`, `M_Link = @@IDENTITY` of the new `DCheck2` row (**not** the cheque id)
  (`CheckDaryaft2U.pas:202-222).
  - **Debit** the collection account (`S_Bed.Tag`) for `S_Mab`.
  - **Credit** the previous notes-receivable-on-hand account (`S_Bes.Tag`, which was loaded from
    `DCheck.S_BedSSN` at `CheckDaryaft2U.pas:117`) for `S_Mab`.
  - `Article` = `'بابت واگذاری به بانک چک شماره ' + no + ' مبلغ ' + amount + ' سررسید ' + dueDate +
    ' توسط ' + payerName` (`CheckDaryaft2U.pas:181-182`).
  - Then `DMoein_Make` + `Dmoein_UpdateMab` (`CheckDaryaft2U.pas:227-228`).

#### T5. Bounce (at bank → back in hand) — state 2 → 1

- **Trigger**: `S_BBank` (`CheckListDU.pas:402`, permission `2106`) → `CheckBargashtF.New(S_SSN)`.
- **Precondition**: `S_State = 2` exactly, else
  `جهت برگشت چک از بانک ، چک باید در بانک باشد .` — "to bounce a cheque from the bank, the cheque
  must be at the bank" (`CheckListDU.pas:412-417`). Fiscal year `S_COID <= Dm.CO_ID`.
- **Accounts** are both derived, not chosen: from the counterparty's Tafsil-1, the screen resolves
  the collection account `Jaryan_K/Jaryan_M/Ta1` (credit side) and the on-hand account
  `Sandoogh_K/Sandoogh_M/Ta1` (debit side) (`CheckBargashtu.pas:131-151`).
- **Fields required**: bounce date `S_Date2` in range; note `S_Desc2` optional.
- **Fields updated on `DCheck`**: `S_State=1`, `S_StateName='چک برگشت شده از بانک'`
  (`CheckBargashtu.pas:209`). **The cheque re-enters the in-hand pool and can be deposited again.**
- **`DCheck2` row**: `S_State=2` (wrong, see §2.1), label `برگشت از بانک`, `S_BedSSN = <on-hand
  account>`, `S_BesSSN = <collection account>`, `S_Desc = 'برگشت چک از بانک ' + S_Desc2`.
- **Voucher**: `M_Id = 22` (same id as the deposit), `M_Link = @@IDENTITY` of the `DCheck2` row
  (`CheckBargashtu.pas:222-242`).
  - **Debit** the on-hand notes account for `S_Mab`; **Credit** the collection account for `S_Mab` —
    an exact reversal of T4.
  - `Article` = `' بابت برگشت از بانک چک شماره ' + no + ' مبلغ ' + amt + ' سررسید ' + due + ' توسط '
    + payer + ' به بانک'`.
  - Then `DMoein_Make` + `Dmoein_UpdateMab` (`CheckBargashtu.pas:247-248`).
- **No bank-charge / penalty posting** is generated.

#### T6. Collect / clear (at bank → cleared) — state 2 → 5

- **Trigger**: `S_Vosool` (`CheckListDU.pas:549`, permission `2107`) → `CheckVosoolF.init(S_SSN)`.
- **Precondition**: `S_State = 2` exactly, else
  `جهت اعلام وصول ، چک باید از قبل به بانک واگذار شده باشد` — "to declare collection, the cheque must
  already have been handed to the bank" (`CheckVosoolU`-caller, `CheckListDU.pas:559-564`).
  `S_COID <= Dm.CO_ID` (`CheckListDU.pas:566-571`).
- **Fields required**: the **bank account to credit the money to**, `S_Bank`, chosen with the
  account picker and remembered per-user in the INI file under key `S_Bank`
  (`CheckVosoolU.pas:145, 152-155, 264-275`). Voucher number `S_Sanad2` (auto-allocated from
  `S_Date2` if zero), date in range, voucher still in draft
  (`Dm.Get_SanadMaxTX`), and `DM.Get_SanadDateID_Valid(S_Sanad2, S_Date2, '21,…,29')`
  (`CheckVosoolU.pas:173-207`).
  **`S_Bank.Tag` is never validated for `> 0`** — see §9.
- **Fields updated on `DCheck`**: `S_State=5`, `S_StateName='چک وصول شد'` (`CheckVosoolU.pas:222`).
  The clearing date and the receiving bank live only on `DCheck2`.
- **`DCheck2` row**: `S_State=5`, label `چک وصول شد`, `S_BedSSN = S_Bank.Tag`, **`S_BesSSN` omitted
  from the column list** (`CheckVosoolU.pas:225-228`), `S_Desc` copied from the *original* cheque
  description, not the operator's note.
- **Voucher**: `M_Id = 23`, `M_Link = @@IDENTITY` of the `DCheck2` row
  (`CheckVosoolU.pas:234-254`).
  - **Debit** the chosen bank account (`S_Bank.Tag`) for `S_Mab`.
  - **Credit** the collection account `Jaryan_K/Jaryan_M/Ta1` (`S_Bes.Tag`, resolved at
    `CheckVosoolU.pas:126-138`) for `S_Mab`.
  - `Article` = `' بابت وصول چک شماره ' + no + ' ' + payerName + ' ' + note`.
- **Finding — the voucher header is never built for a collection.** Unlike every other transition,
  `CheckVosoolU.sBitBtn5Click` ends at `Q2.ExecSQL` (`:258`) with **no** `Dm.DMoein_Make` and no
  `Dm.Dmoein_UpdateMab`. If the allocated voucher number is new, no `DMoein` header row exists for
  it; if it is an existing treasury voucher, its cached total is left stale.

#### T7. Return to the issuer (in hand → returned) — state 1 → 4

- **Trigger**: `S_Bargasht` (`CheckListDU.pas:370`, permission `2109`) → `CheckEsterdadF.init(S_SSN)`.
  Note the naming trap: the button is called *Bargasht* ("bounce") but performs *Esterdad*
  ("return to issuer"), and the button that actually performs a bounce is called `S_BBank`.
- **Precondition**: `S_State <= 1`, else `جهت استرداد چک باید چک در صندوق موجود باشد`
  (`CheckListDU.pas:380-385`); `S_COID <= Dm.CO_ID` (`:387-392`).
- **Accounts**: both loaded from the cheque itself, no picker — debit `DCheck.S_BesSSN` (the party
  gets their obligation back), credit `DCheck.S_BedSSN` (the notes-receivable account)
  (`CheckEsterdadU.pas:113-119`). Both are re-validated as leaves
  (`CheckEsterdadU.pas:135-151`).
- **Fields required**: return date `S_Date2` in range; note `S_Desc2` optional; voucher number
  auto-allocated (`CheckEsterdadU.pas:176`).
- **Fields updated on `DCheck`**: `S_State=4`, `S_StateName=' چک مسترد شد '` (`CheckEsterdadU.pas:188`).
- **`DCheck2` row**: `S_State=4`, label `استرداد چک`, `S_BedSSN = <payer>`, `S_BesSSN = <notes
  account>`, `S_Desc = 'استرداد چک ' + S_Desc2`.
- **Voucher**: `M_Id = 24`, `M_Link = @@IDENTITY` of the `DCheck2` row (`CheckEsterdadU.pas:201-221`).
  - **Debit** the payer account for `S_Mab`; **Credit** the notes-receivable account for `S_Mab` —
    an exact reversal of T1.
  - `Article` = `'بابت استرداد چک شماره ' + no + ' مبلغ ' + amt + ' سررسید ' + due + ' به ' + payer`.
  - Then `DMoein_Make` + `Dmoein_UpdateMab` (`CheckEsterdadU.pas:226-227`).
- State 4 is **terminal**. Every button's precondition excludes it (`>1` or `=2` or `=1`), and
  the due-date filter explicitly hides it (`and (S_State<4)`, `CheckListDU.pas:327`).

#### T8. Endorse to a third party — **not implemented**

The schema carries `S_Zssn`, `S_ZCR`, `S_ZName` (`Dmu.dfm:982-991`) and the cheque list declares
persistent fields for them (`CheckListDU.pas:96-98`), but a repository-wide search finds **no read
and no write** of any of the three outside field declarations and three commented-out
`CreateFieldInTable` calls (`Dmu.pas:249-251`). There is no endorsement screen, no menu entry and no
voucher id reserved for it. See §4.

#### T9. Undo a collection — **not implemented**

`S_DVosool: TsBitBtn` exists on the cheque list, is guarded by permission `2108`
(`CheckListDU.pas:245`), and has **no `OnClick` handler** in `CheckListDU.dfm`. State 5 is terminal
in practice. There is no undo for a deposit, a bounce, a return, or a collection: the only reversal
in the system is T5 (bounce), which undoes T4 (deposit) both in state and in accounting.

### 2.4 State-transition table

| # | From | To | Screen / button | Permission | State precondition | Fiscal-year precondition | `M_Id` | Debit | Credit | `DCheck2` row |
|---|---|---|---|---|---|---|---|---|---|---|
| T1 | *(none)* | 1 | `CheckDaryaftU` ← `S_New` | 2102 | — | year open | 21 | `S_BedSSN` (notes on hand) | `S_BesSSN` (payer) | **none** |
| T2 | 1 | 1 | `CheckDaryaftU` ← `S_Edit` | 2103 | `=1` | `S_COID = CO_ID` **and** voucher in draft | 21 (re-posted) | as T1 | as T1 | none |
| T3 | 1 | *(deleted)* | `S_Delete` | 2104 | `=1` | `S_COID = CO_ID` | — | — | — | — **dead code, no-op** |
| T4 | ≤1 | 2 | `CheckDaryaft2U` ← `S_Bank` | 2105 | `<=1` | `S_COID <= CO_ID` | 22 | collection acct `Jaryan_K-M-Ta1` | old `S_BedSSN` | yes, `S_State=2` |
| T5 | 2 | **1** | `CheckBargashtu` ← `S_BBank` | 2106 | `=2` | `S_COID <= CO_ID` | 22 | on-hand acct `Sandoogh_K-M-Ta1` | collection acct | yes, `S_State=2` *(bug)* |
| T6 | 2 | 5 | `CheckVosoolU` ← `S_Vosool` | 2107 | `=2` | `S_COID <= CO_ID` | 23 | chosen bank account | collection acct | yes, `S_State=5` |
| T7 | ≤1 | 4 | `CheckEsterdadU` ← `S_Bargasht` | 2109 | `<=1` | `S_COID <= CO_ID` | 24 | `S_BesSSN` (payer) | `S_BedSSN` (notes on hand) | yes, `S_State=4` |
| T8 | — | — | endorsement | — | — | — | — | — | — | **not implemented** |
| T9 | 5 | 2 | `S_DVosool` | 2108 | — | — | — | — | — | **button has no handler** |

Reachability: `1 ⇄ 2` (T4/T5) is the only cycle; `1 → 4` and `2 → 5` are the two exits; `3` is
unreachable; nothing leaves `4` or `5`.

### 2.5 Amount source

Every posting in every transition uses `DCheck.S_Mab` unchanged. There is no partial deposit, no
partial collection, no fee, no FX and no rounding anywhere in the cheque lifecycle. Where a screen
appears to let the operator type an amount (`S_Mab` on the bounce/deposit/return screens) the field
is populated read-only from the cheque and only checked for `> 0`.


---

[← 1. Entity model](06-01-entity-model.md) | [index](00-index.md) | [3. Received versus issued cheques →](06-03-received-versus-issued-cheques.md)
