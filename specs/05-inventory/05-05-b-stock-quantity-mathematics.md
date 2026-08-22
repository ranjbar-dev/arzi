_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 5.2 Negative stock

#### 5.2.1 The only check in the system

`AnbarFactorAddU.pas:173-187`, in the line-editor's OK button:

```pascal
procedure TAnbarFactorAdd.B_OKClick(Sender: TObject);
begin
   if AnbarFactor.AF_Type.Tag = 2  then
   if DM.Anbar_Jens_Phi1.fieldByName('AJ_Manfi').Asinteger <> 1 then
   if Rem1.Tag - Num.FloatValue < 0 then
   Begin
      MessageDlg('تعداد وارد شده بیشتر از موجودی انبار است', mterror, [mbok], 0);
      ActiveControl := Num;
      Exit;
   End;
   if Total.IntValue =0 then Exit;
   tag := 1;
   Close;
end;
```

`'تعداد وارد شده بیشتر از موجودی انبار است'` — "The quantity entered is greater than the stock on
hand."

`Rem1.Tag` is set at `AnbarFactorAddU.pas:129-132`:

```pascal
I :=  Dm.Anbar_Jens_Phi1.FieldByName('Noin').Asinteger  - Dm.Anbar_Jens_Phi1.FieldByName('Noout').Asinteger
    - Dm.Anbar_Jens_Phi1.FieldByName('NoBin').Asinteger + Dm.Anbar_Jens_Phi1.FieldByName('NoBout').Asinteger ;
Rem1.tag := I;
```

#### 5.2.2 Six ways the check fails

1. **It fires only for `AF_Type = 2`.** Type `3` (return to supplier) also *reduces* stock — see
   the sign table in §5.1.1 — and is never checked. You can return 500 kg of an item you hold
   10 kg of and the system will book it.
