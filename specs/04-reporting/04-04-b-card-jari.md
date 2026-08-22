_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 4.8 Buttons — every one

| Control | Caption / hint | Handler | Live? | What it does |
|---|---|---|---|---|
| `Report1` | `مشاهده دفتر` ("view ledger", `.dfm:6059`) | `Report1Click` (`:384-392`) | yes, gated by key 1123 | `DMoeinF.init(S_Ko, S_Mo, S_Ta1, S_Ta2, COID.KeyValue)` for the **current row only** — ignores the multi-select |
| `Report2` | `مشاهده تجمیعی` ("view consolidated", `.dfm:6232`) | `Report2Click` (`:394-430`) | yes, **ungated** | builds an `OR` predicate over the **selected** rows and calls `TMoeinF.init` |
| `sSpeedButton1` | `?` (`.dfm:5839`) | `sSpeedButton1Click` (`:437-443`) | yes | party picker → `Sahamdar.init2` |
| `B_Close` | hint `چاپ` = **"print"** (`.dfm:6248`), image index 4 | `B_CloseClick` (`:166-169`) | yes | **closes the form.** The hint is wrong; there is no print in this unit |
| `SP_Size` | — | `SP_SizeClick` (`:432-435`) | **dead** | `Visible = False` (`.dfm:6078`) *and* the single line of the handler is commented out |
| `POP_Size` / `Size6..Size15` | `سایز 6` … `سایز 15` | `Size6Click` (`:378-382`) | **unreachable** | handler works and persists to INI, but the only thing that would pop the menu is `SP_SizeClick`, which is commented out |
| `GridFontSize: TsUpDown` | — | `GridFontSizeChangingEx` (`:225-231`) | yes | the surviving font control |
| `Splitter2`, `sSplitter2` | — | — | cosmetic | — |
| `G1` double-click | — | `G1DblClick` (`:214-223`) | yes | **toggles selection**, not drill-down — see below |

### 4.9 Drill-down and the "inline editing" question

`G1DblClick` (`:214-223`):

```pascal
if VT1.Active=false then exit;
if VT1.RecordCount=0 then exit;
Vt1.Edit;
if G1.IsActiveSelected then G1.DeSelectActive else G1.SelectActive;
```

Double-click **toggles row selection**; it does not open anything. Every other grid in the reporting
surface opens the voucher on double-click (`DKolU.pas:165`, `DMoein.pas:294`, `TMoein.pas:153`), so
this is a consistency trap for users.

**`Vt1.Edit` at `:218` is the only "inline editing" in the unit** and it is spurious: it puts the
in-memory `TVirtualTable` into edit state purely so the selection toggle can run, and is never
matched by a `Post` or `Cancel`. Because `VT1` is a `TVirtualTable` — memory only, with no
`Connection`, no `UpdateObject`, no linkage to `Moein` — **nothing can reach the database from it.**
The grid is *technically* editable (the `Options` set at `.dfm:36` does not include `dgReadOnly`, and
`ReadOnly` is not set on `G1` or on any `VT1` field), so a user can type over `G_Bed` in place; the
edit lives until the next `ClearForm` and is invisible to everyone else. It also silently corrupts the
footer `%Sum` values and, if done before pressing `مشاهده تجمیعی`, has **no** effect on the child
query — `Report2Click` reads `S_Ko`/`S_Mo`/`S_Ta1`/`S_Ta2`, not the amounts.

Drill-down proper:

**`Report1Click` (`:384-392`)** — current row only:
```pascal
DMoeinF.init( Vt1.FieldValues['S_Ko'], Vt1.FieldValues['S_Mo'],
              Vt1.FieldValues['S_Ta1'], Vt1.FieldValues['S_Ta2'], Coid.KeyValue );
```
No date arguments, so `DMoein` defaults to the selected year's `Base.FromDate`..`ToDate` (§3.2).
Note `S_Ko` is a **string** field (`.dfm:6312-6314`) while `DMoeinF.init` takes an `integer` — the
`Variant` coercion works only because the value is always numeric text.

