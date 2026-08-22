_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 6.6 Right-to-left Persian text

**There is no report-level RTL switch.** FastReport's `RTLReport` property does not appear in any
`.dfm`. RTL is achieved three ways, all manual:

1. **Physical column placement.** Memos are laid out with the first-read (rightmost) column at the
   largest `Left` coordinate. §2.2 documents the resulting ordering for the 6-column trial balance:
   `Left` 948.66 → 0.00 reads right to left. Any port that lays columns out left-to-right will
   silently mirror every report.
2. **`HAlign = haRight`** on text memos, `haCenter` on headers, and `haRight`/left on numbers
   depending on the report. There is no consistent rule; e.g. `DKolU`'s grid aligns amounts
   `taLeftJustify` (`.dfm:233,241`) while its report memos vary.
3. **`BiDiMode = bdRightToLeft` on the VCL form and its controls** — 361 occurrences across the
   `.dfm` files, against 439 `BiDiMode = False` and 73 `bdLeftToRight`. This affects the **on-screen**
   grid and edit boxes, not the printed report. `Taraz6SetooniU.dfm:6` sets it at form level;
   `DKolU`/`DMoein` set it only on the `TFullDate` controls (`DKolU.dfm:144,159`).

The mixture is why some screens read right-to-left and others do not. The rebuild should set direction
once at the document/page level (`dir="rtl"` plus `unicode-bidi: isolate` on mixed-content cells) and
delete all per-control settings.

**Digit shaping in mixed text** is not handled at all. A Persian description containing a Latin number
relies on the Unicode bidi algorithm as implemented by the GDI text renderer. Any string that
concatenates Persian and Latin — e.g. `' حساب کل : ' + EKo.Text + ' ' + SKo.Text`
(`DKolU.pas:197`) — will render with the number position determined by the renderer, and FastReport's
renderer and a browser's will not necessarily agree. Snapshot-test these strings.

### 6.7 Persian digits

**Persian-Indic digits are produced by font substitution, not by transforming the text.** The only
implementation is in the 4-column trial balance (`Taraz4Setooni_U.pas:165` and `:245`):

```pascal
if F_Type.ItemIndex=0 then _FN := 'B Yekan' else _FN := 'WeblogmaYekan' ;
```

driven by the combo `F_Type` with items `اعداد فارسی` ("Persian numerals") / `اعداد انگلیسی`
("English numerals"), persisted to INI key `F_Type` (`:225`). The chosen name is then assigned to
`Font.Name` of the memos `D1`..`D11` in a loop (`:166-169`), together with
`Font.Size := F_Size.ItemIndex + 6` from the `سایز 6`..`سایز 13` combo.

Consequences:

- **The numerals are ASCII `0`-`9` in the data; only the glyphs differ.** Copying from the preview,
  exporting to Excel or CSV, or searching the output all yield Latin digits regardless of the setting.
- It depends on two specific fonts being installed on every workstation (`B Yekan`, `WeblogmaYekan`).
  Neither is a standard Windows font.
- **No other report offers the choice.** Every other report prints whatever its `.dfm` font gives.
- The font inventory across all `.dfm` files is: `Tahoma` 1003, `B Nazanin` 498, `Arial` 293,
  `tahoma` 224, `b Nazanin` 201, `MS Sans Serif` 65, `B Titr` 51, `Vazir` 14, `B Yekan` 3,
  `Arial Narrow` 3, plus one `tahome` (a typo that silently falls back) and one `arial`. The
  case-inconsistent duplicates (`Tahoma`/`tahoma`, `B Nazanin`/`b Nazanin`) are harmless on Windows
  and will not be on Linux.

For the rebuild: hold numbers as numbers, format with `Intl.NumberFormat('fa-IR')` or
`'fa-IR-u-nu-latn'` according to an explicit `numeralSystem` preference (§11.3), and never rely on a
font to change a character's identity.

### 6.8 Number formatting on printed output

