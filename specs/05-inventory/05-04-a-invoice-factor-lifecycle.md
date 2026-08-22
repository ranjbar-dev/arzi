_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 4. The invoice (Factor) lifecycle

### 4.0 There is no status column

`Anbar_Factor` has no `AF_State`, `AF_Status` or `AF_Lock`. Compare subsystem B, which does have
one (`FactorMaster.FM_Lock`, values `0` → `1` → `2`, §3, §5.3). **In subsystem A the invoice's
lifecycle state is entirely derived** from three things outside the invoice row:

| Derived state | Determined by | Where |
|---|---|---|
| Does the invoice exist? | a row in `Anbar_Factor` for `(AF_COID, AF_Factor)` | `AnbarFactorU.pas:704` |
| Is it still editable? | `max(Moein.M_Tx)` for `M_Sanad = AF_Sanad` — **`0` = editable, `> 0` = frozen** | `Dmu.pas:1166-1178`, called at `AnbarFactorU.pas:190,475` and `AnbarListU.pas:346` |
| Is it still deletable? | additionally: no `DFish` and no `DCheck` row links to it | `AnbarListU.pas:355-375` |

`M_Tx` is the accounting core's voucher-lock flag (`docs/03-accounting-core.md`). The inventory
invoice inherits the voucher's state; it has no state of its own. **This is the single most
important structural fact about the lifecycle** and it means the rebuild cannot simply add a
`status` column without deciding whether the ledger or the document is authoritative.

---

### 4.1 The real state machine

```
                    ┌──────────────────────────────────────────────┐
                    │  (no row)                                    │
                    └───────┬──────────────────────────────────────┘
                            │ T1/T2/T3/T4 on the list screen
                            │  guard: fiscal year IsActive = 1
                            │  guard: user permission 1404/1405/1406/1407
                            ▼
                    ┌──────────────────────────────────────────────┐
                    │  DRAFT-IN-MEMORY                             │
                    │  lines live in CDS1 (a TVirtualTable)        │
                    │  nothing is in the database yet              │
                    └───────┬──────────────────────────────────────┘
                            │ B_Save  (AnbarFactorU.pas:567)
                            │  allocates AF_Factor and AF_Sanad
                            ▼
                    ┌──────────────────────────────────────────────┐
                    │  SAVED / VOUCHER IN DRAFT   (M_Tx = 0)       │
                    │  Anbar_Factor + Anbar_FactorD + Moein M_ID=1 │
                    │  fully editable, fully deletable             │
                    └──┬──────────┬───────────────┬────────────────┘
                       │          │               │
      edit (Anbar0) ───┘          │               └─── settle (AR_Variz, §9)
      re-runs the whole save      │                    creates DFish / DCheck
                                  │                         │
                                  │                         ▼
                                  │              ┌─────────────────────────┐
                                  │              │  SETTLED (partly/fully) │
                                  │              │  still editable         │
                                  │              │  NOT deletable          │
                                  │              └─────────────────────────┘
                                  │
                                  │ someone finalises the voucher in the
                                  │ accounting module (M_Tx := 1 or 2)
                                  ▼
                    ┌──────────────────────────────────────────────┐
                    │  FROZEN   (M_Tx > 0)                         │
                    │  read-only. Edit refuses; Delete refuses.    │
                    │  Only View and Print remain.                 │
                    └──────────────────────────────────────────────┘
```

There is no "confirmed but unposted" state, no approval step, and **no posting step** — the
voucher lines are written by the same `Save` that writes the invoice (§10). Posting and saving are
the same action.

---

### 4.2 Transitions in detail

#### 4.2.1 Create

Entry points `T1Click` / `T2Click` / `T4Click` / `T3Click` (`AnbarListU.pas:218-256`), all
identical apart from the type argument:

```pascal
procedure TAnbarList_F.T1Click(Sender: TObject);
begin
    if DM.Is_New_Sanad_Valid( Dm.CO_ID)=False Then Exit;
     AnbarFactor.NewFactor( 1  );
     Reload;
end;
```

**Precondition 1 — the fiscal year must be open.** `Is_New_Sanad_Valid` (`Dmu.pas:997-1015`):

```pascal
if Base.Locate('CO_ID', Coid, …) = false then
   MessageDlg('   سال مالی پیدا نشد  ', …);          // "fiscal year not found"
if Base.FieldByName('IsActive').asinteger <> 1 then
   MessageDlg('          سال مالی مورد نظر بایگانی شده است                '+#13#10+
              '   اجازه تغییر در این سال و صدور فاکتور و سند را ندارید    ', …);
```
— "the selected fiscal year has been archived / you are not allowed to change this year or issue
invoices and vouchers in it". This same guard fronts **every** mutating action in the module:
`AnbarListU.pas:221, 231, 241, 251, 275, 326, 406, 472`; `FactorPesteh_U.pas:98, 114`.

**Precondition 2 — per-type permission.** `AnbarListU.pas:151-158`:

| Menu item | Type | Permission code | Persian caption source |
|---|---|---|---|
| `T1` | 1 receipt | `1404` | `AnbarListU.dfm` popup `PopupMenu1` |
| `T2` | 2 issue / sales invoice | `1405` | |
| `T4` | 4 sales return | `1406` | |
| `T3` | 3 purchase return | `1407` | |
| `Anbar0` (edit) | — | `1408` | |
| `AR_Chap` (print) | — | `1410` | |
| `AR_Delete` | — | `1414` | |
| `AR_Variz` (settle) | — | `2115` **or** `2101` (`:503`) | |

`Dm.IsEnabel(userId, code)` — see `docs/08-platform-and-security.md`. Note the permissions are
applied by **disabling** the buttons, not by re-checking inside the handler: `Anbar0Click`,
`AR_DeleteClick` and the rest contain no permission test of their own.

`NewFactor(FactorType)` (`AnbarFactorU.pas:136-178`) sets `AF_Type.Tag`, calls `ClearForm`, and
shows the form modally. Everything else in the body — a default-customer lookup driven by an INI
key `Base/NewFactorCustomer` — is **commented out** (`:154-174`). The `T_panel.visible` line at
`:152` is commented too. Only `S_Bed` (the counterparty code) gets a default, from a different
mechanism: `MyIni.readstring(Name, 'S_Bed', '')` (`ClearForm`, `:688`), written by the popup menu
item `ذخیره به عنوان پیش فرض` ("save as default", `N1Click`, `:131-134`).


---

[← 3. Document types (part b)](05-03-b-document-types.md) | [index](00-index.md) | [4. The invoice (Factor) lifecycle (part b) →](05-04-b-invoice-factor-lifecycle.md)