**`Report2Click` (`:394-430`)** — the multi-select handoff, and the source of `TMoein`'s injected
predicate:
```pascal
I := Vt1.FieldByName('S_SSN').AsInteger;   // remember cursor
VT1.First;
for J := 1 to vt1.RecordCount do
Begin
   if G1.IsActiveSelected then
   Begin
      _SN := _SN + Vt1.FieldByName('S_Name').AsString + CRLF;
      _SC := _SC + VT1.FieldByName('S_R').AsString + CRLF;
      if Length(_SW) > 0 then _SW := _SW + ' OR ';
      _SW := _SW + '(M_Ko='+ S_Ko +' and M_Mo='+ S_Mo +
                   ' and M_Ta1='+ S_Ta1 +' and M_Ta2='+ S_Ta2 + ' )' + CRLF;
   End;
   VT1.Next;
End;
Vt1.Locate('S_SSN', I, [loCaseInsensitive]);   // restore cursor
if Length(_SW)=0 then
Begin
  MessageDlg(' لطفا حد اقل یک مورد را انتخاب کنید. ', Mterror, [mbok], 0);
  Exit;
End;
TMoeinF.init( _SW, _SC, _SN, COID.KeyValue );
```

- `_SW` is a disjunction of exact four-segment matches, e.g.
  `(M_Ko=51 and M_Mo=3 and M_Ta1=0 and M_Ta2=1481 ) OR (M_Ko=52 and …)`. `TMoein` wraps it in
  parentheses on both legs (`TMoein.pas:237,249`), so the `OR` cannot leak.
- `_SC` and `_SN` are newline-joined code and name lists, printed verbatim in the `TMoein` report
  header memos `T3`/`T4`.
- Validation: at least one selected, message `لطفا حد اقل یک مورد را انتخاب کنید.` ("please select at
  least one item", `:425`).
- **The trailing blank `Vt1.Append` row (§4.4) is iterated too.** If it is selected, its four segments
  are empty strings and `_SW` gains `(M_Ko= and M_Mo= …)` — a SQL syntax error raised inside `TMoein`.
- **This is where §3.3's double-count lands:** every consolidated view launched from Card Jari gets an
  opening balance that also sums `M_Kind = 2` rows.

### 4.10 Writes performed

**None.** Every statement in the unit is a `SELECT` (`:130-131`, `:147-150`, `:286`, `:308-309`) or a
`#R` temp-table manipulation inside `QList` / `Jari_Rem`. `VT1` is memory-only. There is no `Insert`,
`Update`, `Delete` or `Post` against a persistent dataset anywhere in `CardJariU.pas`.

The only side effects are INI writes on close (`FormClose:203-212` — column widths, position, size)
and on font change (`:230`, `:381`).

### 4.11 Defects to decide on before porting

1. **Two balance figures that never agree.** Grid footer `Σ R_Bed`/`Σ R_Bes` covers all accounts;
   `S_Rem` covers only `SC_Rem = 1` accounts. Nothing on screen explains the difference.
2. **`Jari_Rem` has no `GROUP BY`** while `QList` does — duplicate `SahamdarConfig` templates
   double-count into the final balance only.
3. **Neither query filters `M_kind` or `M_Tx`**: journal-summary rows and unposted drafts are both
   included in every figure on this screen.
4. **`Report2` is not permission-gated** while `Report1` is (key 1123), and both reach the same data.
5. **The lock check runs after the identity data is already displayed** (`:291-337` before `:342`).
6. **`B_Close` is hinted `چاپ` ("print")** and closes the form.
7. **Trailing blank row** from `Vt1.Append` (`:163`), selectable, and fatal to `Report2Click` if
   selected.
8. **`S_R` reads the stale `Sarfasl.M_R`** whose maintenance is commented out (`02-data-model.md`
   §4.1.3) — the account-code column is unreliable.
9. **Changing the fiscal year clears the screen without reloading** (`.dfm:5931` →
   `S_CardChange` → `ClearForm`); the user must re-trigger `S_Card.OnExit`.
10. **Design-time connection strings with `User ID=sa`** are still in the `.dfm`
    (`.dfm:6294-6296`, `.dfm:6302-6304`, `.dfm:6487-6492`, `.dfm:6532-6537`), naming catalogues
    `RPPC` and `Arzi89` on hosts `PESTEH` and `MOHSEN-RANJBAR\SQLEXPRESS`. Overwritten at runtime
    (`:129`, `:261`, `:285`, `:363`) but present in source control.
11. **`ADOConnection1` (`.dfm:6301-6308`) is declared and never used** — dead component.
12. **N+1 query pattern:** two round trips per account plus three fixed, all serial, all rebuilding
    `Q1.SQL` from strings.


---

[← SS4 Card Jari (1/2)](04-04-a-card-jari.md) | [Index](00-index.md) | [SS5 Date-range and fiscal-year filtering semantics →](04-05-date-range-and-fiscal-year-filtering-semantics.md)
