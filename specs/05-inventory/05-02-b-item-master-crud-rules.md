_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 2.4 Delete

`TAnbarCala.A_DeleteClick` (`AnbarCalaU.pas:169-194`), button captioned `حذف` ("delete",
`AnbarCalaU.dfm:84`).

```pascal
if Dm.Anbar_AjnasView.RecordCount = 0 then Exit;
I:= DM.Anbar_AjnasView.FieldByName('AJ_Code').AsInteger;
Dm.Q1.SQL.Add('Select * From Anbar_FactorD Where AFD_Code='+inttostr(i) );
Dm.Q1.Open;
if Dm.Q1.RecordCount >0 Then
Begin
   Application.MessageBox('با اين کد فاکتور صادر شده است','Error');
   …Exit;
End;
Dm.Q1.SQL.Add('Delete Anbar_Jens Where AJ_Code='+ inttostr(i) );
Dm.Q1.ExecSQL;
```

| Rule | Persian message | English |
|---|---|---|
| The item must never have been used on an invoice line | `'با اين کد فاکتور صادر شده است'` | "An invoice has been issued with this code" |

**Findings:**

1. **There is no confirmation dialog.** One click on `حذف` with a row selected permanently
   deletes the item. Every other destructive action in the module asks
   (`AnbarListU.pas:332`, `FactorPesteh_U.pas:166`, `SodoorSanadU.pas:240`). This one does not.
2. **It is a hard delete.** There is no `is_active` flag on `Anbar_Jens` (§1.2), so the only way
   to retire an item is to delete it — and an item that has ever been invoiced can never be
   deleted. **There is therefore no way to retire a used item at all.** It stays in the search
   list, the code picker and the balance report forever. This is a real operational gap and the
   strongest argument for adding `is_active` (§15).
3. **The usage check is correctly unscoped by fiscal year** (`Where AFD_Code=<n>` with no
   `AFD_Coid`), so prior years protect the item.
4. **The usage check covers only subsystem A.** `Anbar.Dbo.FactorDetail` is not consulted — which
   is right, because `Cala` and `Anbar_Jens` are separate item masters (§1.6), but worth stating.
5. **The post-delete navigation is broken.** `:189-193`:
   ```pascal
   Dm.Anbar_AjnasView.Next;
   I:= DM.Anbar_AjnasView.FieldByName('AJ_Code').AsInteger;
   Dm.Anbar_AjnasView.Close;
   Dm.Anbar_AjnasView.Open;
   Dm.Anbar_AjnasView.Locate('', inttostr(I), [LoCaseInsensitive]);
   ```
   `Locate` is called with an **empty key-field list**. That is not a valid `TDataSet.Locate` call;
   at best it does nothing, at worst it raises. The intent was `Locate('AJ_Code', …)`, as written
   correctly three lines away at `:140` and `:155`. Also, `Next` past the last record leaves the
   dataset in EOF state and `AJ_Code` reads as `0`.
6. **Uses the shared `Dm.Q1`** for both the check and the delete, leaving it closed afterwards —
   `Dm.Q1` is used by half the application (`Dmu.pas:1022, 1258, 1464, …`), so this is a global
   side effect.

---

### 2.5 The list screen — `AnbarCalaU`

Form caption `انبار کالا` ("goods warehouse", `AnbarCalaU.dfm:5`).

`init(AnbarCode)` (`AnbarCalaU.pas:51-82`):

1. Opens `Anbar_Config`. If empty: `ShowMessage('انبار را تعريف کنيد')` — **"define a warehouse"**
   — and exits. So the item master is unusable until at least one warehouse exists.
2. **Rebuilds `PopupMenu1` at runtime**, one item per warehouse, captioned
   `'<AC_ID> : <AC_Name>  '`, `Tag := AC_ID`, `OnClick := PopClick` (`:63-74`). The design-time
   menu item `T1` (`AnbarCalaU.pas:15`) is destroyed by the `Items.Delete(0)` loop at `:63`.
3. Positions on the requested (or first) warehouse and calls `AnbarLocate`, which sets the
   selector caption and reopens `Anbar_AjnasView(@ID := <warehouse>)` (`:116-128`).

So **the list is always filtered to exactly one warehouse** and there is no "all warehouses" view.

| Button | Persian caption | English | Handler | Status |
|---|---|---|---|---|
| `A_Select` | *(caption is the warehouse name, set at runtime)* | Warehouse selector | `A_SelectClick` → popup | works |
| `A_Add` | `کالای جدید` | New item | `A_AddClick` (`:130-141`) | works |
| `A_Edit` | `اصلاح` | Correct / edit | `A_EditClick` (`:143-156`); also `G1DblClick` (`:163-167`) | works |
| `A_Delete` | `حذف` | Delete | `A_DeleteClick` (`:169-194`) | works — §2.4 |
| `A_Exit` | `برگشت` | Back | `A_ExitClick` | works |
| **`A_Resome`** | **`سابقه`** | **History** | **none** | **DEAD — declared at `AnbarCalaU.pas:22`, no `OnClick` in `AnbarCalaU.dfm:103-118`, no handler in the `.pas`.** An item-history feature was planned and never built. |

