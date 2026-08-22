_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

### 4.4 Person/entity lock

```pascal
// SahamdarU.pas:78-93
procedure TSahamdar.B_LockClick(Sender: TObject);
var L:integer;
begin
   if Q1.Active=false then Exit;
   if Q1.RecordCount=0 then exit;
   if G1.Visible then ActiveControl := G1;
   if G2.Visible then ActiveControl := G2;
   L := Q1.FieldByName('S_lock').Value;
   if L=1 then L:=0 else L:=1;

   Q1.Edit;
   Q1.FieldByName('S_lock').Value := L;
   Q1.Post;
   if L=1 then  MessageDlg('   جاری قفل شد   ', mtInformation, [mbok], 0);
   if L=0 then  MessageDlg('   قفل جاری باز شد   ', mtInformation, [mbok], 0);
end;
```

| Persian | English |
|---|---|
| `جاری قفل شد` | "The current account has been locked" |
| `قفل جاری باز شد` | "The current-account lock has been released" |

Enforcement:

```pascal
// Dmu.pas:968-981
function TDM.Is_Admin_Or_Valid_Jari(_Jari: integer): Boolean;
begin
   Result := Admin;
   if Dm.Admin then Exit;
   ...
   Qs.SQL.Add('Select * From Sahamdar Where S_Card='+ inttostr(_Jari) );
   QS.Open;
   Result := True;
   if Qs.RecordCount=0 then exit;
   Result := Qs.FieldByName('S_Lock').AsInteger = 0 ;
   QS.Close;
end;
```

Admins bypass; an **unknown card is treated as unlocked** (`Result := True` before the record-count
check). The lock button itself is admin-only (`SahamdarU.pas:112-113`, `:127-128`).

Consumer (party-linkage aspect of `CardJariU`):

```pascal
// CardJariU.pas:342-347
   if not Dm.Is_Admin_Or_Valid_Jari(S_Card.IntValue) then
   begin
     ActiveControl := S_card;
     MessageDlg('   مشاهده اطلاعات فقط تو سط مدیر سیستم مجاز است     ', mterror, [mbok], 0);
     exit;
   end;
```
`مشاهده اطلاعات فقط تو سط مدیر سیستم مجاز است` = "Viewing this data is permitted only to the system
administrator". Note the early `exit` occurs **after** the person data and photo have already been
rendered (`CardJariU.pas:283-337`) — the lock hides only the balance and the account list.

### 4.5 Moving a party between the person and company registers

Hidden context menu, **hard-coded to user 68**:

```pascal
// SahamdarU.pas:95-105
procedure TSahamdar.init;
begin
    _HaHo := 1;
    ReOpenDB;
    G1.PopupMenu := nil;
    G2.PopupMenu := nil;
    if Dm.userId=68 then
    begin
      G1.PopupMenu := POP1;
      G2.PopupMenu := Pop1;
    end;
```

```pascal
// SahamdarU.pas:136-155  (person → company)
procedure TSahamdar.P_12Click(Sender: TObject);
var i:integer;
begin
   ActiveControl:=G1;
   if _HaHo=2 then exit;
   if Q1.Active=false then exit;
   if Q1.RecordCount=0 then exit;
   i:= MessageDlg('  آیا برای انتقال مطمئن هستید؟  ', mtWarning, [mbYes,mbCancel], 0);
   if I<>6 then exit;
   I:= Q1.FieldByName('S_Card').AsInteger;
   QS.Close;
   QS.SQL.Clear;
   Qs.ConnectionString := Dm.Ado.ConnectionString;
   Qs.SQL.Add('UpDate Sahamdar Set S_Kind=2 Where S_Card='+ inttostr(I) );
   QS.ExecSQL;
   _HaHo := 2;
   ReOpenDB;
   Q1.Locate('S_Card', I, [loCaseInsensitive] );
   ActiveControl:=G2;
end;
```
`SahamdarU.pas:157-176` (`P_21Click`) is the mirror image, setting `S_Kind=1`.

| Persian | English | Line |
|---|---|---|
| `آیا برای انتقال مطمئن هستید؟` | "Are you sure you want to transfer?" | `SahamdarU.pas:143`, `:164` |
| `انتقال به جاری شرکتها` | "Move to company current accounts" | `SahamdarU.dfm:475` |
| `انتقال به جاری اشخاص` | "Move to person current accounts" | `SahamdarU.dfm:482` |

> `if I<>6` compares against the raw Win32 `IDYES = 6` rather than `mrYes`. Under Delphi's
> `System.UITypes`, `mrYes = 6` as well, so it works — but it is an unsafe idiom.

**The `S_Kind` flip does not touch any `Sarfasl` node.** A party moved from person to company keeps
its detail accounts under the *person* control accounts (103-001, 104-001, …) while the editor now
offers the *company* set (103-002, 104-002, …). §12-Q2.

### 4.6 `SahamdarP.pas` — dead legacy form

`TSahamdarP_F`, caption `فرم مشخصات سهامداران` ("Shareholder particulars form",
`SahamdarP.dfm:5`). Registered in `arzi.dpr:16` and instantiated at `arzi.dpr:194`, but **no unit
calls `init` or `Edit`**. Its `init` body is entirely commented out (`SahamdarP.pas:71-120`).

It is nevertheless the **only** documentation of the two stored procedures `Sahamdar_Seek` and
`Sahamdar_Edit` (§9.2, §9.3), and of a `S_kind` combo whose items are:

| Index | Persian (`SahamdarP.dfm:396-397`) | English |
|---|---|---|
| 0 | `سهامدار حقيقي` | Natural-person shareholder |
| 1 | `سهامدار حقوقي` | Legal-entity shareholder |

> Note the **off-by-one against the live code**: `SahamdarP.pas:134` passes `S_Kind.ItemIndex`
> (0/1) straight to `@Kind`, whereas the live editors write literal `1`/`2`
> (`SahamdarEditU.pas:291`, `CompanyEditU.pas:263`) and the list query filters on 1/2
> (`SahamdarU.dfm:465`). The stored procedure is therefore incompatible with the live encoding.
> Do not port `Sahamdar_Edit`. §12-Q12.

Its one live-looking behaviour, for reference:

```pascal
// SahamdarP.pas:165-172
    DM.Sahamdar_Seek.Active := False;
    Dm.Sahamdar_Seek.Parameters.ParamByName('@S_Card').Value := Sah_Card;
    Dm.Sahamdar_Seek.Active := True;
    if Dm.Sahamdar_Seek.RecordCount =0 then Begin
       Dm.Sahamdar_Seek.Active := False;
        SayMSG('Error' , ' ',' سهامدار '+inttostr(Sah_Card)+' پيدا نشد ') ;
       Exit;
    End;
```
`سهامدار <n> پيدا نشد` = "Shareholder <n> not found".

---


---

[← Previous](07-04-a-person-legal-entity-sahamdar-model.md) · [Index](00-index.md) · [Next →](07-05-shareholder-equity-profit-distribution.md)
