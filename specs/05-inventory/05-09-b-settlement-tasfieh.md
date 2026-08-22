_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 9.4 Operations

#### 9.4.1 Add

| Button | Caption | Handler | Calls |
|---|---|---|---|
| `B_AddF` | `واریزی جدید` "new deposit" | `:209-216` | `FishDaryaftF.New_From_PRg(_Prg, F_No.IntValue, F_Code.Tag, F_Date.Text)` |
| `B_ADDC` | `چک جدید` "new cheque" | `:200-207` | `CheckDaryaftF.New_From_PRg(_Prg, F_No.IntValue, F_Code.Tag, F_Date.Text)` |

`FishDaryaftU.New_From_PRg` (`FISHDaryaftU.pas:274-291`) shows what the link and the defaults are:

```pascal
procedure TFishDaryaftF.New_From_PRg(_PRg, _Factor, _Bes: integer; _Date: String);
begin
    ClearForm;
    _LinkPrg := _PRg;
   _LinkSSN := _Factor;                      // ← the invoice NUMBER
    S_Sanad.ReadOnly := true;
    S_Date.Farsi_Date := _Date;              // ← defaults to the invoice date
    Taraf.Set_SSN( _Bes );
    S_Bes.Text := Taraf.Get_FullCode;
    S_Bes.ReadOnly := True;                  // ← counterparty locked to the invoice's
    B_Bes.Enabled := False;
    if _prg=1 then  S_Desc.Text := 'بابت فاکتور کالا شماره ' + inttostr(_Factor) ;
    ShowModal;
end;
```

`'بابت فاکتور کالا شماره <n>'` — "for goods invoice number \<n\>". Note the default description is
**only set for `_Prg = 1`**; a pistachio settlement (`_Prg = 2`) gets an empty description.

Three things are forced and cannot be changed by the operator: the counterparty account
(`S_Bes.ReadOnly := True`, `B_Bes.Enabled := False`), the link, and the voucher number field.
The **date defaults** to the invoice date but stays editable — so a settlement can be dated before
the invoice, and nothing checks.

**No amount default and no amount cap.** The instrument's `S_Mab` is keyed freely. The invoice
total is not passed in.

#### 9.4.2 Edit

`B_EditClick` (`:255-290`) branches on the `Type` discriminator:

- `Type = 1` (deposit slip): requires permission `2117`, else
  `اجازه اصلاح فیش را ندارید` ("you do not have permission to correct the deposit slip");
  then `FishDaryaftF.Edit(S_SSN)`.
- `Type = 2` (cheque): requires permission `2103`, else
  `اجازه اصلاح چک را ندارید`; **and** requires `S_State = 1`, else
  `فقط چک موجود در صندوق قابل اصلاح میباشد` — **"only a cheque still in the cash box can be
  corrected"**; then `CheckDaryaftF.Edit(S_SSN)`.

#### 9.4.3 Delete

`B_DeleteClick` (`:218-253`), same shape:

- deposit slip: permission `2118`, else `اجازه حذف فیش را ندارید`; then
  `FishDaryaftF.Delete_Fish(S_SSN)`.
- cheque: permission `2104`, else `اجازه حذف چک را ندارید`; **and** `S_State = 1`, else
  `فقط چک موجود در صندوق قابل حذف میباشد` ("only a cheque still in the cash box can be deleted");
  then `CheckDaryaftF.Delete_Check(S_SSN)`.

`S_State = 1` is the treasury "in hand / in the cash box" state — see `docs/06-treasury.md`. Once
a cheque has been deposited, cleared or bounced it can no longer be detached from the invoice from
here.

> **Defect — the Delete button's enabled-state uses the wrong permission.**
> `FormActivate` sets `B_Delete.Enabled := Dm.IsEnabel(Dm.userId, 2118)` (`:96`) — the **deposit
> slip** delete permission only. But the handler also serves cheques, gated on `2104` (`:236`).
> A user granted `2104` (delete cheque) but not `2118` therefore has the button disabled and
> **cannot delete a cheque at all**, despite holding the right. Compare `B_Edit`, which correctly
> ORs both codes: `Dm.IsEnabel(…,2117) or Dm.IsEnabel(…,2103)` (`:93-94`). The `B_Delete` line is
> missing its `or Dm.IsEnabel(Dm.userId, 2104)`.

**Permission codes used by this screen:**

| Code | Grants |
|---|---|
| `2115` or `2101` | opening the screen at all (`AnbarListU.pas:503`) |
| `2116` | create deposit slip |
| `2102` | create cheque |
| `2117` | edit deposit slip |
| `2103` | edit cheque |
| `2118` | delete deposit slip |
| `2104` | delete cheque |

Note the permission codes are in the **treasury** range (2xxx), not the inventory range (14xx) —
the feature is governed by treasury rights even though it is reached from the inventory list.

`Q1BeforeDelete` calls `abort` (`:195-198`), blocking grid-level deletion.

---

### 9.5 Restrictions and gaps