2. **It is opt-out per item, and the opt-out is ON by default.**
   `Anbar_Jens.AJ_Manfi = 1` disables the check entirely.
   > **Correction (added while writing §2 and §6).** An earlier draft of this bullet stated that
   > `AJ_Manfi` is not editable from any screen and that its default is unknown. Both statements
   > are wrong. The column **is** editable: `AnbarCalaAddU` exposes it as a two-state slider
   > `AJ_Manfi: TsSlider` (`AnbarCalaAddU.pas:14`), captioned `منفی شدن موجودی` ("stock going
   > negative", `AnbarCalaAddU.dfm:43`) with positions `بله`/`خیر` (yes/no, `:139-140`), and it is
   > written on both insert and update (`AnbarCalaAddU.pas:150-152, 168, 173-175`). Its default
   > for a **new** item is set by the UI at `AnbarCalaAddU.pas:74` — `AJ_Manfi.SliderOn := True;`
   > — i.e. `1`, i.e. **negative stock permitted**. This confirms the data-layer agent's finding
   > that the column is `int NOT NULL DEFAULT 1`: the schema default and the UI default agree,
   > and both permit negative stock. Full CRUD detail in §2.
3. **It truncates fractional stock.** `Noin`…`NoBout` are read with `.AsInteger` even though the
   underlying `AFD_Num` is `Numeric(14,3)` and `Anbar_MandehU` reads the same figures as `TBCDField`
   with 3 decimals (`Anbar_MandehU.pas:22-35`). An item with 0.4 kg on hand reads as `0`; an item
   with 10.9 reads as `10`. It rounds *toward zero*, so it is conservative for the balance itself
   but silently wrong at the boundary, and it makes the check useless for any unit of measure that
   is not whole (`AJ_Vahed = 'کیلوگرم'`, kilogram, is the common case).
4. **It does not consider the lines already in the current invoice.** `Rem1.Tag` comes from the
   database via `Anbar_Jens_Phi1`, which excludes the entire current invoice
   (`AFD_Factor <> @F`) and knows nothing of the in-memory `CDS1` grid. With 10 on hand you may
   add "item X, qty 10" five times; each line passes independently, and the invoice ships −40.
5. **It is a per-line UI check, not an invariant.** `B_SaveClick` (`AnbarFactorU.pas:567-667`)
   re-validates nothing. Rows loaded by `LoadFactor` (`:697-739`), by the "load stock as invoice"
   helper `OP1Click` (`:204-239`), or by the `.Green` INI-file import `OP3Click` (`:288-340`)
   never pass through `B_OKClick` at all and are therefore never checked.
6. **There is no database constraint.** The write path is the stored procedure
   `SP_AnbarAddToFactor` (`AnbarFactorU.pas:628-643`); its body is not in the repo, but its
   parameter list carries no stock or validation output, and the caller ignores its result.

#### 5.2.3 How negative stock is dealt with instead: after the fact

`Anbar_MandehU.pas:213-231` — a toggle button captioned `مانده منفی` ("negative balance")
that filters the finished report to `R2<0`:

```pascal
B_filter.Tag := 1;
Q1.Active:=False;
Q1.Filtered := True;
Q1.Filter := 'R2<0';
Q1.Open;
```

That is the system's actual negative-stock policy: allow it, then let a human find it in a report.

**Port decision required (§15):** whether the rebuild enforces non-negative stock at the service
boundary or reproduces the permissive behaviour. Default per instructions is port-as-is, but
historical data will contain negative balances and any hard constraint will reject a migration.

---

### 5.3 Confirmed defect: an inventory document can be posted to the ledger twice

This defect was found from the accounting side and is confirmed here from the inventory side. It
lives entirely in subsystem B (`FactorMaster`).

#### 5.3.1 The posting path writes nine possible `M_Id` values

`MakeSanadU.pas` is the voucher generator for warehouse documents. Its four public entry points
each hard-code a different `Moein.M_Id`:

| Entry point | `.pas:line` | Source `FM_ID` | Sets `_ID` to | Caption |
|---|---|---|---|---|
| `init11` | `MakeSanadU.pas:186,206` | `11` | **`31`** | `صدور سند موجودی اول دوره` — "issue opening-stock voucher" |
| `init12` | `MakeSanadU.pas:321,341` | `12` | **`32`** | `صدور سند خرید مواد و کالا` — "issue purchase voucher" |
| `init22` | `MakeSanadU.pas:590,610` | `22` | **`33`** | `صدور سند فروش` — "issue sales voucher" |
| `init13` | `MakeSanadU.pas:456,476` | `13` | **`35`** | `صدور سند برگشت از فروش` — "issue sales-return voucher" |

A fifth value, `34`, is written by a completely different unit: `FactorPesteh_U.pas:1` documents
`// moein id 34 = kharid pesteh` and `:224,226` insert with literal `34`. Values `36`–`39` are
reserved but unused.

The idempotency guard at the top of `B_OkClick` deletes the **full range**
(`MakeSanadU.pas:84-86`):

```pascal
QS.SQL.Add(' Delete moein ');
QS.SQL.Add('  Where M_Coid='+ inttostr(Dm.CO_ID) +' and  M_Id in(31,32,33,34,35,36,37,38,39) and M_Link='+ inttostr(_SSN) );
```

and the voucher-number allocator asks for the same full range (`MakeSanadU.pas:314,449,583,716`):

```pascal
DM_Sanad.IntValue := Dm.Get_NewSanad_DateID( DM_Date.Text, '31,32,33,34,35,36,37,38,39' );
```

Then it stamps the source document (`MakeSanadU.pas:121-123`):

```pascal
QS.SQL.Add('  Update Anbar.Dbo.FactorMaster Set FM_Lock=2, FM_SanadNo='+DM_Sanad.Inttext+' ,FM_SanadDate='+ QuotedStr(DM_Date.Text) );
QS.SQL.Add('     Where FM_SSN= '+ inttostr(_SSN) );
```

#### 5.3.2 The un-posting path deletes only three of them

`SodoorSanadU.pas:250-263`, `B_DeleteClick` — "delete the voucher for this document":

```pascal
QS.SQL.Add(' Begin Transaction ');
QS.SQL.Add('Declare @Coid int Set @Coid='+ inttostr(_Coid) );
QS.SQL.Add('Declare @Sanad int Set @Sanad='+inttostr(_Sanad) );
QS.SQL.Add('Declare @link int Set @Link='+inttostr(_Link) );
QS.SQL.Add('Declare @Factor int Set @Factor='+inttostr(_Factor) );
// Update moein
QS.SQL.Add('Delete moein Where M_Coid=@Coid and M_Sanad=@Sanad and M_Link=@Link and M_id in (32,33,35) ');
// update factormaster
QS.SQL.Add('Update Anbar.DBO.FactorMaster  Set FM_SanadNo=0 , FM_SanadDate='''' , FM_Lock=1 ');
QS.SQL.Add('    Where FM_Coid=@Coid and FM_Factor=@Factor and FM_SSN=@Link');
// update factordetail
QS.SQL.Add(' Commit');
QS.ExecSQL;
DM.Dmoein_UpdateMab(_Sanad);
```

`M_id in (32,33,35)` — **`31` and `34` are missing.**

#### 5.3.3 Exact effect

Take an opening-stock document (`FM_ID = 11`, posted by `init11` with `M_Id = 31`).

1. **Post.** `B_SodoorClick` (`SodoorSanadU.pas:193-195`) routes `FM_ID=11` to `init11`.
   `init11` allocates voucher `S`, writes 2–4 `Moein` lines with `M_Id=31, M_Link=FM_SSN,
   M_Sanad=S`, and sets `FM_Lock=2, FM_SanadNo=S`.
2. **Un-post.** `B_DeleteClick` requires `FM_Lock=2` (`SodoorSanadU.pas:220-224`), then runs the
   delete above. `M_Id=31` is not in `(32,33,35)`, so **the voucher lines survive**. But
   `FM_Lock` is set back to `1` and `FM_SanadNo` to `0`, so the *link from document to voucher is
   erased*. `Dmoein_UpdateMab(_Sanad)` then recomputes the `DMoein` header total for the voucher
   which — still holding all its lines — is unchanged. From the operator's point of view the
   un-post reported `'انجام شد'` ("done") and succeeded.
3. **Re-post.** The document is now `FM_Lock=1`, which is exactly the state `B_SodoorClick`
   requires (`SodoorSanadU.pas:176-185` rejects `FM_Lock=0` "not confirmed" and `FM_Lock>1`
   "voucher already issued"). It routes to `init11` again.
   `init11` allocates a **new** voucher number `S'` via `Get_NewSanad_DateID`.
   `B_OkClick`'s idempotency delete *does* cover `31`, and it is **not scoped by `M_Sanad`** —
   it deletes `M_Id in (31..39) and M_Link=_SSN` for the whole fiscal year. So the orphaned lines
   under `S` are removed and re-created under `S'`.

**Therefore the defect does *not* produce doubled amounts on re-post through the same screen.**
The idempotency delete in `MakeSanadU` is broader than the un-post delete in `SodoorSanadU`, and
it saves the situation. What actually goes wrong is this:

| Symptom | Mechanism | Severity |
|---|---|---|
| **Orphaned voucher lines** | After un-post, `Moein` still contains `M_Id=31` lines under voucher `S` with `M_Link` pointing at a document that now claims to have no voucher (`FM_SanadNo=0`). Voucher `S` remains in the trial balance, the journal and the general ledger. Nothing in the inventory UI can find or remove them — `SodoorSanadU` lists documents, and this document no longer points at `S`. | **High.** Silent overstatement/understatement of whatever accounts `init11` touched, persisting until someone reconciles by hand. |
| **`DMoein` header retained for a voucher nobody owns** | `Dmoein_UpdateMab(_Sanad)` is called but recomputes to a non-zero total (the lines are still there), so the `DMoein` header for `S` is *not* cleaned up either. | High. |
| **Voucher-number burn** | Every un-post/re-post cycle consumes a fresh voucher number and leaves the old one occupied by orphans. Voucher numbering has permanent gaps that are not gaps. | Medium. |
| **Same for pistachio purchases (`M_Id = 34`)** | `FactorPesteh_U` writes `M_Id=34` (`:224,226`) and sets `FM_Lock=2` directly at insert time (`:201`, the literal `2` in the `FactorMaster` insert). If an operator un-posts such a document from `SodoorSanadU`, the `34` lines survive *and* `MakeSanadU` can never clean them, because `B_SodoorClick` has no branch for `FM_ID=14` — it falls through to `'Not implemented yet.'` (`SodoorSanadU.pas:202-204`). The orphans are then **permanent**. | **Critical.** No code path in the repository can delete an orphaned `M_Id=34` line. |

#### 5.3.4 Blast radius

- Affected document kinds: `FM_ID = 11` (opening stock) via `M_Id=31`, and `FM_ID = 14`
  (pistachio purchase receipt) via `M_Id=34`.
- Unaffected: `FM_ID = 12/22/13` → `M_Id = 32/33/35`, which the un-post does delete.
- Also never posted at all: `FM_ID = 15,25` (production in/out), `16,26` (inter-warehouse
  transfer), `21` and any other kind — `B_SodoorClick` has no branch and shows
  `' Not implemented yet. '` (`SodoorSanadU.pas:203`). These documents move stock in the external
  warehouse system with **no accounting entry whatsoever**. See §3 and §10.
- The corresponding accounting-side observation is in `docs/03-accounting-core.md`; both sides
  describe the same two SQL statements.

**Rebuild requirement:** the posting/un-posting pair must be one symmetric operation over a single
declared set of source-document ids, ideally a `voucher.source_document_id` foreign key with
`ON DELETE` semantics rather than two hand-written `IN` lists that drifted apart.

---

### 5.4 Other quantity-integrity gaps found while tracing

| Gap | Evidence | Effect |
|---|---|---|
| Editing an invoice deletes **all** its lines and re-inserts them | `AnbarFactorU.pas:617-622` (`Delete Anbar_FactorD ... Delete Moein ... M_ID=1`) then the insert loop at `:627-645` | Line identity (`AFD_SSN`) is not stable across edits. Any external reference to a line id breaks. Also, the delete/insert is **not** in a transaction — `QS.ExecSQL` at `:622` commits, and a failure in the loop at `:627` leaves the invoice with zero or partial lines while the header still exists. |
| Invoice save is not transactional at all | `AnbarFactorU.pas:567-667` — five separate `ExecSQL`/`Post` round trips with no `Begin Transaction` | A crash mid-save leaves `Anbar_Factor` present, `Anbar_FactorD` empty, `Moein` empty, and `AF_Total` stale. Contrast `AnbarListU.pas:379-388` (delete) and `:433-457` (renumber), which *are* wrapped in `Begin Transaction … Commit`. |
| Header money totals are caches recomputed by four separate `UPDATE`s | `AnbarFactorU.pas:654-661` | `AF_Total`, `AF_Kasr`, `AF_Maliat`, `AF_Mab` are derived sums over `Anbar_FactorD`. They drift if lines are altered by any other path. Same failure mode as `DMoein` in `docs/03-accounting-core.md`. |
| Renumbering an invoice does not renumber the voucher | `AnbarListU.pas:440-454` updates `Anbar_Factor`, `Anbar_FactorD`, `Moein.M_Link`, `DFish.S_LinkSSN`, `DCheck.S_LinkSSN` — but only `Where M_link=@Old ... and M_Sanad=@Sanad` | Correct as written, but it depends on `AF_Sanad` still matching; if the invoice was saved twice, `B_SaveClick` allocates a *new* `AF_Sanad` each time (`AnbarFactorU.pas:593`) and abandons the old one after calling `Dmoein_UpdateMab(_OLD)` at `:649`. The old voucher's inventory lines were deleted at `:621` (`M_ID=1 and M_Link=<factor>`, unscoped by `M_Sanad`), so this one is handled — but only because that delete is also broader than it looks. |
| No unique constraint is enforced in code on `(AF_COID, AF_Factor)` | `New_AnbarFactor` is `Select isnull(Max(AF_Factor),0)+1` (`Dmu.pas:1253-1262`) with no locking | Two concurrent users creating an invoice at the same instant both get the same number. The renumber screen has an explicit duplicate check (`AnbarListU.pas:423-429`, `'شماره فاکتور تکراري است'` — "duplicate invoice number") but the create path has none. |


---

[← 5. Stock quantity mathematics (part a)](05-05-a-stock-quantity-mathematics.md) | [index](00-index.md) | [6. Costing and valuation (part a) →](05-06-a-costing-and-valuation.md)
