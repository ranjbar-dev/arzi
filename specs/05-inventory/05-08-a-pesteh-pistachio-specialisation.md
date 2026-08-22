_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 8. The Pesteh (pistachio) specialisation

This is the only product-specific vertical in the application. It is **not** an extension of the
generic invoice (§3, §4): it is a second, parallel purchasing pipeline that bypasses
`Anbar_Factor` / `Anbar_FactorD` entirely, writes straight into subsystem B
(`Anbar.Dbo.FactorMaster` / `FactorDetail`, §5.0) and into the ledger, and is driven by data
produced by a **third** system — the weighbridge (`Rppc_Solution.Dbo.NewRamz`).

### 8.0 There are two pistachio implementations, and only one of them is reachable

| | Implementation **P1** — "Kharid" | Implementation **P2** — "FactorPesteh" |
|---|---|---|
| Units | `Kharid_U.pas` + `PestehD_U.pas` (line dialog) + `Kharid_BU.pas` (account settings) | `FactorPesteh_U.pas` |
| Data source | Manual keying of every weight and percentage | Read-only from `Rppc_Solution.Dbo.NewRamz` (the weighbridge) |
| Persistence | **None.** See below. | `Anbar.Dbo.FactorMaster` + `FactorDetail` + `Moein` + `NewRamz` state update |
| Deduction maths | Full — moisture, blanks, tare, other (`PestehD_U.pas:118-139`) | **None in this repo.** The net weight `NR_Vazn` arrives already computed by the weighbridge application. |
| Reachable from the UI? | **No** — see §8.0.1 | Yes — `Mainu.pas:511-514`, tab `TS_Pesteh` (`عملیات خرید پسته`, "pistachio purchase operations") |

Both must be documented: **P2 is what runs**, but **P1 is the only place in the repository where
the deduction formula is written down**, and the weighbridge application's source is not available.
P1 is therefore the de-facto specification of the arithmetic that P2 consumes as a finished number.

#### 8.0.1 P1 is dead code — three independent reasons

1. **Its menu buttons live on a permanently hidden panel.** `Mainu.dfm:102-128` declares
   `Panel3: TsPanel` with `Visible = False`, containing `Button4` (`اطلاعات پایه`, "base data" →
   `Kharid_B.init`, `Mainu.pas:367-370`) and `Button8` (`خرید پسته`, "pistachio purchase" →
   `Kharid.New_Kharid`, `Mainu.pas:362-365`). `Panel3` is never referenced in `Mainu.pas` other
   than in its own field declaration (`Mainu.pas:19`); nothing ever sets `Panel3.Visible := True`.
2. **The form has no save handler.** `TKharid` declares `B_Save: TButton` (`Kharid_U.pas:12`) but
   `Kharid_U.dfm` assigns `OnClick` to only four controls — `B_CancelClick`, `Button1Click`,
   `Button2Click`, `Button3Click` (`Kharid_U.dfm:110,303,312,321`). `B_Save` has **no `OnClick`**,
   and neither do `KH1`…`KH4`, `Button8`, `Button9` (`Kharid_U.pas:34-39`). Even if the panel were
   visible, the screen could add, edit and delete lines in the in-memory array `KharidList`
   (`Kharid_U.pas:57`) and then lose all of it on close.
3. **Nothing writes the record type anywhere.** `KharidRec` (`PestehD_U.pas:9-28`) exists only as
   `Var KharidList : Array[1..10] Of KharidRec` (`Kharid_U.pas:57`) and the global
   `Data : KharidRec` (`PestehD_U.pas:81`). No SQL in the repository mentions any of its fields.

`Kharid_BU` (the account-code settings, §8.5) is reachable only through the same hidden `Panel3`,
so the eight `Base.Kh*_Code` account slots it maintains **can only have been set before the panel
was hidden, or by direct SQL**. This matters because P2 does *not* read them — it hard-codes its
account prefixes instead (§8.4).

---

### 8.1 `Kind_Table` — pistachio product grades

`Dmu.pas:20` declares `Kind_Table: TADOTable`; `Dmu.dfm:301-307` binds it to
`TableName = 'Kinds'` on the main `Ado` connection with no filter, no index and no parameters.
Columns actually used: `K_id` (key) and `K_name` (display) — `PestehD_U.dfm:261-263`.

