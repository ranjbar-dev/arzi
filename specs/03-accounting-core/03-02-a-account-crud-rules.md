_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 2. Account CRUD rules

The live chart-of-accounts maintenance screen is `SNewu.pas` (`TSNew`, caption
`'سرفصلهای حسابداری'` = "Accounting chart of accounts", `SNewu.dfm:5`). It is a drill-down grid, one
level at a time.

### 2.1 Create

**Entry point:** `B_Add` speed button (`SNewu.pas:618`), hint `'+ افزودن کد جدید'` ("+ add new code"),
caption `'+ جدید'`. Also bound to the numeric-keypad **`+`** key in the grid (`SNewu.pas:446-456`,
virtual key `107`).

**Step 1 — propose the next code** (`SNewu.pas:545-596`):

| Level (`State`) | SQL | Fallback if empty |
|---|---|---|
| 1 (Kol) | `Select isnull( Max(S_ko)+1 , 111 ) as code From Sarfasl Where S_Mo=0` | `111` |
| 2 (Moein) | `Select isnull( Max(S_mo)+1 , 111 ) as code From Sarfasl Where S_Ko=<Kol> And S_Ta1=0` | `111` |
| 3 (Tafsil 1) | `Select isnull( Max(S_Ta1)+1 , 1 ) as code From Sarfasl Where S_Ko=<Kol> And S_Mo=<Moein> And S_Ta2=0` | `1` |
| 4 (Tafsil 2) | `Select isnull( Max(S_Ta2)+1 , 1 ) as code From Sarfasl Where S_Ko=<Kol> And S_Mo=<Moein> And S_Ta1=<Taf1>` | `1` |

Note the level-1 query is **not** scoped by anything — a global max. Levels 2–4 are scoped to the
parent. Note also that level 4's query omits an `S_Ta2` predicate (unlike 2 and 3 which exclude
deeper rows) — harmless because there is no level 5.

**Step 2 — prompt** (`SNewu.pas:625-626`) using the generic code+name dialog
`GetCodeName` (`CodeNameU.pas:25-54`):

```pascal
    if Not GetCodeName('ایجاد کد جدید', 'کد حساب', 'نام حساب', 6, 50, Code, St, 2)
    Then  Exit;
```

Title `'ایجاد کد جدید'` ("create new code"), field 1 label `'کد حساب'` ("account code", max 6 digits),
field 2 label `'نام حساب'` ("account name", max 50 chars), right-aligned.
The **OK button is disabled** until both the name is non-empty and the code is `> 0`
(`CodeNameU.pas:47`, `CodeNameU.pas:58`):

```pascal
   BOk.Enabled := (Length( Trim( _Name.Text )) > 0) and ( _Code.IntValue>0)  ;
```

**Step 3 — call the stored procedure** (`SNewu.pas:628-649`):

```pascal
    SP_Add.Parameters.ParamByName('@Ko').Value := Kol;
    SP_Add.Parameters.ParamByName('@Mo').Value :=  Moein;
    SP_Add.Parameters.ParamByName('@Ta1').Value := Taf1;
    SP_Add.Parameters.ParamByName('@Ta2').Value :=  Taf2;
    SP_Add.Parameters.ParamByName('@Name').Value :=  ST;

    if State=1 then SP_Add.Parameters.ParamByName('@Ko').Value := Code;
    if State=2 then SP_Add.Parameters.ParamByName('@Mo').Value := Code;
    if State=3 then SP_Add.Parameters.ParamByName('@Ta1').Value := Code;
    if State=4 then SP_Add.Parameters.ParamByName('@Ta2').Value := Code;
    SP_Add.Active := True;
    if SP_Add.FieldByName('_Error').AsInteger>0 Then
         MessageDlg( Sp_Add.FieldByName('_Desc').AsString  , mterror, [mbok] , 0);
```

The current drill-down context `(Kol, Moein, Taf1, Taf2)` supplies the parent path; the entered code
replaces the component for the current level. Validation is delegated to `Sarfasl_ADD`, which returns
`_Error` (0 = success) and `_Desc` (a Persian message). **The body of `Sarfasl_ADD` is not in this
repository** — its rules are an open question (§14).

**The legacy create screen** (`NewSarfaslu.pas`, unreachable — see §0.2) performed these checks
client-side, and its rules are the best available evidence for what `Sarfasl_ADD` does:

| # | Check | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | `Trim(Name.Text)` is empty | `نام کد را وارد کنيد` | "Enter the code name" | `NewSarfaslu.pas:187` |
| 2 | Tuple `(Kol, Moein, Taf, Taf2)` already exists | `کد وارد شده تکراري است` | "The entered code is duplicate" | `NewSarfaslu.pas:198` |
| — | Success | `کد ثبت شد` | "Code saved" | `NewSarfaslu.pas:217` |

Column defaults written on create (`NewSarfaslu.pas:204-215`):

```pascal
   Sarfasl.FieldByName('S_ko').AsInteger := kol.IntValue;
   Sarfasl.FieldByName('S_mo').AsInteger := moein.IntValue;
   Sarfasl.FieldByName('S_ta1').AsInteger := taf.IntValue;
   Sarfasl.FieldByName('S_ta2').AsInteger := taf2.IntValue;
   Sarfasl.FieldByName('S_name').Asstring := name.Text;
   Sarfasl.FieldByName('S_bed').Asstring := '0';
   Sarfasl.FieldByName('S_bes').Asstring := '0';
   Sarfasl.FieldByName('S_remi').Asstring := '0';
   Sarfasl.FieldByName('S_count').Asstring := '0';
   Sarfasl.FieldByName('S_active').Asstring := '1';
```

