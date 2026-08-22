_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 5. Voucher line editing behaviour

### 5.1 The in-memory line buffer

`SanadEditU` does **not** edit `Moein` rows directly. It loads the voucher into a client-side
`TVirtualTable` named `VSanad` (`SanadEditU.pas:51`), edits that, and writes the whole set back on
save. Fields of `VSanad` (`SanadEditU.pas:64-76`, `SanadEditU.pas:103`):

`M_SSN` (row ordinal, **not** the database id), `M_Bed`, `M_Bes`, `M_Ted`, `M_Article`, `M_Ko`,
`M_Mo`, `M_Ta1`, `M_Ta2`, `M_Id`, `M_Link`, `M_Code` (account id), `M_Name` (display), `M_CodeStr`
(display code).

Loading (`SanadEditU.pas:382-422`) assigns `M_SSN := I` (1..n) as a **sequence number**, not the
database key. The database key is not carried into the buffer at all — which is why saving deletes
and re-inserts rather than updating.

### 5.2 Grid layout and behaviour — `G1: TrDBGrid_MS`

Columns, in display order (`SanadEditU.dfm:631-685`):

| # | Field | Persian title | English | Alignment | Width |
|---|---|---|---|---|---|
| 1 | `M_CodeStr` | کد حساب | Account code | right | 133 |
| 2 | `M_Name` | نام حساب | Account name | default | 231 |
| 3 | `M_Bed` | *(field default)* | Debit | left | default |
| 4 | `M_Bes` | *(field default)* | Credit | left | default |
| 5 | `M_Ted` | *(field default)* | Quantity | centre | 50 |
| 6 | `M_Article` | شرح | Description | default | 200 |
| 7 | `M_Id` | سیستم | Source system | default | default |

Grid features (`SanadEditU.dfm:605-630`):
- `dgRowSelect` — whole-row selection; the grid is **not** inline-editable. All editing goes through
  the modal line dialog.
- `OptionsEx2.Filters.TextBar = True` — a per-column text filter bar. `G1.ResetFilter` is called
  before any total-dependent operation (`SanadEditU.pas:600`, `:766`, `:970`, `:189`).
