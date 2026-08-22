_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 2. Item master CRUD rules

Two screens: `AnbarCalaU` (the list, caption `انبار کالا` "goods warehouse") and `AnbarCalaAddU`
(the editor, caption `مشخصات کالا` "item specification"). Entity model at §1.2.

### 2.1 Item code generation — there is none

**The item code is keyed by the operator. There is no auto-numbering, no sequence, no prefix and
no suggestion.**

`AnbarCalaAdd.init` (`AnbarCalaAddU.pas:59-103`) initialises a new item with:

```pascal
AJ_Code.Text := '0';
AJ_Code.Tag  := 0 ;
…
AJ_Code.ReadOnly := CodeCala > 0 ;      // :85
```

so a new item opens showing `0` and the operator overtypes it. Contrast every other numbering in
the module — `New_AnbarFactor` (`Dmu.pas:1253-1262`), `AC_ID` (`AnbarTanzimU.pas:216-219`),
`FM_Factor` (`FactorPesteh_U.pas:194-195`) — all of which use `Max(...)+1`. The item code is the
one identifier the system does not allocate.

Consequences:

- **Code `0` is accepted.** Nothing checks `AJ_Code > 0`. The duplicate check
  (`AnbarCalaAddU.pas:113-119`) will pass if no item `0` exists, and the insert proceeds.
- **An item with code `0` can never be edited.** `A_EditClick` (`AnbarCalaU.pas:143-156`) reads
  `J := AJ_Code` and calls `AnbarCalaAdd.init(I, J)`; `init` treats `CodeCala > 0` as "edit" and
  anything else as "new" (`:85-86`). So opening item `0` opens a **blank new-item form**, and
  saving it creates a second item. The only way out is direct SQL.
- **The code cannot be changed after creation.** `AJ_Code.ReadOnly := CodeCala > 0`. The author
  left the comment `//  Check for change code and factored` at `:120` — the check that would have
  been needed if the code were editable — but the branch is unreachable because the field is
  read-only. Renaming an item code requires SQL, and would have to update `Anbar_FactorD.AFD_Code`
  by hand.
- **The code space is global, not per warehouse.** The duplicate check queries all of
  `Anbar_Jens` with no `AJ_ID` filter, which is correct for a primary key but means warehouses
  cannot maintain independent numbering.

`AJ_Code` is a `TsCalcEdit` with `DisplayFormat = '########0;0;0'` (`AnbarCalaAddU.dfm:206`) —
up to nine digits, integer only.

---

### 2.2 Create / update — the validations

`TAnbarCalaAdd.B_SaveClick` (`AnbarCalaAddU.pas:110-202`), button captioned `ذخیره` ("save",
`AnbarCalaAddU.dfm:154`). Four validations, in order, each an `Application.MessageBox` with the
title literal `'Error'`:

| # | Rule | Persian message (verbatim) | English | `file:line` |
|---|---|---|---|---|
| 1 | The code must not already exist. Skipped when editing (the code is read-only, so `AJ_Code.Tag = AJ_Code.AsInteger`). | `'  کد تکراري است  '` | "The code is duplicate" | `AnbarCalaAddU.pas:113-119` |
| 2 | `Trim(AJ_Name)` must be non-empty | `'  نام کالا را وارد کنيد  '` | "Enter the item name" | `:122-127` |
| 3 | `AJ_Vahed.KeyValue > 0` — a unit of measure must be picked | `'  واحد شمارش را وارد کنيد  '` | "Enter the unit of measure" | `:128-133` |
| 4 | `AJ_Phi.AsInteger <> 0` — a sale price must be entered | `'   قيمت فروش را وارد کنيد  '` | "Enter the sale price" | `:134-139` |

Verbatim:

```pascal
    // Code
    if ( AJ_Code.Tag = 0 ) or (AJ_Code.Tag<> AJ_Code.AsInteger ) Then
    if Dm.AnbarJens.Locate('AJ_Code', inttostr(AJ_Code.AsInteger), [LoCaseInsensitive] ) Then
    Begin
        Application.MessageBox('  کد تکراري است  ' , 'Error' );
        ActiveControl := AJ_Code;
        Exit;
    End;
    //  Check for change code and factored

    if Length( Trim(AJ_Name.Text)) = 0 Then
    Begin
        Application.MessageBox('  نام کالا را وارد کنيد  ' , 'Error' );
        ActiveControl := AJ_Name;
        Exit;
    End;
    if AJ_Vahed.KeyValue <= 0 Then
    Begin
        Application.MessageBox('  واحد شمارش را وارد کنيد  ' , 'Error' );
        ActiveControl := AJ_Vahed;
        Exit;
    End;
    if AJ_Phi.AsInteger = 0 Then
    Begin
        Application.MessageBox('   قيمت فروش را وارد کنيد  ' , 'Error' );
        ActiveControl := AJ_Phi;
        Exit;
    End;
    // Control Data
```