`S_Child`, `S_Lock`, `FullName`, `M_R`, `M_L`, `LineName` are left to the server.

### 2.2 Rename (change `S_Name`)

`B_EditName` (`SNewu.pas:315-352`), hint `'اصلاح نام کد'` ("edit code name"), caption `'نام'`.

**No preconditions.** A name can be changed at any time, on any node, even one with postings.

Prompt: `GetString('تغییر نام', 'نام جدید', 50, S )` — title "change name", label "new name",
max 50 (`SNewu.pas:328`).

```sql
-- SNewu.pas:332-348, verbatim
 Begin Transaction

 Update Sarfasl Set S_Name='<new name>'
     Where S_SSN=<id>
 Update Sarfasl Set NeedUpdate=1 Where S_Ko=<K>
    and S_Mo=<M>        -- appended only if M>0
    and S_ta1=<T1>      -- appended only if T1>0
    and S_ta2=<T2>      -- appended only if T2>0
 exec Active_Set

 Commit
```

The `NeedUpdate=1` marker plus `exec Active_Set` is the mechanism that rebuilds `FullName`, `M_R`,
`M_L`, `LineName` for the affected subtree.

**Bug worth recording (do not port):** the `and S_Mo=` / `and S_ta1=` / `and S_ta2=` fragments are
appended to the SQL script *unconditionally in sequence* but each is guarded by its own `if`
(`SNewu.pas:337-344`). Because `SQL.Add` appends lines to one script, if `K>0` but `M=0` the
`Update ... Where S_Ko=<K>` statement is emitted alone — correct. But if `M>0` and `T1=0` and `T2>0`
(impossible in a well-formed tree) the fragments would attach wrongly. In practice the tree invariant
saves it. The rebuild should scope the dirty-marking by the full ancestor path explicitly.

### 2.3 Renumber (change the code of a node)

`B_EditCode` (`SNewu.pas:244-313`), hint `'اصلاح کد'` ("edit code"), caption `'کد'`.

Validations, **in order**:

| # | Check | Persian message | English | Cite |
|---|---|---|---|---|
| 1 | Grid empty (`Q1.RecordCount = 0`) | — (silent `Exit`) | — | `SNewu.pas:248` |
| 2 | Node has children (`SNO > 0`) | `' این کد زیر شاخه دارد و قابل تغییر نیست '` | "This code has sub-branches and cannot be changed" | `SNewu.pas:251` |
| 3 | Node has postings (`SanadCode > 0`) | `' بر روی این کد سند صادر شده است و قابل تغییر نیست '` | "A voucher has been issued against this code and it cannot be changed" | `SNewu.pas:257` |
| 4 | New code equals old code | — (silent `Exit`) | — | `SNewu.pas:264` |
| 5 | New code duplicates a sibling | `' کد داده شده تکراری است و تغییر غیر قابل اجرا است '` | "The given code is duplicate and the change is not executable" | `SNewu.pas:268` |

Prompt: `GetNo('تغییر کد حساب', 'کد جدید را وارد کنید' , OldCode )` — "change account code" /
"enter the new code" (`SNewu.pas:263`).

`SanadCode` — the "has postings" test (`SNewu.pas:130-147`):

```sql
-- SNewu.pas:134-143
 Select M_Coid, M_Sanad From Moein Where M_Ko=<S_Ko>
   and M_Mo=<S_Mo>      -- appended only if S_Mo>0
   and M_Ta1=<S_Ta1>    -- if S_Ta1>0, else: and M_ta1=0
   and M_Ta2=<S_Ta2>    -- if S_Ta2>0, else: and M_ta2=0
 Group By M_Coid , M_Sanad
```

Returns the number of distinct `(fiscal_year, voucher)` pairs touching this account **or any
descendant** (because the deeper components are only constrained when non-zero). Note this scans
**all fiscal years**, not just the current one — correct, since the chart is global.

Update SQL (`SNewu.pas:272-307`) — one of four shapes depending on `State`:

```sql
 Begin Transaction
 -- State=1 (Kol):
 Update Sarfasl Set NeedUpdate=1 , S_Ko=<new> Where S_Ko=<old>
 -- State=2 (Moein):
 Update Sarfasl Set S_Mo=<new> Where S_Ko=<parentKol> And S_Mo=<old>
 -- State=3 (Tafsil 1):
 Update Sarfasl Set S_ta1=<new> Where S_Ko=<K> And S_Mo=<M> And S_ta1=<old>
 -- State=4 (Tafsil 2):
 Update Sarfasl Set S_ta2=<new> Where S_Ko=<K> And S_Mo=<M> And S_ta1=<T1> And S_ta2=<old>

 exec Active_Set
 Commit
```

Success message: `'تغییر کد انجام شد'` ("the code change was performed") — `SNewu.pas:311`.

**Two defects to fix in the rebuild, not port:**
1. Only `State=1` sets `NeedUpdate=1`; levels 2–4 do not, so `Active_Set` may not refresh their
   denormalised strings.
2. Guard #2 (`SNO > 0`) means only *leaves* can be renumbered. But guard #3's `SanadCode` query is
   written to also cover descendants — dead logic given guard #2. Harmless.
3. `Kol := newCode` is assigned unconditionally at `SNewu.pas:308` even when renaming at level 2–4,
   corrupting the drill-down breadcrumb. A cosmetic bug.

---

_Prev: [03-01-b-account-hierarchy-model](03-01-b-account-hierarchy-model.md) | Next: [03-02-b-account-crud-rules](03-02-b-account-crud-rules.md)_
