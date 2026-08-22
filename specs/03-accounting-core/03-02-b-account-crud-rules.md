_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

### 2.4 Delete

`B_DeleteCode` (`SNewu.pas:163-195`), hint `'حذف کد'`, caption `'حذف'`.

Validations, **in order**:

| # | Check | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | Grid empty | — (silent `Exit`) | — | `SNewu.pas:166` |
| 2 | Node has children (`SNO > 0`) | `' این کد زیر شاخه دارد و قابل حذف نیست '` | "This code has sub-branches and cannot be deleted" | `SNewu.pas:169` |
| 3 | Node has postings (`SanadCode > 0`) | `' بر روی این کد سند صادر شده است و قابل حذف نیست '` | "A voucher has been issued against this code and it cannot be deleted" | `SNewu.pas:175` |

There is **no confirmation dialog** on this screen. The delete is immediate.

```sql
-- SNewu.pas:182-188
 Delete From Sarfasl Where S_Ko=<S_Ko>
   and S_Mo=<S_Mo>     -- only if S_Mo>0
   and S_Ta1=<S_Ta1>   -- only if S_Ta1>0
   and S_Ta2=<S_Ta2>   -- only if S_Ta2>0
```

Then `MessageDlg('   انجام شد   ')` ("done"), `Dm.Update_Sarfasl_Child` (`SNewu.pas:192`), and the
grid reloads positioned on the previous row.

**Danger, must be fixed in the rebuild:** because deeper components are only added to the `WHERE`
when non-zero, deleting a Kol node whose `S_Child` happens to be `0` (stale denormalisation) would
issue `Delete From Sarfasl Where S_Ko=<K>` and **wipe the entire subtree**. Guard #2 is the only
thing standing between the user and mass deletion, and it reads a denormalised column that is only
recomputed on delete. Port this as an explicit, transactional, server-enforced check.

**The legacy delete path** (`ListSarfaslu.pas:160-186`, unreachable) is different and worth recording:

```pascal
  if Sp1.RecordCount=0 Then Exit;
  if Not GetYes('حدف سرفصل', 'سرفصل حذف شود ؟' ) Then Exit;   // "delete account" / "delete the account?"
  ...
  SP_Del.Parameters.ParamByName('@k').Value := K;
  SP_Del.Parameters.ParamByName('@M').Value := M;
  SP_Del.Parameters.ParamByName('@T1').Value := T1;
  SP_Del.Parameters.ParamByName('@T2').Value := T2;
  SP_Del.Parameters.ParamByName('@Co').Value := Dm.CO_ID;
  Sp_del.Active := True;
  ...
  SayMsg('پيغام', SP_Del.FieldByName('M').AsString );   // "message"
```

It confirms first, then delegates every check to the stored procedure `Sarfasl_Deep`
(`ListSarfaslu.dfm:251`), which returns a message column `M`. Note the typo in the Persian title:
`حدف` should be `حذف`. **`Sarfasl_Deep`'s body is not in this repository.**

### 2.5 Lock / unlock

`B_Lock` (`SNewu.pas:354-371`). **Visible only to administrators**: `B_Lock.Visible := Dm.Admin;`
(`SNewu.pas:95`).

Toggles `S_Lock` between 0 and 1:

```sql
-- SNewu.pas:365
Update Sarfasl Set S_Lock=<0 or 1> Where S_SSN=<id>
```

No validation, no confirmation. Rendered in the grid as a padlock icon
(`Image0` = unlocked, `Image1` = locked) by the owner-draw handler `SNewu.pas:413-444`.

**Effect of the lock** — `Dm.Is_Admin_Or_Valid_Daftar` (`Dmu.pas:921-966`):

```pascal
function TDM.Is_Admin_Or_Valid_Daftar(_Ko, _Mo, _Ta1, _Ta2: integer): Boolean;
begin
   Result := Admin;
   if Dm.Admin then Exit;                      // admins bypass all locks

   // walk the ancestor chain, top down
   Q1.SQL.Add(' Select * From sarfasl Where S_Ko='+_Ko+' and S_Mo=0 and S_ta1=0 and S_Ta2=0 ');
   if Q1.RecordCount=0 then Begin Result:=True; exit; end;      // missing ancestor => allow
   if Q1.FieldByName('S_Lock').AsInteger=1 then Begin Result :=false; Exit; end;
   if _Mo=0 then begin result := True; exit; end;
   // ... repeated for (Ko,Mo), (Ko,Mo,Ta1), (Ko,Mo,Ta1,Ta2) ...
   result := True;
end;
```

**The lock is inherited downward**: locking a Kol node blocks every descendant. A missing ancestor
row is treated as *unlocked* (fail-open).

`Is_Admin_Or_Valid_Daftar` is called only from the ledger/report screens; **it is not called from the
voucher-entry path**, so a locked account can still be posted to via `SanadEditU`. Recorded as an
inconsistency (§14).

### 2.6 Supplementary party data

`Sarfasl_TakmilU.pas` (`TSarfasl_Takmil`) — "complete the account details". Opened from
`SNewu.pas:600-616` (`BitBtn4`, caption `'اطلاعات تکمیلی'` = "supplementary information") and from
the legacy list (`ListSarfaslu.pas:106-116`).

Precondition (`SNewu.pas:606-610`): node must be a leaf —
`' در سطح آخر حساب نیست'` ("this is not the last account level").

Fields (`Sarfasl_TakmilU.pas:135-142` load, `Sarfasl_TakmilU.pas:65-84` save):

| Control | `Sarfasl` column | Persian label | English |
|---|---|---|---|
| `CoName` | `S_Name` | نام | Name |
| `CoAddress` | `S_Address` | آدرس | Address |
| `CoSabt` | `S_Sabt` | شماره ثبت | Registration number |
| `CoMelli` | `S_Melli` | کد ملی | National ID |
| `CoEgh` | `S_Egh` | کد اقتصادی | Economic code |
| `CoPost` | `S_Post` | کد پستی | Postal code |
| `CoFax` | `S_Fax` | فاکس | Fax |
| `CoTel` | `S_Tel` | تلفن | Phone |

Single validation (`Sarfasl_TakmilU.pas:59-64`):

| Check | Persian | English |
|---|---|---|
| `Trim(CoName.Text)` empty | `نام سرفصل  را وارد کنید` | "Enter the account name" |

Success: `' ثبت انجام شد '` ("saved") — `Sarfasl_TakmilU.pas:85`.

The four `S_IS_*` role checkboxes were on this form but are **commented out** in both the load
(`Sarfasl_TakmilU.pas:143-146`) and save (`Sarfasl_TakmilU.pas:75-82`) paths — they were replaced by
`base_config` (§1.9).

---

_Prev: [03-02-a-account-crud-rules](03-02-a-account-crud-rules.md) | Next: [03-03-a-voucher-sanad-model](03-03-a-voucher-sanad-model.md)_