The trailing comment `// Control Data` marks where further validation was intended and never
written.

#### 2.2.1 What is **not** validated

| Field | Missing rule | Effect |
|---|---|---|
| `AJ_Code` | `> 0` | §2.1 — an unmaintainable item |
| `AJ_Code` | uniqueness is checked against a **client-side cached dataset** (`Dm.AnbarJens`, opened at `:80-81`), not with a server round trip | Two users creating the same code simultaneously both pass. No unique constraint is known to exist (DDL absent — §14). |
| `AJ_Name` | uniqueness, max length | Two items may share a name. `@Name` is declared `Varchar(80)` (`:158`) but the item-search query and the grid make the name the primary way operators find an item. |
| `AJ_Phi` | `> 0` — only `<> 0` is checked | **A negative sale price is accepted.** `TsCalcEdit` permits a minus sign. |
| `AJ_Alarm` (min stock) | `>= 0` | A negative minimum is accepted (it is only displayed, §1.2) |
| `AJ_Prop` | anything | free text, `Varchar(50)` |
| `SSTID` (tax item code) | format, length, checksum | `Varchar(13)`; the Iranian tax-authority "شناسه کالا/خدمت" has a defined structure. Nothing checks it. |
| `AJ_ID` (warehouse) | that it exists | It is taken from the list screen's current `Anbar_Config` row (`AnbarCalaU.pas:133,147`), so it is always valid in practice — but the dialog trusts `AJ_ID.Tag` blindly (`:154`). |

> **Length-truncation defect.** `AJ_Name` is written as `Varchar(80)` (`AnbarCalaAddU.pas:158`),
> but when the name is copied onto an invoice line the stored-procedure parameter `@Name` is
> `varchar(50)` (`AnbarFactorU.dfm`, `SP_AnbarAddToFactor`). **An item name longer than 50
> characters is silently truncated on every invoice line and every printed invoice**, while the
> item master keeps the full 80. `@prop` and `@Vahed` are `varchar(50)` on both sides and are
> safe.

#### 2.2.2 Defaults for a new item

`init(CodeAnbar, 0)` (`AnbarCalaAddU.pas:59-85`):

| Field | Default | Note |
|---|---|---|
| `AJ_ID` | the warehouse currently selected on the list screen | display-only; `AJ_ID.Tag` carries the id |
| `AJ_Code` | `'0'` | §2.1 |
| `AJ_Name`, `AJ_Prop`, `SSTID` | empty | |
| `AJ_Vahed` | `KeyValue := -1` | forces validation 3 to fire until picked |
| `AJ_Phi` | `'0'` | forces validation 4 |
| `AJ_Alarm` | `'0'` | |
| `AJ_Maliat` (taxable) | `SliderOn := False` → **0, not taxable** | `:76` |
| **`AJ_Manfi` (allow negative stock)** | **`SliderOn := True` → 1, negative stock PERMITTED** | `:74` |

> **This is the confirmation of the data-layer finding, from the inventory side.**
> `AnbarCalaAddU.pas:74` — `AJ_Manfi.SliderOn := True;` — means **every item created through the
> UI defaults to allowing negative stock**, which switches off the only stock check in the system
> (§5.2.1, `if DM.Anbar_Jens_Phi1.fieldByName('AJ_Manfi').Asinteger <> 1 then`). The reported
> schema default `int NOT NULL DEFAULT 1` agrees with the UI default. Both are "permitted".
> The `§5.2.2` bullet claiming the column is not editable and its default unknown has been
> corrected in place.
>
> The control is a two-position slider captioned `بله` / `خیر` (yes/no,
> `AnbarCalaAddU.dfm:139-140`) under the label `منفی شدن موجودی` ("stock going negative",
> `:43`). So the label reads "stock going negative: **yes**" by default — the operator has to
> understand that "yes" means "permitted" and actively switch it off per item.

#### 2.2.3 Loading an existing item for edit

`init(CodeAnbar, CodeCala)` with `CodeCala > 0` (`:86-100`) reads its values from
**`DM.Anbar_AjnasView`**, not from the `DM.AnbarJens` row it just located:

```pascal
DM.AnbarJens.Locate('AJ_Code', inttostr(CodeCala), [LoCaseInsensitive] );   // :87 — result unused
AJ_Code.text := inttostr(CodeCala) ;
AJ_Code.Tag  := CodeCala;
AJ_Name.Text := DM.Anbar_AjnasView.FieldByName('AJ_Name').AsString;         // :90
…
```