Two independent formatting systems, one for the grid and one for the report, and they disagree:

| Layer | Property | Value | Zero renders as | Negative renders as |
|---|---|---|---|---|
| VCL grid field | `DisplayFormat` | `'###,###'` (`DKolU.dfm:553`, `DMoein`, `CardJariU`) | **empty string** | `-1,234` |
| VCL grid field | `DisplayFormat` | `'#,###'` (`BedBes.dfm:515`) | **empty string** | `-1,234` |
| VCL grid field | `DisplayFormat` | `'#,###'` (`Taraz4Setooni_U.dfm:383`) | **empty string** | n/a (clamped) |
| FastReport memo | `DisplayFormat.FormatStr` + `Kind = fkNumeric` | `'%2.0n'` | `0` unless `HideZeros` | `-1,234` |
| FastReport memo | `HideZeros` | `True` on all amount memos | **blank** | — |
| FastReport memo | `DisplayFormat.DecimalSeparator` | `'/'` (`Taraz4Setooni_U.dfm:1161`) | — | — |

Points that matter:

- `'%2.0n'` is "grouped, zero decimals" — all amounts are integral rials. No report prints fractions.
- **`DecimalSeparator = '/'`** is set on the trial-balance memos. Since there are no decimals it never
  shows, but it records the Persian convention (`٫`/`/` as decimal mark) and would surface the moment
  a fractional field is added.
- **Negatives essentially never print.** Every accounting report splits a signed balance into two
  clamped unsigned columns (§2.1, §3, §4), and the one place a signed value reaches a report — the
  ledger `Rem` — is wrapped in `ABS()` with a `بد`/`بس` letter beside it (`DKolU.dfm:894`,
  §3.1.b). **There is no parenthesis convention and no minus-sign convention anywhere in the printed
  output.** Only the on-screen grids show a leading `-`.
- **Zero prints as blank everywhere**, in both layers, by two different mechanisms. Preserve this: a
  page of zeros would otherwise be unreadable.

### 6.9 Fonts and sizing at runtime

Only `Taraz4Setooni_U` lets the user change print font size, via `F_Size` (`سایز 6` … `سایز 13`,
mapped `Font.Size := ItemIndex + 6`, `:166-169`, persisted to INI `F_size`). `CardJariU` has a
`POP_Size` menu with `سایز 6`..`سایز 15` but it is unreachable (§4.8) and in any case only affects the
on-screen grid.

`RoyatJU.Report2Click` is the one report that **derives its printed column widths from the user's
on-screen grid**: it reads `G1`/`G2` column widths, normalises them against the report title memo's
width, and assigns the result to `C1..C6`, `D1..D6` and `S1..S4` (`RoyatJU.pas:392-406`, `:443-460`).
Documented in §1.4; flagged here because it is the only dynamic layout in the print pipeline.

### 6.10 What the rebuild must reproduce

- A4 portrait, 10 mm left margin, repeating column header, grand-total footer, no group bands, no
  forced page breaks.
- Right-to-left column order, per-report, matching the legacy `Left` coordinates.
- Persian column captions verbatim (§11.5), including the two-tier trial-balance headers.
- Zero → blank; no negative sign; balance split into debit/credit columns with a `بد`/`بس` indicator.
- Row banding on odd rows; level-based tinting on the trial balances.
- The four configurable signature blocks (`Tanzim` 1011–1014) and the letterhead image.
- The amount-in-words footer for vouchers, with the legacy spelling of the scale words.
- `[Page#] از [TotalPages#]` and `[line#]`.

And must **not** reproduce: the `pbEdit`/`pbLoad` designer access from the preview; font-substitution
digits; hard-coded template selection in source; `FindObject`-by-string-literal binding.


---

[← SS6 Print pipeline (1/2)](04-06-a-print-pipeline.md) | [Index](00-index.md) | [SS7 Export pipeline →](04-07-export-pipeline.md)