- **Footer with running totals** (`SanadEditU.dfm:626-630`):
  ```
  FooterRow.FooterVisible = True
  FooterRow.FieldFooterDefs.Strings = ( M_Bed=%Sum   M_Bes=%Sum )
  FooterRow.RecalculateAfterFilter = True
  ```
  These two footer sums are what the balancing check compares (§4.1 #7).
- Font size is user-adjustable via a `TsUpDown` spinner (`SanadEditU.pas:273-278`), persisted to the
  INI file under `G1FontSize`. Column widths are persisted per column as `G1C<i>`
  (`SanadEditU.pas:169-170` restore, `:184-188` save).

### 5.3 Keyboard shortcuts in the grid

| Key | Action | Cite |
|---|---|---|
| **Double-click** | Edit the current line (equivalent to the `اصلاح` button) | `SanadEditU.pas:212-217` |
| **Alt + ↑** | Move the current line **up** one position | `SanadEditU.pas:222-245` |
| **Alt + ↓** | Move the current line **down** one position | `SanadEditU.pas:246-269` |

The reorder implementation is a three-step swap of the `M_SSN` ordinal via a sentinel value of `-1`:

```pascal
// SanadEditU.pas:226-244 (Alt+Up)
       LN1 := VSanad.FieldByName('M_SSN').Value ;
       VSanad.Prior;
       LN2 := VSanad.FieldByName('M_SSN').Value ;
       if LN1=Ln2 then exit;              // already at the top
// L2 = -1
       VSanad.Edit; VSanad.FieldByName('M_SSN').Value := -1; VSanad.Post;
// Locate L1
       VSanad.Locate('M_SSN' , LN1, [loCaseInsensitive] );
       VSanad.Edit; VSanad.FieldByName('M_SSN').Value := LN2; VSanad.Post;
// Locate L2
       VSanad.Locate('M_SSN' , -1, [loCaseInsensitive] );
       VSanad.Edit; VSanad.FieldByName('M_SSN').Value := LN1; VSanad.Post;
       VSanad.Locate('M_SSN' , LN2, [loCaseInsensitive] );
```

**Line order is not persisted.** `M_SSN` is a client-side ordinal; the insert loop
(`SanadEditU.pas:631-651`) walks `VSanad` in its current order and lets SQL Server assign fresh
identity values, and the reload query orders by `M_SSN` (the database identity —
`SanadEditU.pas:376`). The ordering therefore survives a save-then-reload only because identity
values are assigned in insertion order. **The rebuild must add an explicit `line_number` column.**

On the chart-of-accounts screen (`SNewu`) the keys are different:

| Key | Action | Cite |
|---|---|---|
| **Enter** / **double-click** | Drill **down** into the sub-level | `SNewu.pas:458-464`, `:404-411` |
| **Esc** | Go **up** one level | `SNewu.pas:465-468` |
| **Numpad +** (VK 107) | Add a new code at the current level | `SNewu.pas:446-456` |

### 5.4 Adding a line

`B_AddClick` (`SanadEditU.pas:981-991`):

```pascal
    EditArticleMoein.new;
    if EditArticleMoein.SSN=0 then exit;
    if EditArticleMoein._Ok=0 then exit;
    VSanad.Append;
    Vsanad.FieldByName('M_SSN').AsInteger := VSanad.RecordCount + 1 ;
    SaveToVsanad;
    G1.RecalculateSummaryResults(True);
```

`EditArticleMoein.new` (`EditArticleMoeinU.pas:151-161`) clears the form, un-read-onlys every field,
focuses the Kol code box, sets `_Ok := 0`, and shows modally. Two success flags must both be set:
`SSN > 0` (a valid leaf account) and `_Ok = 1` (validation passed).

`SaveToVsanad` (`SanadEditU.pas:705-728`) copies the dialog into the buffer row, including the derived
display strings:

```pascal
     Vsanad.FieldByName('M_Bed').AsString := EditArticleMoein.Bed.Inttext;
     Vsanad.FieldByName('M_Bes').AsString := EditArticleMoein.bes.Inttext;
     Vsanad.FieldByName('M_Ted').AsString := EditArticleMoein.Ted.TextValue;
     Vsanad.FieldByName('M_Ko').AsInteger := EditArticleMoein.EKo.Tag;
     ...
     Vsanad.FieldByName('M_Article').AsString := EditArticleMoein.Des.Text;
     Vsanad.FieldByName('M_Link').AsInteger := 0;
     Vsanad.FieldByName('M_ID').AsInteger := 0;
     S:= EditArticleMoein.SKo.Text+'/'+EditArticleMoein.SMO.Text;
     if EditArticleMoein.ETa1.Tag>0 then S:= S + '/'+EditArticleMoein.STa1.Text;
     if EditArticleMoein.ETa2.Tag>0 then S:= S + '/'+EditArticleMoein.STa2.Text;
     Vsanad.FieldByName('M_Name').AsString := S;
     Vsanad.FieldByName('M_Code').AsInteger := EditArticleMoein.SSN;
     VSanad.FieldByName('M_CodeStr').AsString := Make_CStr(...);
```

Note `M_Link := 0` and `M_ID := 0` are hard-coded — a manually added line is always a manual line.

### 5.5 Editing a line

`B_EditClick` (`SanadEditU.pas:1013-1041`): blocks generated lines (§3.4), then pre-populates the
dialog from the buffer row field by field and calls `ShowModal` directly (not `new`, so `ClearForm`
is invoked explicitly first at `SanadEditU.pas:1023`).

### 5.6 The account picker flow — `EditArticleMoeinU`

Four cascading code boxes (`EKo`, `EMo`, `ETa1`, `ETa2`), each paired with a read-only name display
(`SKo`, `SMO`, `STa1`, `STa2`) and a `...` browse button (`BKo`, `BMO`, `BTa1`, `BTa2`).

**Delphi `Tag` convention (used throughout the codebase):** the `.Tag` property of a code edit box
holds the **resolved** code, or `0` if the typed text does not resolve to an existing account. Code
that reads `.Text` is reading raw user input; code that reads `.Tag` is reading validated state.

**Per-keystroke resolution.** `EKoChange` (`EditArticleMoeinU.pas:169-186`):

```pascal
   EKo.Tag := 0;
   SKo.Text := '';
   EMo.Text := '';
   EMo.ReadOnly := True;
   Qs.SQL.Add(' Select * From sarfasl ');
   Qs.SQL.Add('  Where S_Ko>0 and S_Mo=0 and S_Ko=0'+ EKo.Text );   // note the '0' prefix
   Qs.Open;
   if Qs.RecordCount=0 then exit;
   SKo.Text := Qs.FieldByName('S_Name').AsString;
   EKo.Tag := Qs.FieldByName('S_Ko').AsInteger;
   EMo.ReadOnly := Qs.FieldByName('S_Child').AsInteger = 0;
```

Three things to note:
1. **The `'0'` string prefix** (`S_Ko=0` + text) is a trick so that an empty text box produces
   `S_Ko=0` rather than a syntax error. It also means a typed `"07"` resolves to `07` = 7. Harmless
   here but a SQL-injection vector — the rebuild must use bound parameters.
2. Changing a level **clears every deeper level**.
3. `EMo.ReadOnly := (S_Child = 0)` makes the Moein box editable when the Kol node *has* children and
   read-only when it is a leaf. Correct behaviour, confusingly written. Same pattern at
   `EditArticleMoeinU.pas:221` and `:313`.

`EMoChange` (`:297-314`) requires `S_Ko>0 and S_Mo>0 and S_Ta1=0`;
`ETa1Change` (`:205-222`) requires `S_Ko>0 and S_Mo>0 and S_Ta1>0 and S_Ta2=0`;
`ETa2Change` (`:258-275`) requires all four `>0`.

**Focus guards.** Entering a deeper box while the shallower one is unresolved bounces focus back:

```pascal
// EditArticleMoeinU.pas:194-204
procedure TEditArticleMoein.EMoEnter(Sender: TObject);
begin
    if EKo.Tag=0 then begin ActiveControl := EKo; exit; end;
    EMo.Left := BMo.Left + 24;   // shrink the edit, reveal the browse button
    EMo.Width := 61;
    BMo.Visible := true;
end;
```

Same at `:224-235` (`ETa1Enter`) and `:277-288` (`ETa2Enter`). The visual effect: the `...` browse
button appears only for the focused level, and the edit box narrows to make room. On exit the box is
restored (`:239-256`, `:290-295`).

**Browse buttons** open `SelectSarfasl` scoped to the current parent:

```pascal
// EditArticleMoeinU.pas:323-356
procedure TEditArticleMoein.BKoClick;  select_Sarfasl.init_Ko(EKo.Text);
procedure TEditArticleMoein.BMOClick;  select_Sarfasl.init_Mo(EKo.Text, EMo.Text);
procedure TEditArticleMoein.BTa1Click; select_Sarfasl.init_ta1(EKo.Text, EMo.Text, ETa1.text);
procedure TEditArticleMoein.BTa2Click; select_Sarfasl.init_ta2(EKo.Text, EMo.Text, ETa1.text, ETa2.Text);
```

After a pick, focus advances to the next level automatically (`:329`, `:338`, `:347`).

`SelectSarfasl` (`SelectSarfasl.pas`) — a single-level browse list. Each `init_*` builds a query
aliasing the relevant level to a uniform column `Code`:

```sql
-- SelectSarfasl.pas:93-95
 Select Code=S_Ko , Sarfasl.* from Sarfasl Where S_Mo=0 Order By S_Ko
-- SelectSarfasl.pas:110-112
 Select Code=S_Mo , Sarfasl.* from Sarfasl Where S_ko=<Ko> and S_Mo>0 and S_Ta1=0 Order By S_Mo
-- SelectSarfasl.pas:127-129
 Select Code=S_Ta1 , Sarfasl.* from Sarfasl Where S_ko=<Ko> and S_Mo=<Mo> and S_ta1>0 and S_Ta2=0 Order By S_Ta1
-- SelectSarfasl.pas:144-146
 Select Code=S_Ta2 , Sarfasl.* from Sarfasl Where S_ko=<Ko> and S_Mo=<Mo> and S_ta1=<Ta1>and S_Ta2>0 Order By S_Ta2
```

(Note the missing space in `<Ta1>and` at `SelectSarfasl.pas:145`; harmless because `Ta1` is numeric.)

The list positions itself on the currently-typed code (`Q1.Locate('Code', …)`) or falls back to the
first row. Double-click = OK (`SelectSarfasl.pas:80-83`). Returns `_Code`, `_Name`, `_FullName`
(`SelectSarfasl.pas:47-57`).

**Note the pickers do NOT filter by `S_Child`** — unlike `Sarfasl_SelectU` (§12.6). A user can pick a
non-leaf from the browse dialog; the final `Get_SSn` leaf check (§4.2 #1) is what rejects it.

---

_Prev: [03-04-voucher-validation-rules](03-04-voucher-validation-rules.md) | Next: [03-05-b-voucher-line-editing-behaviour](03-05-b-voucher-line-editing-behaviour.md)_
