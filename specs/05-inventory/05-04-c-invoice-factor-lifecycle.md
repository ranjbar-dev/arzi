_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

#### 4.2.3 Edit

`Anbar0Click` (`AnbarListU.pas:269-283`) → `AnbarFactor.EditFactor(F)`
(`AnbarFactorU.pas:180-202`):

```pascal
ClearForm;
if FN = 0 Then FN:= GetNo('اصلاح فاکتور' , 'شماره فاکتور' , 0 );   // "correct invoice" / "invoice number"
if FN=0 then exit;
LoadFactor(FN);
_NewEditView := 2;
if Dm.Moein_TX( AF_Sanad.intvalue ) > 0 then
Begin
    Application.MessageBox( 'براي اصلاح فاکتور سند را در حالت تحرير قرار دهيد', 'Error');
    _NewEditView := 3;
    B_Add.enabled:=False; B_Edit.enabled:=False;
    B_Delete.enabled:=False; B_Save.enabled:=False;
End;
```

`'براي اصلاح فاکتور سند را در حالت تحرير قرار دهيد'` — **"to correct the invoice, put the voucher
into draft (تحریر) state"**. This is the immutability rule, stated in one line. It is enforced
*after* the invoice has already been loaded into the form, so a frozen invoice is fully visible,
just not saveable.

Editing is destructive-and-recreate: `LoadFactor` (`:697-739`) pulls the header and every line
into the in-memory `CDS1`, and `B_SaveClick` deletes and re-inserts everything (step 7/8). There
is no differential update and no optimistic-concurrency check — two users editing the same invoice
silently overwrite each other, last writer wins.

#### 4.2.4 View and print

- `AR_Charp4` → `AR_Chap4Click` (`AnbarListU.pas:302-310`) → `ViewFactor(F)`
  (`AnbarFactorU.pas:453-468`): sets `_NewEditView := 3`, disables the four mutating buttons.
- `AR_Chap` (`چاپ فاکتور`, "print invoice") → `Factorprint3.init(AF_Factor)`.
- `AR_Chap2` (`چاپ فاکتور رسمی`, "print official invoice") → `Factorprint2.init(AF_Factor)`.
- `sButton8` (`چاپ لیست`, "print list") → `AR_Chap3Click`, the FastReport `RP1` over the list.
- `TAnbarFactor.PrintFactor` (`:343-354`) also exists, sets `State := 3` and calls
  `UpdateState` — but **nothing calls it**: `AnbarListU.pas:317` is the commented-out line
  `// AnbarFactor.PrintFactor( … )`. Dead entry point.

> **Defect — `State` is a sticky global that can lock out a fresh invoice.**
> `TAnbarFactor` has two overlapping mode variables (`AnbarFactorU.pas:86-89`): `_NewEditView`
> (1 new / 2 edit / 3 read-only) and `State`, commented `// 1-new 2-Edit 3-view-print 5-delete`.
> `State` is assigned in exactly two places: `0` in `FormCreate` (`:121`) and `3` in the dead
> `PrintFactor` (`:346`). It is **never reset** — `ClearForm` (`:671-695`), `NewFactor` and
> `ViewFactor` all leave it alone. Yet `B_AddClick`, `B_EditClick` and `B_DeleteClick` all guard
> on `if State > 2` (`:494, 518, 557`) and refuse with
> `'اجازه تغييرات در فاکتور را نداريد'` ("you are not permitted to change the invoice").
> Because the form is a Delphi auto-created singleton, **once anything sets `State := 3` the line
> buttons stay dead for the rest of the process**, on every subsequent new invoice — while
> `B_Save` (which guards on `_NewEditView`) still works, so the user can save an empty invoice
> they cannot add lines to. In the shipped build `PrintFactor` is unreachable, so `State` stays
> `0` and the guards are simply inert: `if State > 2` never fires, and `UpdateState`'s
> `B_Add.Enabled := (State=1) or (State=2)` would disable everything if it ran — which it only
> does from `PrintFactor`. **Net effect today: two of the three mode guards in the invoice screen
> are dead code, and the read-only mode is enforced solely by the four explicit
> `B_*.enabled := False` assignments in `ViewFactor` and `EditFactor`.** Do not port `State`.

