_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 3.5 Header maintenance: `DMoein_Make` and `Dmoein_UpdateMab`

**`Dm.DMoein_Make(_Sanad, _Date, _Desc, _Kind=1)`** — `Dmu.pas:815-839`. Upsert of the header from the
lines. Called after every save/generate.

```sql
-- Dmu.pas:820-836, verbatim
 Declare @Sanad int Set @Sanad=<n>
 Declare @Coid  int Set @Coid=<coid>
 Declare @User  int Set @User=<userId>
 Declare @Date varchar(10) Set @Date='<jalali>'
 Declare @Desc varchar(200) Set @Desc='<narration>'
 Declare @TC int, @TBed Bigint, @TBes Bigint
 Select @TBed=isnull(Sum(M_Bed),0), @TBes=isnull(Sum(M_bes),0), @TC=isnull(Count(*),0)
    From moein where M_Sanad=@sanad and M_Coid=@Coid
 if Exists( Select * From DMoein Where DM_Sanad=@Sanad and DM_Coid=@Coid)
    Update DMoein Set DM_CUser=@User, DM_CDate=GetDate(), DM_Tbed=@Tbed, DM_Tbes=@TBes,
                      DM_Count=@TC, DM_Desc=@Desc
    Where DM_Sanad=@Sanad and DM_Coid=@Coid
 else
   Insert DMoein (Dm_Sanad, DM_Date, DM_Desc, DM_Coid, DM_Tx, DM_TBed, DM_Tbes, DM_Count,
                  DM_Muser, DM_MDate, DM_CUser, DM_Kind)
   Values( @Sanad, @Date, @Desc, @Coid, 0, @TBed, @TBes, @Tc, @User, GetDate(), 0, <kind> )
```

Note: on **update** it does **not** refresh `DM_Date`. Changing a voucher's date therefore requires
the dedicated "change date" action (§12.3), which updates `DMoein`, `Moein` and every linked
subsystem table.

Note also: a new header is always created with `DM_Tx = 0` and `DM_CUser = 0`.

**`Dm.Dmoein_UpdateMab(_Sanad)`** — `Dmu.pas:841-857`. Recomputes the totals **and deletes the header
if the voucher has no lines left**:

```sql
-- Dmu.pas:846-855, verbatim
 Declare @Sanad int Set @Sanad=<n>
 Declare @Coid  int Set @Coid=<coid>
 Declare @Count int, @TBed Bigint, @TBes Bigint
 Select @TBed=isnull(Sum(M_Bed),0), @TBes=isnull(Sum(M_bes),0), @Count=isnull(Count(*),0)
    From moein where M_Sanad=@sanad and M_Coid=@Coid
 Update DMoein Set DM_TBed=@TBed, DM_TBes=@TBes, DM_Count=@Count
    where DM_Sanad=@sanad and DM_Coid=@Coid
    Delete DMoein Where DM_Sanad=@sanad and DM_Coid=@Coid and DM_Count=0
```

Called after un-posting an inventory document (`SodoorSanadU.pas:265`) and after the closing routine
(`NewFinalu.pas:228`).

### 3.6 Voucher lifecycle / state machine

`DM_TX` (header) and `M_Tx` (lines) hold the same three-valued state.

```
                 ┌──────── B_TX10 (perm 1118) ─────────┐
                 │                                     │
                 v                                     │
      ┌──────────────────┐   B_TX01 (perm 1116)  ┌─────┴──────────┐   B_TX12 (perm 1117)   ┌───────────────┐
      │  0  DRAFT        │ ────────────────────> │  1  APPROVED   │ ────────────────────>  │  2  PERMANENT │
      │  در حال تحریر     │  requires balanced    │  تایید شده     │                        │  ثبت دائم      │
      └──────────────────┘                       └────────────────┘ <───────────────────── └───────────────┘
                                                          ^          B_TX21 (perm 1145)
                                                          │
   Orthogonal, at any state:  DM_Lock 0 <-> 1   (popup menu, perm 1144)
```

| Value | Persian (canonical) | English | Constant name proposed |
|---|---|---|---|
| `0` | در حال تحریر | Draft / in preparation | `Draft` |
| `1` | تایید شده | Approved / closed | `Approved` |
| `2` | ثبت دائم | Permanently posted | `Posted` |

**The Persian labels are inconsistent across the codebase.** Recorded verbatim so the rebuild can
pick one and normalise:

| Source | 0 | 1 | 2 |
|---|---|---|---|
| `SanadEditU.pas:444-446` | `در حال تحریر` | `بسته` ("closed") | `ثبت دائم` |
| `MergeSanad.pas:195-197` | `در حال تحریر` | `تایید شده` | `ثبت دادئم` *(typo for `ثبت دائم`)* |
| `SanadViewU.pas:648-650` | `باز` ("open") | `بسته` ("closed") | `تایید` ("approved") — **mislabelled** |
| `RooznamehViewU.pas:~` (`Q1DM_TXGetText`) | `باز` | `بسته` | `تایید` |
| `SanadMoeinu.pas:132-134` | `در حال تحرير` | *(absent)* | `ارجاع شده` for 2, `ثبت دائم شده` for 3 — **stale, references a state 3 that no longer exists** |
| `SanadViewU.pas:126-128` (window captions) | `نمايش اسناد معين در حال تحرير` | `نمايش اسناد معين تاييد شده` | `نمايش اسناد معين ثبت شده` |