| Rule | Where | Effect |
|---|---|---|
| **Only sales invoices can be settled** (subsystem A) | `AnbarListU.pas:476-480` — `فقط برای فاکتورهای فروش فعال است` "only enabled for sales invoices" | Purchases (type 1) can never carry a payment link. Supplier payments are recorded in treasury with no invoice reference. |
| **Only `FM_ID = 22` can be settled** (subsystem B) | `SodoorSanadU.pas:149-153`, same message | Same restriction, same gap |
| **A settled invoice can still be edited** | `EditFactor` (`AnbarFactorU.pas:180-202`) checks only `Moein_TX` | The invoice total can be changed after payment; the settlement amount becomes unrelated. §4.4 |
| **A settled invoice cannot be deleted** | `AnbarListU.pas:355-375` | The only place settlement is treated as a constraint |
| **A settled invoice can be renumbered** | `AnbarListU.pas:450-454` rewrites `S_LinkSSN` | Works — but it is the *only* thing keeping the link valid, and any renumber outside this screen orphans every payment |
| **No over-settlement check** | nowhere | §9.3 |
| **No settlement status on the document** | no column on `Anbar_Factor` or `FactorMaster` | "Paid / partly paid / unpaid" must be derived by summing on every read |
| **Settlement does not post anything** | `TasfiehFactor` writes no `Moein` row | The instrument's own voucher is created by `FishDaryaftU` / `CheckDaryaftU`; see `docs/06-treasury.md` |
| **Currency, exchange gain/loss, discount-on-early-payment** | do not exist | — |

---

### 9.6 A second, dead implementation: `DM.Anbar_Tasfieh`

`Dmu.dfm:1017-1072` defines a query `Anbar_Tasfieh` with parameters `@Sal` (year) and `@Factor`:

```sql
Declare @Sal int Set @Sal = :Sal
Declare @Sanad int
Declare @Factor int Set @Factor =:Factor
Declare @Jari int
Set @Jari  = ( Select min(AF_Customer) From Anbar_Factor Where AF_Coid=@Sal and AF_Factor=@Factor )
set @Sanad = ( Select min(AF_Sanad)    From Anbar_Factor Where AF_Coid=@Sal and AF_Factor=@Factor )

Select 22 AS _ID , S_StateName as _SN, S_SSN As _SSN , S_FishNo As _Link , S_Mab As _Mab ,
       S_Desc As _Desc , '' As _DSar
--   From DFish Where S_Coid=@Sal And S_Sanad=@Sanad and S_BesSSN=@Jari
   From DFish Where S_Coid=@Sal And S_LinkPRG=1 and S_LinkSSN=@Factor
union
Select 21 AS _ID , S_StateName as _SN, S_SSN As _SSN , S_CheckNo As _Link , S_Mab As _Mab ,
       S_Desc As _Desc , S_DateS as _DSar
--   From DCheck Where S_Coid=@Sal And S_Sanad=@Sanad and S_BesSSN=@Jari
   From DCheck Where S_Coid=@Sal And S_LinkPRG=1 and S_LinkSSN=@Factor
```

**It is dead.** Its only references (`AnbarFactorU.pas:367, 413`) are inside the
`{ … }` comment blocks of the four disabled handlers `D_NaghdiClick`, `D_EditClick`,
`D_checkClick`, `D_DeleteClick` (`AnbarFactorU.pas:364-451`), which are themselves commented out
at the declaration site (`:76-79`). The design being abandoned was an **inline settlement panel on
the invoice screen** — the buttons for cash, cheque, edit and delete lived on the invoice itself —
which was replaced by the separate `TasfiehFactor` form reached from the list.

Two things are worth keeping from it:

1. **The `_ID` discriminator convention is inverted relative to `TasfiehFactor`.** Here `22` =
   deposit slip and `21` = cheque; in `TasfiehFactor` `1` = deposit slip and `2` = cheque. Two
   discriminators for the same union.
2. **The commented-out `WHERE` clauses record the *previous* linking strategy**:
   `S_Sanad = @Sanad and S_BesSSN = @Jari` — match by voucher number **and** counterparty account,
   rather than by an explicit link column. That was replaced by `S_LinkPRG` / `S_LinkSSN`. The
   history is visible in the source; the migration of existing rows to the new columns is not.
   **Open question §14**: do pre-migration `DFish`/`DCheck` rows have `S_LinkPRG = 0`?

`Anbar_Tasfieh` also has a `BeforeDelete` handler wired in `Dmu` and seven persistent field
definitions (`Dmu.pas:81-87`) — all dead weight.

---

### 9.7 Summary for the rebuild

The current feature reduces to:

```
settle(document) :=
    list treasury instruments where (link_module, link_number, fiscal_year)
                                  = (1 or 2, document.number, document.fiscal_year)
    allow create / edit / delete subject to treasury permissions
    show the client-side sum of their amounts
```

What the rebuild needs, at minimum, to be equivalent-but-correct:

1. Link by **surrogate key** (`settlements.document_id`), not by number + module + year.
2. A real `settlements` join table if one instrument may cover several invoices — **which today it
   cannot**, because `S_LinkSSN` is scalar. Confirm with the business whether that limitation is
   intended (§14) before designing it away.
3. A derived, indexed `settled_amount` and `outstanding_amount` on the document, plus a
   `settlement_status` enum.
4. An over-settlement rule: reject, warn, or allow — **a decision, not a default** (§15).
5. Extend settlement to purchase documents, or confirm that supplier payments are deliberately
   unlinked (§14).
6. One discriminator for "deposit slip vs cheque", not the two conflicting ones of §9.6.


---

[← 9. Settlement (Tasfieh) (part a)](05-09-a-settlement-tasfieh.md) | [index](00-index.md) | [10. Accounting integration (part a) →](05-10-a-accounting-integration.md)