**It is not an account-type table.** Nothing in the accounting units reads it; the only consumer
in the entire repository is `PestehD_U.pas:94-96` (`Dm.Kind_Table.Active := True;
Kind.KeyValue := Data.KindId`), which is inside dead form P1. The glossary's note
(`docs/01-glossary.md` §6b) is confirmed and the pointer to `Sarfasl_SelectU.pas` is wrong —
that unit contains no reference to `Kind_Table` or `Kinds`.

The grade ids are enumerated in a source comment on the live path,
`FactorPesteh_U.pas:132-133`:

```pascal
// test code mojood dar anbar
// anbar=17 and 1=fandoghi  2=badami  3=kallehghoochi  4=momtaz  5=ahmadaghaei  6=akbari +7=dahanbast
```

| `K_id` | Transliteration | Persian | English (pistachio cultivar / grade) |
|---|---|---|---|
| 1 | fandoghi | فندقی | Fandoghi ("hazelnut" — round) |
| 2 | badami | بادامی | Badami ("almond-shaped" — long) |
| 3 | kallehghoochi | کله‌قوچی | Kalleh-Ghouchi ("ram's head" — jumbo round) |
| 4 | momtaz | ممتاز | Momtaz ("premium") |
| 5 | ahmadaghaei | احمدآقایی | Ahmad-Aghaei (long, thin) |
| 6 | akbari | اکبری | Akbari (long, premium) |
| 7 | dahanbast | دهان‌بست | Dahan-Bast (closed-shell / non-split) |

The list is *not* read from `Kinds` on the live path — `K_id` values are used as literals in
strings. The `Kinds` table row set is therefore not verifiable from source (**open question §14**);
the comment above is the only enumeration in the repository. Note the comment's `+7=dahanbast`
suggests 7 is a residual bucket appended later.

#### 8.1.1 One integer plays three roles

`NR_Kind` (the grade id) is used simultaneously as:

| Role | Where | Code |
|---|---|---|
| **Item code** in the external warehouse item master | `FactorPesteh_U.pas:137` | `Select * From <Anbar>.cala where C_code='+ _cala +' and ( C_Anbar like ''%,17,%'')` where `_cala := Q1.FieldByName('NR_Kind').AsString` (`:119`) |
| **Item code** on the stock movement line | `FactorPesteh_U.pas:209` | `Select 1, 17, @FmSSN, NR_Kind, …` → `FD_Code` |
| **Third segment of the purchase account code** | `FactorPesteh_U.pas:146` | `BedCode := '700-3-'+ Q1.FieldByName('NR_Kind').AsString;` |

So grade 5 (Ahmad-Aghaei) is item `5` in warehouse `17` *and* analytic account `700-3-5`.
There is no mapping table; the identity is by convention only. Any renumbering of grades silently
repoints both the stock ledger and the general ledger.

**Rebuild consequence:** three separate concepts (`pistachio_grades.id`, `items.id`,
`accounts.code`) that must be explicitly related by foreign key, not by shared integer.

---

### 8.2 The deduction formula (implementation P1 — the specification of the arithmetic)

`PestehD_U.pas:118-139`, `TPestehD.BascVChange`. This single handler is wired to the `OnChange`
of every input on the form (`PestehD_U.dfm:313,326,339,352,407,448` and `AdlV`'s
`OnChange`/`OnCloseUp`/`OnExit` at `:475-477`), so it recomputes on every keystroke. Quoted
verbatim:

```pascal
procedure TPestehD.BascVChange(Sender: TObject);
begin
    if AdlV.ItemIndex =0 Then Adlk.FloatValue := Adl.IntValue * 0.1 ;
    if AdlV.ItemIndex =1 Then Adlk.FloatValue := Adl.IntValue * 0.2 ;
    if AdlV.ItemIndex =2 Then Adlk.FloatValue := Adl.IntValue  ;
    PookK.FloatValue := Pook.FloatValue * BascV.FloatValue / 100 ;
    Rotk.FloatValue  := Rot.FloatValue  * BascV.FloatValue / 100 ;
    Kasr.FloatValue  := Adlk.FloatValue + PookK.FloatValue +
                        RotK.FloatValue + Sayer.FloatValue ;
    Nabv.FloatValue := BascV.FloatValue - Kasr.FloatValue ;
    if BascV.FloatValue < Kasr.FloatValue Then NabV.FloatValue := 0;
    Kol.IntValue := Round(NabV.FloatValue * Phi.IntValue );
    …
end;
```

