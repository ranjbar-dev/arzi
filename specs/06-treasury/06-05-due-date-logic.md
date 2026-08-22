_Part of [06-treasury](../06-treasury.md) — [index](00-index.md)_

## 5. Due-date logic

### 5.1 Where the due date lives

`DCheck.S_DateS` — a `varchar(50)` holding a Jalali date string (`Dmu.dfm:947-950`). It is the
**only** due date in the whole treasury module: issued cheques, deposit slips and petty-cash claims
have none (§3.3).

The screen control is `S_Dates: TFullDate` (`CheckDaryaftU.pas:22`), captioned `سررسید` "due date"
on every screen that displays it.

### 5.2 Which Jalali algorithm the treasury uses: **neither of the two in the repository**

Both known converters are dead code as far as treasury is concerned:

- `TUtil.FarsiDate` (`Utility.pas:435-442`) — arithmetically wrong *and* emits a **two-digit** year
  (`inttostr(AYear mod 100)`), producing `03/05/12`. Its companion `TUtil.IsFarsiDate`
  (`Utility.pas:526-541`) accordingly demands `Length(D) = 8`. **Never called from anywhere** — a
  repository-wide search for `FarsiDate(` returns only its own declaration and definition.
- `TDM.MiladiToShamsi` (`Dmu.pas:362-435`) — near-correct, emits a zero-padded **four-digit**
  `YYYY/MM/DD` (`Dmu.pas:430-434`). Also **never called from anywhere**.

Every treasury screen instead uses the `TFullDate` VCL control's own `.Farsi_Date`, `.Farsi_Valid`
and `.SetToDate(TDateTime)` members. `TFullDate` lives in the unit `Tools`, **whose source is not in
this repository** (only compiled). Its conversion algorithm, its leap-year rule and its exact output
format therefore cannot be verified from source. Everything below is inferred from how the strings
are compared and stored.

Empirically the format must be zero-padded `YYYY/MM/DD` (10 characters), because:

- `S_Date` is declared `varchar(10)` (`Dmu.dfm:943-946`);
- every date comparison is a **string** comparison against `Dm.From_Date` / `Dm.To_Date`, which come
  straight out of `Base.FromDate` / `Base.ToDate` as strings (`Dmu.pas:1137-1149`) and must be in the
  same format for `<` and `>` to mean anything;
- `Order By S_Dates` (`CheckListDU.pas:329`) is relied on to produce chronological order.

**This must be verified against the live database before the migration.** If any historical row was
written with a two-digit year or without zero padding, every range filter and every sort silently
misbehaves on that row. See §12.

### 5.3 How the due date is set

`CheckDaryaftU.ClearForm` (`:383`):

```pascal
S_Dates.SetToDate(Date()+1);
```

**The default due date is tomorrow** — the Gregorian system date plus one day, converted by the
control. There is no business rule behind it; it is a placeholder that guarantees a syntactically
valid value.

The operator can overwrite it with anything the control accepts. §9.1 records that `S_DateS` is
subject to **no validation whatsoever** — not format, not fiscal-year range (unlike `S_Date`, which
is range-checked at `CheckDaryaftU.pas:248-256`), not "after the receipt date". A cheque received on
`1403/05/12` may be stored with a due date of `1399/01/01` and the system will accept it and sort it
to the top of the list.

### 5.4 What the due date drives

**(a) Sort order.** The cheque list's only `ORDER BY` is `Order By S_Dates` (`CheckListDU.pas:329`).
Cheques are always presented in due-date order, oldest first, regardless of state, fiscal year, or
counterparty. The deposit-slip list uses the same clause (`FishListD.pas:231`) even though `DFish`
has no `S_DateS` in any declared field list — which is itself evidence that the physical `DFish`
table *does* carry an `S_DateS` column that this application never writes (§1.3, §12).

**(b) The aging filter.** `CheckListDU.ReopenQ1` (`:327`):

```sql
And ( S_DateS<= '<_Date>' ) and (S_State<4)
```

Semantics: "every cheque whose due date has arrived, that has not yet left the pipeline". `S_State < 4`
excludes returned (4) and cleared (5), leaving in-hand (1), at-bank (2) and the unreachable 3. This
is the system's entire concept of *aging* — a single as-of cutoff, no buckets, no 30/60/90 split, no
days-overdue computation anywhere.

**(c) The report title.** `Print1Click` (`CheckListDU.pas:267`):

```pascal
if Length(Trim(_Date))>0 then S := '  لیست چکهای سررسید شده تا تاریخ  ' + _Date ;
```

"List of cheques falling due up to date D".