#### 4.2.5 Delete

`AR_DeleteClick` (`AnbarListU.pas:320-397`). Five preconditions, then one transaction:

| # | Check | Line | Persian message | English |
|---|---|---|---|---|
| 1 | Fiscal year active | `:326` | (see §4.2.1) | |
| 2 | User confirms | `:332` | `فاکتور <n> حذف شود؟` | "Delete invoice \<n\>?" |
| 3 | The voucher exists | `:337-345` | `سند پیدا نشد` | "Voucher not found" |
| 4 | The voucher is in draft | `:346-351` | `سند معین را در حالت تحریر قرار دهید` | "Put the subsidiary voucher into draft state" |
| 5 | No deposit slip attached | `:355-363` | `فاکتور حاوی فیش واریزی می باشد` | "The invoice has a deposit slip attached" |
| 6 | No post-dated cheque attached | `:367-375` | `فاکتور حاوی چک موعدی می باشد` | "The invoice has a post-dated cheque attached" |

```sql
Begin Transaction
Declare @F int Set @F=<AF_Factor>
Declare @C int Set @c=<AF_COID>
Declare @S int Set @S=<AF_Sanad>
Delete Moein          Where M_Sanad=@S and M_Coid=@C and M_Link=@F and M_id in (1,2,3,4,5,6,7,8,9)
Delete Anbar_FactorD  Where AFD_Coid=@C and AFD_Factor=@F
Delete Anbar_Factor   Where AF_Coid=@C  and AF_Factor=@F
Commit
```
followed by `Dm.Dmoein_UpdateMab(S)` (`:390`) — **outside** the transaction — and
`فاکتور حذف شد` ("the invoice has been deleted").

Observations:

- Delete is a **hard delete**. No soft-delete flag, no archive, no audit row. `docs/01-glossary.md`
  §6b already records that no audit trail exists anywhere in the system.
- Unlike the save path, this `Moein` delete **is** scoped by `M_Sanad` *and* covers the full
  `1..9` range — the mirror image of the §5.3 asymmetry, and correct here.
- `Dmoein_UpdateMab` is outside the transaction, so a crash between `Commit` and that call leaves
  a `DMoein` header whose total no longer matches its (now fewer) lines. Same drift class as
  `docs/03-accounting-core.md`.
- Grid-level deletion is separately blocked: `Q1BeforeDelete` calls `abort`
  (`AnbarListU.pas:188-191`) and `G1KeyDown` swallows Ctrl+Delete (`:193-197`).

#### 4.2.6 Renumber

`AR_ReNoClick` (`AnbarListU.pas:399-465`), button `تغییر شماره فاکتور` ("change invoice number").
Prompts for a new number (`GetNo('تغيير شماره فاکتور','شماره فاکتور جديد', _Old)`), rejects
duplicates with `شماره فاکتور تکراري است` ("duplicate invoice number", `:427`) — **the only
uniqueness check on `AF_Factor` anywhere in the codebase** — then runs one transaction updating
five tables:

```sql
Update Anbar_Factor  Set AF_Factor  = @New Where AF_Factor  = @Old and AF_Coid   = @COID
Update Anbar_FactorD Set AFD_Factor = @New Where AFD_Factor = @Old and AFD_Coid  = @COID
Update Moein         Set M_Link     = @New Where M_link     = @Old and M_COID    = @COID and M_Sanad=@Sanad
Update DFish         Set S_LinkSSN  = @New Where S_LinkSSN  = @Old and S_LinkPRG = 1 and S_Coid=@COID
Update DCheck        Set S_LinkSSN  = @New Where S_LinkSSN  = @Old and S_LinkPRG = 1 and S_Coid=@COID
```

**This is the inventory-side confirmation of the treasury finding.** `DFish.S_LinkSSN` and
`DCheck.S_LinkSSN` are updated with the invoice **number**, not `AF_SSN`. The name says "SSN"
(surrogate key) and the content is a business document number. Corroborated independently by the
list query at `AnbarListU.pas:538-539`:

```sql
, payF = (Select Sum(S_Mab) From DFish  Where S_Linkprg=1 and S_Coid=AF_Coid and S_LinkSSN=AF_Factor )
, payC = (Select Sum(S_Mab) From DCheck Where S_Linkprg=1 and S_Coid=AF_Coid and S_LinkSSN=AF_Factor )
```
and by the delete preconditions at `:356,368`. **Confirmed: treasury links back to an inventory
invoice by `(S_LinkPRG = 1, S_COID, S_LinkSSN = AF_Factor)` — module, year, number.**

`S_LinkPRG = 1` is the source-module discriminator; `1` means "inventory invoice". See
`docs/06-treasury.md` for the full enumeration.

**Three different link conventions coexist on this one table:**

| Consumer | Links to an invoice by | Evidence |
|---|---|---|
| `Moein.M_Link` | `AF_Factor` (number) | `AnbarFactorU.pas:621`, `AnbarListU.pas:384,446` |
| `DFish.S_LinkSSN`, `DCheck.S_LinkSSN` | `AF_Factor` (number) | `AnbarListU.pas:356,368,450-454,538-539` |
| `Moadian.M_Link` | `AF_SSN` (surrogate key) | `AnbarListU.pas:537` |

Renumbering therefore updates the first two and correctly leaves the third alone. But it means
`AF_Factor` is a **mutable business key that other tables depend on**, and the renumber screen is
the only thing keeping them consistent. Any direct SQL renumber, or any table the author forgot,
silently orphans records. In the rebuild, everything links by surrogate id and `invoice_number`
becomes a plain unique attribute (§15).

Also note: renumbering does **not** touch `AF_Sanad`, and the `Moein` update is scoped by
`M_Sanad = @Sanad` taken from the grid row. §5.4 analyses why that is nevertheless safe.

#### 4.2.7 Settle

`AR_VarizClick` (`AnbarListU.pas:467-486`), button `تسویه فاکتور` ("settle invoice"). Refuses
anything but a sales invoice:

```pascal
if Q1.FieldByName('AF_Type').AsInteger <>2 then
    MessageDlg('   فقط برای فاکتورهای فروش فعال است   ', mterror, [mbok], 0);   // "only enabled for sales invoices"
```
then `TasfiehFactorF.Init_Factor( Q1.FieldByName('AF_SSN').AsInteger )` — **passing `AF_SSN`, the
surrogate key**, in contrast to everything else on this screen. Full treatment in §9.

---

### 4.3 Import and export of an unsaved invoice

Three popup-menu items on the invoice screen (`AnbarFactorU.dfm:556-573`) operate on the in-memory
line set:

| Item | Caption | English | Handler |
|---|---|---|---|
| `OP1` | `لیست مانده با متوسط قیمت تمام شده` | "list of balances at average cost price" | `:204-239` — §6.4 |
| `OP2` | `ذخیره اقلام وارد شده در فایل` | "save entered items to a file" | `:241-286` |
| `OP3` | `بارگزاری اقلام ذخیره شده قبلی` | "load previously saved items" | `:288-340` |

`OP2`/`OP3` write and read a **plain INI file** through `TMyIni`, with a `[Base]` section and one
`[Line<n>]` section per row. The format is self-identifying by two magic values checked on import
(`:306-318`):

```pascal
S1 := F1.Readstring('Base','Name', '');
if S1<>'Green Gold Anbar' then  … 'فایل باید حاوی اطلاعات فاکتور باشد' … ;  // "the file must contain invoice data"
I := F1.Readinteger('Base','ID', 0);
if I<>12 then                   … same message … ;
```

The export also writes the developer's own name, mobile number and e-mail as literals into every
exported file (`:256-260`): `'Mohsen Ranjbar'`, `'09131912805'`,
`'MohsenRanjbar.1350@Gmail.com'`. Flag for `docs/08-platform-and-security.md`.

**Import bypasses every validation.** `OP3Click` appends straight into `CDS1` with
`ReadString(…, '0')` defaults — no item-code existence check, no stock check (§5.2.2 point 5), no
recomputation of `Kol`/`Maliat`/`Total` from `Num × Phi`. A hand-edited INI file can therefore
inject an arbitrary item code with arbitrary, internally inconsistent money columns, and
`B_SaveClick` will write it. There is no signature and the magic values are in the source.