**Grid columns** (`AnbarCalaU.dfm:145-260`), over `Anbar_AjnasView`:

| Field | Persian title | English | Width |
|---|---|---|---|
| `AJ_Code` | `کد کالا` | Item code | 75 |
| `AJ_Name` | `نام` | Name | 307 |
| `AJ_Prop` | `مشخصه  فنی` | Technical specification | 125 |
| `AJ_vahed` | `واحد` | Unit | 112 |
| `AJ_PhiS` | `قیمت` | Price (pre-formatted by the view) | 76 |
| `AJ_Maliat` | `مالیات` | Taxable | 69 |
| `AJ_Manfi` | `موجودی منفی` | Negative stock | — |
| `SSTID` | `شناسه` | Tax identifier | — |

**No stock quantity is shown on the item list.** To see on-hand for an item the operator must run
`Anbar_MandehU` (§11.2) or open a line editor (§7.1).

**No permission checks.** Unlike `AnbarListU` (§4.2.1), which gates seven buttons on
`Dm.IsEnabel`, `AnbarCalaU` gates nothing — any user who can reach the menu item
(`Mainu.pas:539-542`, `Anbar_AjnasClick`) can create, edit and delete items.

---

### 2.6 Item search — `AnbarCalaSelectU`

The picker used by the invoice line editor (`AnbarFactorAddU.pas:195-202`) and the stock card
(`AnbarCardJensiU.pas:71-78`). Form has one text box `S_name`, a grid, OK and Cancel.

`S_nameChange` (`AnbarCalaSelectU.pas:68-74`) re-runs `Dm.AnbarCala_SeekName` on **every
keystroke**, with no debounce:

```sql
Declare @Name Varchar(20)
Set @Name = '%'+Ltrim(RTrim(:Name))+'%'

Select *
From Anbar_Jens
Where ( ( PATINDEX ( @Name , AJ_Name ) > 0 )  or ( Len(@Name) = 2  ) )
Order by AJ_ID, AJ_Code
```
(`Dmu.dfm:604-628`)

| Finding | Detail |
|---|---|
| **The search term is truncated to 18 characters** | `@Name` is `Varchar(20)` and holds `'%' + term + '%'`, so anything past 18 characters of the term is cut. Typing a long item name narrows the search and then silently stops narrowing. |
| **`Len(@Name) = 2` is the "show everything" case** | `'%%'` has length 2, i.e. an empty search term. Written as a magic number rather than `Len(term) = 0`. |
| **`PATINDEX` means the term is a pattern, not a literal** | A user typing `%`, `_`, `[` or `]` gets wildcard behaviour. `50%` matches everything from `50` onward. Not an injection (the value is a bound parameter) but a usability trap. |
| **`Ltrim(RTrim(...))` trims, but the parameter is declared `Size = 1`** in the `.dfm` (`Dmu.dfm:608`) | ADO widens it at bind time from the assigned value, so this is inert — but it is the kind of design-time artefact that makes the declared metadata untrustworthy. |
| **Not filtered by warehouse** | `Order by AJ_ID, AJ_Code` groups by warehouse but shows all of them, unlike the maintenance list (§2.5). |
| **No `AJ_Alarm`/stock context** | The picker shows master data only. |
| `BitBtn1.Enabled` is driven by `RecordCount > 0` (`:73`) | correct |
| `NewDbGrid1DblClick` → `BitBtn1Click` (`:88-91`) | double-click selects |

---

### 2.7 Summary of CRUD rules to port

```
create(item):
    require code <> existing code            -- "کد تکراري است"
    require trim(name) <> ''                 -- "نام کالا را وارد کنيد"
    require unit_of_measure_id > 0           -- "واحد شمارش را وارد کنيد"
    require sale_price <> 0                  -- "قيمت فروش را وارد کنيد"
    warehouse := the warehouse being browsed
    is_taxable := false
    allow_negative_stock := TRUE             -- the legacy default
    stamp updated_by, updated_from_host, updated_at

update(item):
    same validations except the duplicate check (code is immutable)
    warehouse is NOT updatable

delete(item):
    reject if any inventory_document_line references it   -- "با اين کد فاکتور صادر شده است"
    otherwise hard delete, with NO confirmation
```

Everything in §2.2.1 (the missing validations), §2.1 (code `0`, no auto-numbering), §2.4
(no confirmation, no retirement path) and §2.5 (`A_Resome` dead, no permissions) is a candidate
for §15 — but the default is port-as-is, including `allow_negative_stock := true`.


---

[← 2. Item master CRUD rules (part a)](05-02-a-item-master-crud-rules.md) | [index](00-index.md) | [3. Document types (part a) →](05-03-a-document-types.md)
