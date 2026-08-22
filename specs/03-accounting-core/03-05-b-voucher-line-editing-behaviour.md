_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 5.7 Defaults carried between lines

**In `EditArticleMoeinU` (the live editor): none.** `ClearForm` (`EditArticleMoeinU.pas:390-414`)
blanks every field including the description.

**In the legacy editors, the description is carried forward** via a unit-level variable:

```pascal
// ArticleMoeinu.pas:352-359
procedure TArticleMoein.DesEnter(Sender: TObject);
begin
   if Length( Trim( Des.Text )) = 0 then
   begin
      Des.Text := LastDesc ;
      des.SelectAll;
   end;
end;
```

`LastDesc` is set on successful save (`ArticleMoeinu.pas:164`) and reset when the voucher screen
opens (`SanadMoeinu.pas:398`). Identical logic in `ArticleRooznamehU.pas:180-187`, `:124`.

**There is no auto-fill of the balancing amount anywhere in the codebase.** The user must type both
sides. This is a notable usability gap relative to typical Iranian accounting packages — see §15.

### 5.8 Bulk line operations on the voucher

Two bulk tools sit on the toolbar of `SanadEditU`, both disabled in view mode (`_NewEditView = 3`).

**(a) Swap the debit and credit columns** — `S_BedBesClick` (`SanadEditU.pas:756-789`),
hint `'تغییر ستون'` ("change column").

Confirmation: `'  مبالغ ستون بدهکار و بستانکار تعویض شود ؟  '`
("Shall the debit and credit column amounts be swapped?") — `SanadEditU.pas:764`.

```pascal
    G1.ResetFilter;
    I2:=VSanad.RecNo;                       // remember cursor
    I3:=0;                                  // counter
    Vsanad.First;
    for i := 1 to VSanad.RecordCount do
    begin
      if VSanad.FieldValues['M_link'] = 0  then     // <-- skips generated lines
      Begin
         _Bed := VSanad.FieldValues['M_Bes'];
         _Bes := VSanad.FieldValues['M_Bed'];
         VSanad.Edit;
         VSanad.FieldValues['M_Bes'] := _Bes;
         VSanad.FieldValues['M_Bed'] := _Bed;
         VSanad.Post;
         inc(I3);
      End;
      VSanad.Next;
    end;
```

Result message: `format(' تعداد %d رکورد از %d رکورد مبالغ ستون بدهکار و بستانکار تعویض شد  ', [I3,I2])`
("The debit and credit column amounts of %d records out of %d records were swapped") —
`SanadEditU.pas:787`.

Note the guard is `M_link = 0`, not `M_Id = 0` — subtly different from the per-line guards.
Note also the message's second `%d` is `I2`, which at that point has been reassigned to
`VSanad.RecordCount` (`SanadEditU.pas:786`) — so it does read "out of N records" correctly, by reuse
of the variable that a moment earlier held the cursor position.

**(b) Find and replace in line descriptions** — `S_ReplaceClick` (`SanadEditU.pas:906-948`),
hint `'تغییر شرح'` ("change description").

Two prompts (`SanadEditU.pas:916-918`):
- `GetString('تغییر در شرح سند', 'شرح', 50, S1)` — "change in the voucher description" / "description"
- `GetString('تغییر در شرح سند', 'به شرح', 50, S2)` — "… / to description"

Then confirmation `MessageDlg('تغییر در شرح سند', mtWarning, [mbyes,Mbno], 0)`.

```pascal
       St1 := VSanad.FieldByName('M_Article').AsString;
       St2:=St1;
       if Pos( S1, St1) > 0  then
         St2:= StringReplace( St1, S1, S2 ,[rfReplaceAll, rfIgnoreCase] );
       if Vsanad.FieldValues['M_link']=0 then
       if St2<>St1 then
       Begin
           Vsanad.Edit;
           VSanad.FieldByName('M_Article').AsString := St2;
           VSanad.Post;
           inc(TR);
       End;
```

Case-insensitive, replace-all, skipping generated lines. Result:
`'تغییر انجام شد'` + newline + `format('تعداد %d رکورد از %d رکورد تغییر پیدا کرد', [TR, Vsanad.RecordCount])`
("The change was performed" / "%d records out of %d records were changed") — `SanadEditU.pas:946`.

### 5.9 Import a voucher from a file

`SanadEditU.Import` (`SanadEditU.pas:280-353`). **Visible only to user id 68**:

```pascal
// SanadEditU.pas:175
     B_Import.Visible := Dm.userId = 68 ;
```

(A hard-coded user id — see §13.)

Format: a `.GGS` file, which is an INI file read through `TPropSaveFile`:

```
[GreenGold]
Count=<n>
Desc=<voucher narration>

[Line1]
Ko=<int>   Mo=<int>   Ta1=<int>   Ta2=<int>
Bed=<string>   Bes=<string>   Desc=<string>
[Line2]
...
```

Flow (`SanadEditU.pas:287-352`):
1. `OD.Execute` — standard open dialog. Abort if cancelled or the file is missing:
   `'فایل سند یافت نشد'` ("voucher file not found") — `SanadEditU.pas:304`.
2. `Count = 0` → `'  فایل انتقالی سند خالی است  '` ("the voucher transfer file is empty") —
   `SanadEditU.pas:313`.
3. Header: `S_Desc := Desc`, `S_Date := today`, `S_Sanad := Dm.New_Sanad`.
4. Progress dialog `WaitF.initForm('در حال بارگذاری ...', 0, lc)` ("loading …").
5. For each line: populate `EditArticleMoein`'s fields, call `B_OKClick` **programmatically**, and if
   `SSN > 0` append to `VSanad`. Otherwise show the section name as an error
   (`MessageDlg(Sec, mtError, …)` — `SanadEditU.pas:343`).

The **same file format is produced** by `BastanHesab` (§9.5) and consumed by `SanadMoeinu.InFileClick`
(§9.6). Treat `.GGS` as a documented interchange format in the rebuild, or replace it with CSV/JSON —
but the *semantics* (a list of `{account tuple, single-sided amount}`) must be preserved.

---

_Prev: [03-05-a-voucher-line-editing-behaviour](03-05-a-voucher-line-editing-behaviour.md) | Next: [03-06-a-automatic-voucher-generation](03-06-a-automatic-voucher-generation.md)_