> **Fragility.** `Anbar_AjnasView` is a shared `DM` dataset whose cursor position is set by the
> *caller* (`AnbarCalaU.pas:148` reads `AJ_Code` from the current row and then passes it in). The
> dialog therefore renders whichever row `Anbar_AjnasView` happens to be on, while the code box
> shows `CodeCala`. They agree only because the caller is the only entry point. Any second caller,
> or any intervening navigation on that global dataset, produces an editor showing item A's code
> and item B's name, price and flags — and saving it writes B's values onto A. The `Locate` at
> `:87` looks like the fix and is simply never used.

`AJ_Vahed.KeyValue := DM.Anbar_AjnasView.FieldByName('AJ_VahedC').Value` (`:93`) — note the
commented-out predecessor at `:92` set `AJ_Vahed.Text` instead.

> **Legacy-row hazard.** If `AJ_VahedC` is `NULL` on rows created before the `AV_Code` lookup was
> introduced, `KeyValue` becomes `Null`, the combo displays blank, and validation 3
> (`if AJ_Vahed.KeyValue <= 0`) compares a `Null` variant with an integer. Delphi's variant
> comparison of `Null` with `0` does not reliably yield `False` — it can raise
> `EVariantTypeCastError`. Either way **such an item cannot be re-saved without re-picking the
> unit**, and it may crash the form. `AJ_Vahed` (the denormalised text) would still hold the old
> label. **Open question §14** — check for `AJ_VahedC IS NULL OR AJ_VahedC = 0` in production.

---

### 2.3 The write

`AnbarCalaAddU.pas:143-177`. The dialog builds one batch of `Declare`s and then either an
`UPDATE` or an `INSERT` depending on `AJ_Code.Tag`:

```pascal
if AJ_Code.Tag>0 then
Begin  // Edit.
   Qs.SQL.Add('Update Anbar_Jens ');
   Qs.SQL.Add(' Set AJ_Maliat=@Maliat, AJ_Manfi=@Manfi, AJ_Phi=@Phi, AJ_Alarm=@Alarm ');
   Qs.SQL.Add('   ,AJ_UserID=@UserID, AJ_VahedC=@VahedC, AJ_Name=@Name, AJ_Prop=@Prop ');
   Qs.SQL.Add('   ,AJ_Vahed=@Vahed, AJ_Net=@Net, AJ_DateTime=GetDate(), SSTID=@SSTID ');
   Qs.SQL.Add(' Where AJ_Code=@Code')
End else Begin  // Append.
   Qs.SQL.Add(' insert Anbar_jens (AJ_Code, AJ_ID, AJ_Maliat, AJ_Manfi, AJ_Phi, AJ_Alarm, AJ_UserID, '+
                                   '  AJ_Name, AJ_Prop, AJ_Vahed,  AJ_Net, AJ_DateTime, SSTID, AJ_VahedC )');
   Qs.SQL.Add(' Values (@code, @ID, @Maliat, @Manfi, @Phi, @Alarm, @UserID, @Name, @Prop, @Vahed, @Net, GetDate(), @SSTID, @VahedC )');
End;
QS.ExecSQL;
```

Observations:

- **`AJ_ID` is not in the `UPDATE` list.** An item's home warehouse can be set at creation and
  **never changed from the UI**. Given §1.2's note that changing `AJ_ID` would retroactively
  re-attribute history and change the VAT rate, this is arguably deliberate — but it means the
  warehouse selector on the editor is decorative when editing.
- **The update is not scoped by `AJ_COID`** — correct, the item master is global (§1.0).
- **String values go through `QuotedStr`** (`:158-163`) and numerics through `inttostr`
  (`:153-157, 161`). This screen is not SQL-injectable, unlike the `_cala` interpolation at
  `FactorPesteh_U.pas:137` (§8.4).
- **`AJ_Net` records the workstation name**, `Util.GetComputerName` (`:162`) — the closest thing
  to an audit trail on any table in this application, alongside `AJ_UserID` and
  `AJ_DateTime = GetDate()`. Note `GetDate()` is the **server's Gregorian** timestamp, not a
  Jalali string — the only place in the inventory domain that stores a real timestamp.
- **No transaction, no error handling.** `QS.ExecSQL` then `Tag := 1; Close;` — a failed write
  reports success. Delphi would raise on an SQL error and the exception would escape to the
  application handler, so the form stays open; but nothing distinguishes "saved" from "not saved"
  for the caller, which unconditionally re-locates the grid (`AnbarCalaU.pas:139-140`).
- **The `TDataSet`-based implementation it replaced is still present, commented out** (`:179-199`)
  — twenty lines of `Dm.AnbarJens.Append` / `FieldByName(…) := …` / `Post`. Useful only as
  evidence that `AJ_DateTime` was once set client-side with `Date()` (`:197`) rather than
  server-side.

---


---

[← 1. Entity model](05-01-entity-model.md) | [index](00-index.md) | [2. Item master CRUD rules (part b) →](05-02-b-item-master-crud-rules.md)