Both handlers also mutate `DM.PS` (the global "PS" file object) and restore it afterwards
(`:252-254, 280-283, 302-305, 337-338`) — an unexplained side effect on a shared object; if the
handler exits early (`:299` on a missing file) the restore is **skipped** and `DM.PS` is left
pointing at the wrong file. Minor, but it is a real leak of global state.

Also note `OP2Click` has no early exit if `CDS1` is empty, and writes `'0'+Kasr` /
`'0'+Maliat` (`:274-275`) — string concatenation producing `'0'` for an empty field and
e.g. `'0145000'` otherwise, which `StrToInt` parses back correctly but which is plainly an
accident.

---

### 4.4 What becomes immutable, and when

| Artefact | Mutable while… | Frozen by |
|---|---|---|
| Header fields (`AF_Date`, `AF_Customer`, `AF_Desc`, `AF_Type`) | `M_Tx = 0` | voucher finalisation |
| Lines (`Anbar_FactorD`) | `M_Tx = 0` | voucher finalisation |
| `AF_Factor` (the number) | `M_Tx = 0`, and only via the renumber screen | voucher finalisation (the renumber screen does not check `M_Tx` — **see below**) |
| `AF_Sanad` | never directly; reassigned on every save | — |
| `AFD_SSN` (line identity) | destroyed and recreated on every save | — |
| Existence (delete) | `M_Tx = 0` **and** no `DFish` **and** no `DCheck` | voucher finalisation or settlement |

> **Gap — renumber has no `M_Tx` check.** `AR_ReNoClick` (`AnbarListU.pas:399-465`) verifies the
> fiscal year, the target number's uniqueness and operator confirmation, but never calls
> `Moein_TX`. **A frozen invoice, whose voucher has been finalised, can still be renumbered**, and
> the renumber rewrites `Moein.M_Link` on that finalised voucher. Every other mutation is gated on
> `M_Tx = 0`; this one is not. Severity: medium — it does not change amounts, but it modifies a
> posted voucher's line data outside the accounting module's own controls.

> **Gap — nothing prevents editing a settled invoice.** Delete checks `DFish`/`DCheck`; **edit does
> not.** An invoice that has been fully settled with a cheque can have its lines and total changed,
> leaving the settlement amount unrelated to the invoice amount. The list screen surfaces the
> mismatch as the `payF` / `payC` columns (`AnbarListU.pas:538-539`) but nothing acts on it. §9.

---

### 4.5 Subsystem B's lifecycle is a different machine

For contrast, because §3 and §10 depend on it: `Anbar.Dbo.FactorMaster` **does** carry an explicit
status, `FM_Lock`:

| `FM_Lock` | Meaning | Enforced at |
|---|---|---|
| `0` | not confirmed | `SodoorSanadU.pas:176-185` rejects posting: "not confirmed" |
| `1` | confirmed, no voucher | the only state from which `B_SodoorClick` will post |
| `2` | voucher issued | `SodoorSanadU.pas:220-224` requires it for un-posting; `:176-185` rejects re-posting |

`arzi` only ever moves `1 → 2` (`MakeSanadU.pas:121-123`) and `2 → 1` (`SodoorSanadU.pas:406`).
`0 → 1` happens in the external warehouse application. The pistachio path skips the machine
entirely and inserts at `FM_Lock = 2` (§8.3.4).

**Rebuild consequence:** the two subsystems must converge on one lifecycle. The recommended target
is an explicit `status` column on the document with `draft → posted → reversed`, plus a
`posted_voucher_id` foreign key, replacing both `M_Tx`-inference (subsystem A) and `FM_Lock`
(subsystem B). This is a §15 proposal because it changes observable behaviour: today, finalising
an unrelated voucher can freeze an inventory invoice that merely shares its date.


---

[← 4. The invoice (Factor) lifecycle (part b)](05-04-b-invoice-factor-lifecycle.md) | [index](00-index.md) | [5. Stock quantity mathematics (part a) →](05-05-a-stock-quantity-mathematics.md)