Use the `SanadViewU.pas:126-128` window captions as canonical: 0 = "in preparation", 1 = "approved",
2 = "posted".

#### Transition 0 → 1 (Approve)

`SanadViewU.B_TX01Click` (`SanadViewU.pas:278-312`). Button caption `'تاييد سند'` ("approve voucher").
Operates on a **range** of voucher numbers obtained from a two-number dialog:
`Get2No('information','تاييد اسناد','از سند','تا سند', No1, No2)` — "approve vouchers" / "from
voucher" / "to voucher" (`SanadViewU.pas:287`). Range is rejected if `No2 < No1` or `No2 = 0`.

```sql
-- SanadViewU.pas:292-304, verbatim
 Declare @S1 int Set @S1=<from>
 Declare @S2 int Set @S2=<to>
 Declare @Coid int Set @Coid=<coid>

 Update Moein Set M_Tx=1 Where M_Coid=@Coid  and M_Sanad in
   (Select DM_Sanad From Dmoein Where DM_Sanad<=@S2 and DM_Sanad>=@S1
       and DM_Tx=0 and DM_TBed=DM_TBes and DM_Coid=@Coid )

 Update DMoein Set DM_Tx=1 Where DM_Sanad<=@S2 and DM_Sanad>=@S1
   and DM_Tx=0 and DM_TBed=DM_TBes and DM_Coid=@Coid

  Select isnull( Max(DM_Sanad),0)  As S From DMoein
   Where DM_Sanad<=@S2 and DM_Coid=@Coid and DM_Tx=0
```

**This is the only place where balancing is enforced at the database level**: `DM_TBed = DM_TBes`.
Unbalanced vouchers are silently skipped (no error message; the third statement returns the highest
still-unapproved voucher so the grid can navigate to it).

Note: there is **no transaction wrapper** here, unlike the other three transitions.

#### Transition 1 → 0 (Return to draft)