#### 8.2.1 Field dictionary

Persian captions from `PestehD_U.dfm` (form caption `مشخصات پسته`, "pistachio specification").

| Control | `.dfm` line | Persian caption | English | Type / precision | Role |
|---|---|---|---|---|---|
| `Kind` | `:256-265` | `نوع پسته` | Pistachio type | `TDBLookupComboBox` over `Kinds` | Grade. **Mandatory** — caption is red (`:31`) |
| `Ons` | `:266-277` | `انس` | Ounce (count per ounce) | decimal 3.2 | Descriptive only — **never used in any formula** |
| `Dahan` | `:278-289` | `دهن بست` | Closed-shell fraction | decimal 3.2 | Descriptive only — **never used in any formula** |
| `Garam` | `:290-301` | `گرم مغز` | Kernel grams | decimal 3.2 | Descriptive only — **never used in any formula** |
| `Adl` | `:302-314` | `تعداد` | Count (of bales/sacks) | integer 4 | Tare driver. Red caption = mandatory (`:88-93`) |
| `AdlV` | `:464-482` | items `100 گرم` / `200 گرم` / `یک کیلو` | Tare allowance per bale: 100 g / 200 g / 1 kg | combo, `csDropDownList` | Selects the 0.1 / 0.2 / 1.0 multiplier |
| `Rot` | `:315-327` | `درصد رطوبت` | Moisture percentage | decimal 3.2 | % of gross |
| `Pook` | `:328-340` | `درصد پوک` | Blank/empty-shell percentage | decimal 3.2 | % of gross |
| `BascV` | `:341-353` | `وزن باسکول` | Weighbridge (gross) weight, kg | decimal 6.1 | **Mandatory**, red (`:133-138`) |
| `Adlk` | `:354-367` | `کسر ظرف` | Container (tare) deduction, kg | decimal 6.1, `ReadOnly` | Derived |
| `RotK` | `:368-381` | `کسر رطوبت` | Moisture deduction, kg | decimal 6.1, `ReadOnly` | Derived |
| `PookK` | `:382-395` | `کسر پوک` | Blank deduction, kg | decimal 6.1, `ReadOnly` | Derived |
| `Sayer` | `:396-408` | `سایر کسورات` | Other deductions, kg | decimal 6.1 | Entered directly in **kg**, not % |
| `Kasr` | `:409-422` | `جمع کسورات` | Total deductions, kg | decimal 6.1, `ReadOnly` | Derived |
| `NabV` | `:423-436` | `خالص وزن` | Net weight, kg | decimal 6.1, `ReadOnly` | Derived. Red = must be > 0 |
| `Phi` | `:437-449` | `بهای واحد` | Unit price (rial/kg) | integer 8 | **Mandatory**, red |
| `Kol` | `:450-463` | `مبلغ کل` | Total amount (rial) | integer 15, `ReadOnly` | Derived |
| `BSave` | `:483-491` | `تایید` | Confirm | | → `BSaveClick` |
| `BCancel` | `:492-500` | `برگشت` | Back / cancel | | → `BCancelClick` |

#### 8.2.2 The formulas

```
tare_allowance_per_bale  ∈ { 0.1, 0.2, 1.0 }        kg   (AdlV.ItemIndex → 0 / 1 / 2)

tare_deduction           = bale_count × tare_allowance_per_bale          kg
moisture_deduction       = moisture_pct  × gross_weight / 100            kg
blank_deduction          = blank_pct     × gross_weight / 100            kg
total_deductions         = tare_deduction + moisture_deduction
                         + blank_deduction + other_deductions            kg
net_weight               = gross_weight − total_deductions               kg
if gross_weight < total_deductions then net_weight = 0
line_amount              = round( net_weight × unit_price )              rial
```

Properties that must be preserved exactly:

- **Percentages apply to the gross weight, not sequentially.** Moisture and blanks are *not*
  compounded (`gross × (1−r) × (1−p)`); both are computed off the same `BascV` base and then
  added. With 3.5 % moisture and 2 % blanks on 2 000 kg the deduction is exactly 110 kg, not
  109.3 kg. This is an arithmetic decision with money consequences and is easy to "fix" wrongly.
- **`Sayer` (other deductions) is in kilograms, not a percentage** — it is added to `Kasr`
  untransformed (`PestehD_U.pas:126`).
