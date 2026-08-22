_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 6. Deposit slips (Fish)

### 6.1 A Fish groups nothing

The name (فیش, "bank deposit slip") and the docs brief both suggest a batching document. **It is
not one.** A `DFish` row is a single flat money-in event:

> on date D, party P paid amount M into our bank account B, by channel C, with reference number N.

There is no detail table, no line items, no `DFishDetail`, and no join from `DCheck` to `DFish`.
**Cheques never attach to a deposit slip.** A cheque's journey to the bank is `CheckDaryaft2U`
(T4, §2.3), which writes `DCheck2` and has no knowledge of `DFish` at all. Cash never attaches
either, because there is no cash-deposit document distinct from `DFish` itself.

If the business practice is "put five cheques on one paying-in slip", the system cannot represent
it: each cheque is deposited individually, and a `DFish` row is only used for money arriving by the
four electronic/cash channels listed below.

### 6.2 The four channels — `S_State` is a method, not a status

`DFish.S_State` is `S_State.ItemIndex + 1` from a fixed 4-item combo box
(`FISHDaryaftU.pas:418`, `FISHDaryaftU.dfm:172-184`), and `S_StateName` is that item's text
(`FISHDaryaftU.pas:432`). A deposit slip has **no lifecycle** — it is created, optionally edited, and
optionally deleted. Nothing ever changes `S_State` after creation.

| Value | Persian (decoded from `FISHDaryaftU.dfm:179-182`) | English | Proposed enum |
|---|---|---|---|
| 1 | `واریزی از طریق کارتخوان` | Deposit via card reader (POS terminal) | `PosTerminal` |
| 2 | `واریزی از طریق فیش نقدی` | Deposit via cash paying-in slip | `CashSlip` |
| 3 | `واریزی از طریق کارت به کارت` | Deposit via card-to-card transfer | `CardToCard` |
| 4 | `واریز حواله پایا و ساتنا` | Deposit by PAYA / SATNA wire transfer | `WireTransfer` |

PAYA (پایا) and SATNA (ساتنا) are Iran's ACH and RTGS rails respectively; the legacy design lumps
them into one option.

The channel is **purely descriptive**: it changes no account, no validation and no posting. Its only
effect is that its Persian label is pasted into both voucher narrations
(`'بابت ' + S_State.Text + ' شماره ' + S_FishNo + …`, `FISHDaryaftU.pas:461-463`).

`ClearForm` defaults it to index 0 → value 1, `کارتخوان` (`FISHDaryaftU.pas:142`).

### 6.3 Numbering

`S_FishNo` is a free-text `varchar(15)` typed by the operator — the reference number the bank gave.
There is **no sequence, no generator, no uniqueness check and no blank check** (§9.4). Two slips may
carry the same number, or none.

The internal identity is `S_SSN` (identity column). The voucher number `S_Sanad` is allocated from
the document date by `Dm.Get_NewSanad_DateID(S_Date, '21,…,29')` (`FISHDaryaftU.pas:375`) and is
therefore shared with the day's cheque activity (§8.2). Note the ordering bug at
`FISHDaryaftU.pas:374-384`: the new voucher number is assigned to `S_Sanad` **before** the
draft-state check runs on `_OldSanad`, so the field is already overwritten when the error dialog
appears.

### 6.4 Accounting effect

One posting, `M_Id = 25`, `M_Link = DFish.S_SSN`, two lines (`FISHDaryaftU.pas:466-480`):

| Side | Account | Amount |
|---|---|---|
| **Debit** | `DFish.S_BankSSN` — the bank/cash account the money landed in | `DFish.S_Mab` |
| **Credit** | `DFish.S_BesSSN` — the party who paid | `DFish.S_Mab` |

Both lines are built with `INSERT … SELECT … left join sarfasl` so the account hierarchy columns
come from the join (§8.4, set-based variant).

Idempotency: `Delete moein where M_coid=@Coid and M_link=@SSN and M_ID=25`
(`FISHDaryaftU.pas:459`) before re-inserting. Note this deletes by `(coid, link, id)` **without**
`M_Sanad`, which is correct — it catches lines left behind on a previously allocated voucher.

Afterwards `DM.DMoein_Make(S_Sanad, S_Date, 'عملیات خزانه مورخ ' + S_Date)` and
`Dm.Dmoein_UpdateMab(_OldSanad)` (`FISHDaryaftU.pas:488-489`).

**The narration strings are attached to the wrong sides** — see §8.5 defect 2.

### 6.5 Screens

**`FISHDaryaftU` (`TFishDaryaftF`)** — the editor. Five public entry points, two of which are dead:

| Entry point | Purpose | Status |
|---|---|---|
| `new` (`:342`) | blank slip | live, called from `FishListD.F_NewClick` |
| `Edit(_SSN)` (`:190`) | load and edit | live, called from `FishListD.F_EditClick` |
| `New_From_PRg(prg, factor, bes, date)` (`:274`) | create pre-linked to a source document; sets `S_LinkPrg`, `S_LinkSSN`, locks the payer, and pre-fills the description `'بابت فاکتور کالا شماره ' + N` "for goods invoice N" when `prg = 1` | live, called from the invoice-settlement flow |
| `New_From_Factor(...)` (`:223`) | older invoice-linked variant | **dead — `exit;` on the first line (`:226`)** |
| `Edit_From_Factor(_SSN)` (`:293`) | older invoice-linked edit | **dead — `exit;` on the first line (`:295`)** |
| `Delete_Fish(_SSN)` (`:159`) | delete with confirmation | defined but **never called**; the working delete lives in `FishListD.F_DeleteClick` |

`New_From_Factor` also contains the only reference to the INI key `Base/NewFishBank`
(`FISHDaryaftU.pas:261-269`), a per-installation default bank account — dead along with the method.

**`FishListD` (`TFishListDF`)** — the list. `Select * From DFish Where 1=1 Order By S_Dates`
(`FishListD.pas:225-231`). Like the cheque list:

- **no fiscal-year filter** — every year's slips are shown together;
- the three filter clauses are present but **commented out** (`FishListD.pas:227-229`);
- `State1Click` (the shared handler for the state speed-buttons) begins with `exit;`
  (`FishListD.pas:242`) — dead;
- `Order By S_Dates` references a column that appears in **no** declared field list for `DFish`,
  which is indirect evidence that the physical table does have an `S_DateS` column this application
  never writes (§1.3, §12);
- permissions are **shared with the received-cheque list**: `2102` new, `2103` edit, `2104` delete
  (`FishListD.pas:165-167`) — the same three keys `CheckListDU.pas:239-241` uses. Granting a user the
  right to enter cheques necessarily grants the right to enter deposit slips.

Delete guards (`FishListD.pas:257-290`): fiscal year must match exactly; the voucher must be in
draft; **and `S_LinkPrg > 0` blocks deletion entirely** with
`از برنامه جانبی برای حذف استفاده کنید` "use the side program to delete" — i.e. a slip created from
an invoice can only be removed by deleting the invoice. The cheque module has no equivalent guard.

The `S_LinkPrg` display formatter recognises **two** source modules here versus the cheque list's one
(`FishListD.pas:188-192`):

| `S_LinkPrg` | Label | Meaning |
|---|---|---|
| 0 | *(blank)* | entered by hand |
| 1 | `' فاکتور کالا ' + S_LinkSSN` | goods invoice N |
| 2 | `' فاکتور پسته ' + S_LinkSSN` | pistachio invoice N |

### 6.6 `Ghabz` and `Asnad_Daryaft_NewU` are **not** part of this

Two units listed as deposit-slip sources turn out to belong elsewhere or nowhere:

- **`Ghabz.pas`** (قبض, "receipt note") is a `TFrame`, not a form, and has nothing to do with
  treasury. It displays a **weighbridge ticket** from `DM.B_SelectSerial`: `SerialNoPsnBts`,
  `SerialnoBts`, `InWeightBts`, `OutWeightBts`, `NetWeightBts`, `StatusBts`
  (`Ghabz.pas:57-86`). Its eight status labels are `توزين اول` (first weighing), `تشخيص`
  (inspection), `توزين دوم` (second weighing), `انبار` (warehouse), `باطل` (void), `رمز شده`
  (encoded), `کشف رمز شده` (decoded), `قيمت شده` (priced). This is the pistachio/inventory domain —
  it belongs in the inventory specification, not here.
- **`Asnad_Daryaft_NewU`** (اسناد دریافتنی, "notes receivable") looks like a second cheque-entry
  form — it has `AD_Mab`, `AD_Desc`, `AD_Sanad`, `AD_Date`, `AD_SDate` (a due date) and a `B_Save`.
  **`B_SaveClick` validates one rule and then returns without saving anything**
  (`Asnad_Daryaft_NewU.pas:52-60`); the only message is
  `مبلغ چک حداقل 1 ریال مجاز است` "the cheque amount must be at least 1 rial". The unit is 97 lines
  long, writes to no table, and is referenced only by `Mainu.pas:285`'s `uses` clause. It is an
  abandoned rewrite of `CheckDaryaftU`. Do not port it.


---

[← 5. Due-date logic](06-05-due-date-logic.md) | [index](00-index.md) | [7. Petty cash (Tankhah) →](06-07-petty-cash-tankhah.md)