`SanadViewU.B_TX10Click` (`SanadViewU.pas:429-466`). Caption `'برگشت به تحرير'` ("return to
preparation"). Dialog title `'برگشت به تحریر'`. Range defaults to `0..0`.

```sql
-- SanadViewU.pas:447-458, verbatim
Begin Transaction
  Declare @S1 int Set @S1=<from>
  Declare @S2 int Set @S2=<to>
  Declare @Coid int Set @Coid=<coid>
  Update Moein Set M_Tx=0 Where M_Coid=@Coid  and M_Sanad<=@S2 and M_Sanad>=@S1 and M_tx=1
  Update DMoein Set DM_Tx=0 Where DM_Sanad<=@S2 and DM_Sanad>=@S1 and DM_Tx=1 and DM_Coid=@Coid
  Select isnull( Max(DM_Sanad),0)  As S From DMoein
     Where DM_Sanad<=@S2 and DM_Coid=@Coid and DM_Tx=1
Commit
```

Gated by `DM.Is_New_Sanad_Valid(Dm.CO_ID)` (`SanadViewU.pas:438`) — the fiscal year must be open (§3.8).

#### Transition 1 → 2 (Post permanently)

`SanadViewU.B_TX12Click` (`SanadViewU.pas:468-502`). Caption `'ثبت دائم سند'` ("permanently post
voucher"). Dialog title `'ثبت سند'`.

```sql
-- SanadViewU.pas:483-494, verbatim
Begin Transaction
  Declare @S1 int Set @S1=<from>
  Declare @S2 int Set @S2=<to>
  Declare @Coid int Set @Coid=<coid>
  Update Moein Set M_Tx=2 Where M_Coid=@Coid  and M_Sanad<=@S2 and M_Sanad>=@S1 and M_tx=1
  Update DMoein Set DM_Tx=2 Where DM_Sanad<=@S2 and DM_Sanad>=@S1 and DM_Tx=1 and DM_Coid=@Coid
  Select isnull( Max(DM_Sanad),0)  As S From DMoein
     Where DM_Sanad<=@S2 and DM_Coid=@Coid and DM_Tx=1
Commit
```

Only `1 → 2` is possible; a draft cannot jump straight to permanent.
**Not** gated by `Is_New_Sanad_Valid` — the only transition that isn't. Probably an oversight.

#### Transition 2 → 1 (Return to approved)

`SanadViewU.B_TX21Click` (`SanadViewU.pas:504-541`). Caption `'برگشت به تایید'` ("return to
approved"). Same shape, `M_tx=2 → 1`. Gated by `Is_New_Sanad_Valid`. Permission `1145`
(`'برگشت از ثبت دائم'` = "reverse permanent posting").

#### What becomes immutable at each state

| Operation | Requires | Enforced at |
|---|---|---|
| Edit voucher header/lines | `DM_TX = 0` **and** `M_TX = 0` | `SanadEditU.pas:797-814` |
| Delete voucher | `Max(M_Tx) = 0` | `SanadViewU.pas:233`, `Dmu.pas:1301` |
| Delete a single line | `Max(M_Tx) = 0` | `Dmu.pas:1346` |
| Add lines from the legacy screen | `SanadState = 0` | `SanadMoeinu.pas:250`, `SanadMoeinu.pas:288`, `SanadMoeinu.pas:343` |
| Un-post an inventory document | `Get_SanadMaxTX(sanad) = 0` | `SodoorSanadU.pas:234` |
| Append to a voucher during closing | `M_TX = 0` | `NewFinalu.pas:103` |
| Year-end carry-forward | **all** vouchers of the year at `M_tx >= 2` | `EnteghalU.pas:104-110` |
| Generate a journal voucher | **all** source vouchers at `M_Tx >= 2` | `MoeinToRU.pas:181-186` |

`SanadEditU.S_EditClick` — the "put this voucher back into edit mode" button — checks both tables:

```pascal
// SanadEditU.pas:794-814
   Q1.SQL.Add('Select DM_TX as TX From DMoein Where DM_Sanad='+..+' and DM_Coid='+..);
   if Q1.FieldByName('TX').AsInteger > 0  then
      MessageDlg('   سند را در حالت تحریر قرار دهید   ', mterror, [mbok], 0);  // "put the voucher into draft state"
   ...
   Q1.SQL.Add('Select M_TX as TX From Moein Where M_Sanad='+..+' and M_Coid='+..);
   if Q1.FieldByName('TX').AsInteger > 0  then
      MessageDlg('   سند را در حالت تحریر قرار دهید   ', mterror, [mbok], 0);
```

then a permission check:

```pascal
// SanadEditU.pas:816-820
   if Not Dm.IsEnabel( Dm.userId , 1114 ) Then
      MessageDlg('   شما مجوز اصلاح سند را ندارید   ', mterror, [mbok], 0);  // "you do not have permission to edit the voucher"
```

and finally `MessageDlg('   سند در حالت تحریر قرار گرفت   ')` ("the voucher was put into draft state")
— `SanadEditU.pas:823`. (Displayed with `mterror` styling; cosmetic bug.)

#### The independent lock

`DM_Lock` is a separate boolean, toggled from a popup menu on both `SanadViewU`
(`SanadViewU.pas:605-635`, menu items `'قفل سند'` / `'برداشتن قفل'` = "lock voucher" / "remove lock")
and `RooznamehViewU`.

```pascal
// SanadViewU.pas:605-619 -- lock
   if Q1.FieldByName('DM_lock').AsInteger=1 then
     MessageDlg('   سند قبلا قفل شده است   ');   // "the voucher is already locked"
   Q1.Edit; Q1.FieldByName('DM_lock').Value := 1; Q1.Post;
   MessageDlg('   سند قفل شد   ');               // "the voucher was locked"
// SanadViewU.pas:621-635 -- unlock
   if Q1.FieldByName('DM_lock').AsInteger=0 then
     MessageDlg('   سند قفل نیست   ');           // "the voucher is not locked"
   ... := 0 ...
   MessageDlg('   قفل سند باز شد   ');           // "the voucher lock was released"
```

**Effect** — `Dm.Is_Admin_Or_Valid_Sanad` (`Dmu.pas:983-995`):

```pascal
function TDM.Is_Admin_Or_Valid_Sanad(_Sanad, _Coid: integer): Boolean;
begin
   Result := Admin;
   if Dm.Admin then Exit;                       // admins bypass
   Qs.SQL.Add('Select * From DMoein Where DM_sanad='+..+' and DM_coid='+..);
   QS.Open;
   if Qs.RecordCount=0 then exit;               // no header => Result stays False!
   Result := Qs.FieldByName('DM_Lock').AsInteger = 0 ;
end;
```

**Bug:** when the header row is missing (a `Moein`-only voucher — see §3.1), the function returns
`False` (the initial `Result := Admin` value), i.e. **denies access to non-admins**. The intent was
clearly to allow. Message shown by callers:
`'   اجازه دسترسی فقط برای مدیر فعال است  '` ("access is enabled for the administrator only") —
`SanadEditU.pas:864`, `SanadViewU.pas:159`, `SanadViewU.pas:215`, `SanadViewU.pas:270`.

`RooznamehViewU` additionally blocks deletion of a locked journal voucher:
`'   امکان حذف سند وجود ندارد سند  قفل شده است   '` ("the voucher cannot be deleted, the voucher is
locked").

---

_Prev: [03-03-a-voucher-sanad-model](03-03-a-voucher-sanad-model.md) | Next: [03-03-c-voucher-sanad-model](03-03-c-voucher-sanad-model.md)_