**(d) Voucher narrations.** Every transition embeds the due date in its `Article` text:
`' سررسید ' + S_DateS` (`CheckDaryaftU.pas:324`, `CheckDaryaft2U.pas:181`,
`CheckBargashtu.pas:201`, `CheckEsterdadU.pas:180`) and in the delete confirmation
(`CheckDaryaftU.pas:431`).

**(e) Read-only display** on the deposit, bounce, collect and return screens, as a plain `TsEdit`
loaded from the row (`CheckDaryaft2U.pas:107`, `CheckBargashtu.pas:114`, `CheckVosoolU.pas:116`,
`CheckEsterdadU.pas:107`). The due date can only be changed by editing the cheque itself, which
requires state 1 and the original fiscal year (§9.2 rules 19-20).

### 5.5 **The aging filter is unreachable from the UI**

`_State`, `_Name` and `_Date` are private fields of `TCheckListDF` (`CheckListDU.pas:138-140`). They
are assigned in exactly two places, and both **reset them to nothing**:

- `init` — `_State := 0; _Name := ''; _Date := '';` (`CheckListDU.pas:248-250`)
- `S_SearchClick` — `_State := 0; _Name := ''; _Date := '';` (`CheckListDU.pas:589-591`)

Nothing else in the unit writes them. The controls that were clearly meant to drive them —
`State1`…`State5` (each carrying its state code in its `Tag`), `B_SarResid` (the due-date button,
`ImageIndex = 41`) and `B_Names` (the counterparty search button) — **have no `OnClick` handler**;
the form's complete handler inventory is `S_Search`, `S_Print`, `GridFontSize`, `S_Edit`,
`S_Delete`, `S_New`, `sBitBtn4`, `S_Bargasht`, `S_Vosool`, `S_BBank`, `S_Bank`, `Print1`
(`CheckListDU.dfm:241, 370, 550, 575, 591, 606, 621, 636, 664, 679, 694, 942`).

Consequences:

- The cheque list **always shows every cheque of every state and every fiscal year**, sorted by due
  date. There is no filtering at all.
- The aging query at `:327` never executes.
- `Print1Click` always falls through to the default title `' لیست همه چکها'` "list of all cheques"
  (`CheckListDU.pas:261`); the five state-specific titles and the due-date title are unreachable.
- The state-based row colouring is doubly dead: `G1DrawColumnCell` begins with `Exit;`
  (`CheckListDU.pas:208`) before ever reaching the colour map at `:213-219`.

The same is true of the deposit-slip list, where the filter lines are literally commented out
(`FishListD.pas:227-229`).

**There are no due-date alerts of any kind** — no startup reminder, no dashboard tile, no badge, no
scheduled job. A cheque falling due today is indistinguishable from one falling due next year except
by reading the sorted grid.

### 5.6 Implications of Jalali string storage for the rebuild

| Issue | Consequence today | What the rebuild must do |
|---|---|---|
| Dates stored as text, compared lexicographically | Correct **only** while every value is exactly `YYYY/MM/DD` zero-padded. One malformed legacy row silently sorts and filters wrongly, with no error. | Store a real `DATE` (Gregorian) as the source of truth; render Jalali in the UI layer. Keep the original string in a `*_jalali_raw` column through the migration so bad rows can be found. |
| `S_DateS` is `varchar(50)`, not `varchar(10)` | Trailing spaces or a longer string are accepted and break both `ORDER BY` and `<=`. | Constrain at the type level. |
| No validation on entry | Due dates outside the fiscal year, before the receipt date, or nonsensical, are all persisted. | Validate: parseable Jalali, and ≥ the receipt date. Whether it must lie inside the fiscal year is a business question — see §12. |
| Aging is a single `<=` cutoff | No overdue-days, no buckets, no "due within N days". | Compute from a real date. Buckets are a §13 proposal, not a port requirement. |
| The conversion algorithm is inside a binary-only VCL control | Migration cannot reproduce the legacy conversion exactly; round-tripping historical strings is the only safe path. | **Migrate the strings, do not recompute them.** Parse `YYYY/MM/DD` as Jalali with a correct algorithm and convert forward; flag every row that fails to parse rather than guessing. |
| Two incompatible converters exist in the source and neither is used | Nothing today, but they are a trap for anyone porting `Utility.pas` or `Dmu.pas`. | Port neither. Use a maintained Jalali library. |


---

[← 4. Endorsement / transfer to a third party](06-04-endorsement-transfer-third-party.md) | [index](00-index.md) | [6. Deposit slips (Fish) →](06-06-deposit-slips-fish.md)