- **Only the net weight is floored at zero, not the deduction total.** `Kasr` can exceed `BascV`;
  the line then values at zero (`Kol = 0`) but `Kasr` still displays the over-deduction.
  There is no error, no warning and no block — `BSaveClick` (`:141-164`) has **no validation at
  all**, it copies every field into `Data` and closes.
- **The three "red" mandatory markers are cosmetic only.** `PestehD_U.pas:131-138` recolours the
  labels `L_Adl`, `L_BascV`, `L_Phi`, `L_Nabv` red when their value is ≤ 0, but nothing prevents
  saving. The only guard is on the caller side: `Kharid_U.pas:182` accepts the line back only
  `if Data.KindId > 0`.
- **`Ons`, `Dahan` and `Garam` are captured, stored in `KharidRec` and displayed in the grid
  (`Kharid_U.pas:238-239`) but never enter the price.** They are quality attributes carried for
  the printed document. `Garam` is not even shown in the grid.
- **Rounding is banker's-neutral Delphi `Round`** — i.e. round-half-to-even on the `.5` boundary,
  not round-half-up. `Round(2.5) = 2` and `Round(3.5) = 4` in Object Pascal. In PostgreSQL,
  `round(numeric)` is round-half-away-from-zero. **This is a real behavioural difference**
  (§15).

#### 8.2.3 Worked arithmetic

**Example A — ordinary lot.**

| Input | Value |
|---|---|
| grade | 5 (Ahmad-Aghaei) |
| bale count `Adl` | 40 |
| tare allowance `AdlV` | index 1 → 0.2 kg |
| gross `BascV` | 2 000.0 kg |
| moisture `Rot` | 3.5 % |
| blanks `Pook` | 2.0 % |
| other `Sayer` | 5.0 kg |
| unit price `Phi` | 1 250 000 rial/kg |

```
tare      = 40 × 0.2          =     8.0 kg
moisture  = 3.5 × 2000 / 100  =    70.0 kg
blanks    = 2.0 × 2000 / 100  =    40.0 kg
other     =                        5.0 kg
Kasr      = 8 + 70 + 40 + 5   =   123.0 kg
NabV      = 2000 − 123        = 1 877.0 kg
Kol       = round(1877 × 1 250 000) = 2 346 250 000 rial
```

**Example B — the deduction floor.** Same lot but a wet, blank-heavy delivery:
`BascV = 500.0`, `Rot = 60`, `Pook = 45`, `Adl = 40 @ 1.0 kg`, `Sayer = 0`.

```
tare      = 40 × 1.0          =    40.0 kg
moisture  = 60 × 500 / 100    =   300.0 kg
blanks    = 45 × 500 / 100    =   225.0 kg
Kasr      = 565.0 kg          >   BascV = 500.0
NabV      = 0                 (forced by PestehD_U.pas:128)
Kol       = 0
```

The screen shows `جمع کسورات = 565.0` against `وزن باسکول = 500.0` and a zero amount, and lets
you save it.

**Example C — rounding divergence.** `NabV = 1 877.5`, `Phi = 3` → Delphi `Round(5632.5) = 5632`
(half-to-even); PostgreSQL `round(5632.5) = 5633`. Off by 1 rial per line, systematically, on
every exact-half product.

#### 8.2.4 Overflow risk on `Kol`

`KharidRec.Kol` is `int64` (`PestehD_U.pas:27`) but is assigned from `Kol.IntValue`
(`PestehD_U.pas:162`), a property of the binary-only `Tools.TEditInt` whose declared width is
**not verifiable from source** — `Tools.pas` / `Tools.dcu` is not in the repository (only
`Lib.inc` is). Example A already produces 2 346 250 000 rial, which exceeds the 32-bit signed
maximum of 2 147 483 647. If `IntValue` is `Integer`, that line silently wraps negative.
Circumstantial evidence that it is 64-bit: `Kharid_U.pas:246` assigns an `int64` sum into
`TotalM.IntValue` without a cast, and the corresponding database column on the live path is
`TLargeintField` (`FactorPesteh_U.dfm:511-514`, `NR_Kol`). **Open question §14** — resolve by
inspecting the compiled `Tools` unit or by testing a >2.1 G rial lot.

---


---

[← 7. Pricing (part b)](05-07-b-pricing.md) | [index](00-index.md) | [8. The Pesteh (pistachio) specialisation (part b) →](05-08-b-pesteh-pistachio-specialisation.md)
