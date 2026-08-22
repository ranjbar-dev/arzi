_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 8.4 Accounting posting for a pistachio purchase

`FactorPesteh_U.pas:223-229`. Exactly two lines, always, with `M_Id = 34`
(documented at `FactorPesteh_U.pas:1`, `// moein id 34 = kharid pesteh`):

| Side | `M_Code` | Account code | Amount | Persian meaning |
|---|---|---|---|---|
| **Debit** (`M_Bed`) | `@BedSSN` | `700-3-<NR_Kind>` | `@Mab` = `NR_Kol` | `کد خرید` — pistachio purchases, analysed by grade |
| **Credit** (`M_Bes`) | `@BesSSN` | `301-1-<NR_Jari>` | `@Mab` = `NR_Kol` | supplier current account (`جاری`) |

Common to both lines: `M_Coid=@Coid`, `M_Sanad=@Sanad`, `M_Date=@date` (= `NR_FDate`),
`M_Ted=0`, `Article=@Des`, `M_Tx=0`, `M_Ko=M_Mo=M_Ta1=M_Ta2=0`, `M_Id=34`,
**`M_Link=@FMFactor`**, `M_User=@user`, `M_Kind=1`, `M_Time=GetDate()`.

Then the four account-level columns are back-filled from the chart of accounts (`:228-229`):

```sql
Update moein Set M_Ko=S_Ko, M_Mo=S_Mo, M_Ta1=S_Ta1, M_Ta2=S_Ta2
  From Sarfasl Where M_Code=S_SSN and M_Sanad=@Sanad and M_Coid=@Coid
```

Note this `Update` is **scoped by voucher, not by `M_Id`** — harmless because a fresh voucher
number was just allocated, but it would rewrite any other line that happened to share the number.

**Five things about this posting that differ from every other inventory posting:**

1. **`M_Link` holds `FM_Factor` (the document *number*), not `FM_SSN` (the surrogate key).**
   `MakeSanadU` links by `FM_SSN` (`MakeSanadU.pas:84-86` deletes `M_Link=_SSN`, and
   `SodoorSanadU.pas:404` deletes `M_Link=@Link` where `_Link` is `FM_SSN`). This path links by
   `@FMFactor`. **The two conventions are incompatible.** This is the inventory-side
   confirmation of the data-layer finding that treasury's `DCheck.S_LinkSSN` holds a document
   *number*: link-by-number vs link-by-key is an unresolved, per-module choice throughout the
   application, not a single convention. It also means `SodoorSanadU`'s un-post could never find
   these lines even if `34` were in its `IN` list — it passes `FM_SSN` as `@Link`.
2. **The account prefixes `700-3-` and `301-1-` are string literals in the source**
   (`:146`, `:156`). The `Base.Kh1_Code` … `Kh8_Code` settings maintained by `Kharid_BU` — which
   exist precisely to configure the pistachio purchase and sale accounts (§8.5) — are **not
   consulted**. The settings screen and the live posting have no connection.
3. **The narration is built by string concatenation with no escaping** (`:171-174`). It embeds
   `NR_Name` (a supplier name, free text) and is passed through `QuotedStr` at `:191`, which
   doubles single quotes correctly — so this one is safe. But `_cala` at `:137`, `BedCode` and
   `BesCode` at `:185,187` are interpolated from field values; `_cala` is **not** quoted
   (`C_code='+ _cala`), relying on `NR_Kind` being an integer column. Every other value on this
   path is either `inttostr` or `QuotedStr`.
4. **`FM_Lock` is set to `2` at insert** (`:201`), skipping the `0` → `1` → `2` confirmation
   sequence that `SodoorSanadU` enforces for every other document kind. There is no unconfirmed
   or confirmed-but-unposted state for a pistachio receipt.
5. **There is no reversal.** `B_DeleteResidClick` (`FactorPesteh_U.pas:95-101`), the handler
   behind the button captioned `برگشت سند` ("reverse voucher",
   `FactorPesteh_U.dfm:137-150`), contains **only** the `Is_New_Sanad_Valid` guard and then
   `end;`. It does nothing. Combined with `SodoorSanadU.pas:202-204` having no branch for
   `FM_ID = 14` (it falls through to `' Not implemented yet. '`), **a posted pistachio purchase
   cannot be un-posted or corrected by any code path in this repository.** This is the
   mechanism behind the "Critical" row of §5.3.3 and it is confirmed here from the pistachio side.

**Narration format** (`:171-174`), reproduced because the rebuild must generate the same text for
continuity of the ledger:

```
'بابت خرید ' + NR_Vazn + ' کیلو پسته ' + NR_KindName +
' فاکتور ' + NR_Factor + ' جاری ' + NR_Jari + ' ' + NR_Name +
' مورخ ' + NR_FDate + ' قبض ' + NR_Ghabz + ' فی ' + NR_Phi + ' ریال'
```

"For the purchase of \<net kg\> kilos of \<grade\> pistachio, invoice \<n\>, current account
\<n\> \<supplier name\>, dated \<date\>, ticket \<n\>, at \<price\> rial." `varchar(200)`
(`:191`) — long supplier names plus a long grade name can truncate.

#### 8.4.1 Dead buttons on the live screen

`FactorPesteh_U.dfm` declares eight buttons; only three do anything:

| Control | `.dfm` line | Caption | State |
|---|---|---|---|
| `B_Exit` | `:42-55` | `خروج` ("exit") | Works |
| `sBitBtn3` | `:96-109` | `چاپ فاکتور` ("print invoice") | Works — `sBitBtn3Click` (`:287-298`), shows `RP_A5` if `NR_State ∈ {3,4}`, else `فاکتور صادر نشده است` ("the invoice has not been issued") |
| `B_NewResid` | `:152-166` | `صدور سند` ("issue voucher") | Works — §8.3 |
| `B_DeleteResid` | `:137-150` | `برگشت سند` ("reverse voucher") | **Dead — handler is empty** |
| `B_Ok` | `:57-69` | *(no caption)* | **Dead — no `OnClick`** |
| `sBitBtn1` | `:70-82` | *(no caption)* | **Dead — no `OnClick`** |
| `sBitBtn2` | `:83-95` | *(no caption)* | **Dead — no `OnClick`** |
| `sBitBtn4` | `:111-123` | *(no caption)* | **Dead — no `OnClick`** |
| `sBitBtn5` | `:124-136` | *(no caption)* | **Dead — no `OnClick`** |

The grid `G1` is `TrDBGrid_MS` with column widths persisted to the INI file per column index
(`FactorPesteh_U.pas:252-257`, `:264-267`) — a pattern repeated on every list screen.
`G1.ResetFilter` is called on close (`:263`) but no filter is ever *set*, matching the
"filter controls that are only ever reset" pattern found elsewhere.

---

### 8.5 `Kharid_BU` — pistachio base accounts (settings, unreachable)

Form caption `اطلاعات پایه خرید و فروش پسته` ("pistachio purchase and sale base data",
`Kharid_BU.dfm:5`). Eight account slots stored on the fiscal-year row `Base`, each as a pair
`Kh<n>_Code` (`Sarfasl.S_SSN`) + `Kh<n>_Desc` (denormalised account name).

| Slot | `Kharid_BU.dfm` | Persian | English | Mandatory? |
|---|---|---|---|---|
| 1 | `:29` | `کد خرید` | Purchase account | **Yes** — `Kharid_BU.pas:223-227`, `ورود يک کد براي خريد پسته اجباري است` ("entering a code for pistachio purchase is mandatory") |
| 2 | `:42` | `کد فروش` | Sale account | **Yes** — `:228-232`, `ورود يک کد براي فروش پسته اجباري است` ("entering a code for pistachio sale is mandatory") |
| 3 | `:55` | `اسناد پرداختني` | Notes payable | No |
| 4 | `:68` | `اسناد دریافتنی` | Notes receivable | No |
| 5 | `:81` | `هزینه 1` | Expense 1 | No |
| 6 | `:94` | `هزینه 2` | Expense 2 | No |
| 7 | `:107` | `هزینه 3` | Expense 3 | No |
| 8 | `:120` | `هزینه 4` | Expense 4 | No |

Loading is eight copy-pasted 18-line blocks (`Kharid_BU.pas:92-210`), each running
`Dm.Sarfasl_seekSSN` with `@SSN` and reading `M_L` (the formatted account code, left-aligned
variant) and `S_Name`. Saving is `Base.Edit` / sixteen `FieldByName` assignments / `Base.Post`
(`:234-255`).

**Defects:**

- `Kharid_BU.pas:267` — `Kh4_DelClick` clears `Kh4_Code.Text` and `Kh4_Desc.Text` but resets
  **`Kh3_Code.Tag`** instead of `Kh4_Code.Tag`. Clicking "clear" on slot 4 blanks slot 4's
  display while leaving slot 4's stored id intact and silently clearing slot 3's id. On save,
  slot 4 keeps its old account and slot 3 loses its account — and slot 3's mandatory-ness is not
  checked. Copy-paste error; slots 5–8 are correct.
- Slots 1 and 2 are mandatory but **`Kh1_Del` / `Kh2_Del` do not exist** — there is no clear
  button for the two mandatory slots (only `Kh3_Del` … `Kh8_Del`, `Kharid_BU.pas:45-50`).
- Picking uses `Sarfasl_Select.init` and reads `M_R` (`:305`, right-aligned variant) while
  loading reads `M_L` (`:101`). The same field is displayed in two different formats depending
  on whether it was loaded or just picked.
- **None of these eight settings is read by any other unit.** `grep` for `Kh1_Code` … `Kh8_Code`
  returns only `Kharid_BU.pas`. The live pistachio posting hard-codes its accounts (§8.4 note 2).

---

### 8.6 The weighbridge/lab front-end in this repository is entirely dead

Three units exist for driving the weighbridge directly:

| Unit | Purpose | Status |
|---|---|---|
| `Get_Serial.pas` | Prompt for ticket number + supplier current-account number, validate against `B_SelectSerial` | Logic intact (`:51-76`), but only ever called from `Lab.pas:46` |
| `Ghabz.pas` | A `TFrame` displaying one weighbridge ticket (supplier, grade, serial, date, in-weight, out-weight, net weight, status) | `LoadForm` (`:57-101`) works; the second half (`:87-99`, `Sp_NRSelectGhabz`) is commented out |
| `Lab.pas` | Assign a blind code (`رمز`) to a lot and print it | **Every meaningful line is commented out** |

`Lab.pas` detail:

- `B_RamzClick` (`:71-112`): the only live statement is a guard
  `if _Serial < 95020 then MessageDlg('فقط تحويل پسته سال 1402 به بعد', …)` — "only pistachio
  deliveries from year 1402 onward" — a hard-coded ticket-number watermark. Everything after it
  (`MakeRamz`, `SP_SetRamz`, setting `StatusBts := 6`, revealing the code, enabling print) is
  inside a `{ … }` comment block (`:83-111`). **Clicking the button either shows the error or
  does nothing at all.**
- `B_ChapClick` (`:114-120`): body is four commented lines. Dead.
- `B_Chap2Click` (`:122-132`): live code is `GetPasswordF.init; if GetPasswordF.Password <>
  234384 then exit;` — a **hard-coded numeric password in the source** — followed by four
  commented lines. Even with the right password it does nothing.
  (Security cross-reference: `docs/08-platform-and-security.md`.)
- `TLabF` is **never instantiated or referenced** anywhere: `Mainu.pas` does not include `Lab` in
  its `uses` clauses (`Mainu.pas:279-291`) and `LabF` appears in no other unit.

The stored procedures these units reference — `B_SelectSerial` (`Dmu.dfm:1143-1160`, parameter
`@GhabzNo`, returning `SerialNoPsnBts`, `FullName`, `KindName`, `SerialnoBts`, `InDate`,
`InWeightBts`, `OutWeightBts`, `NetWeightBts`, `StatusBts`), `SP_SetRamz` (`@GhabzNo`, `@Ramz`,
`@User`, returning `Error` / `Message`) and `Sp_NRSelectGhabz` (`@GhabzNo`) — live in
`Rppc_Solution` and belong to the weighbridge application. Only `B_SelectSerial` is declared in
`Dmu.dfm`; the other two are referenced against a `Dmf` data module that **does not exist in this
project** (`Lab.pas:88-92,102-104`, `Ghabz.pas:87-90`) — leftovers from the weighbridge
application's own source tree, which is further proof the code was never compiled in this state.

`KharidPeste_List` (`Dmu.dfm:1114-1124`) is `Select * From NewRamz` on the **main** connection —
where `NewRamz` does not exist. It is declared at `Dmu.pas:70` and referenced nowhere. Dead, and
would throw if ever opened.

---

### 8.7 Summary for the rebuild

What must be ported:

1. **The lot/grade model.** `pistachio_grades` (id, name, active) as a first-class table, with an
   explicit FK from `items` and an explicit mapping to the purchase account — not the current
   triple-duty integer (§8.1.1).
2. **The deduction calculation of §8.2**, exactly: percentages off gross (not compounded), other
   deductions in kg, net floored at zero, total = round(net × price). Decide the rounding mode
   explicitly (§15).
3. **The receipt-from-weighbridge flow** of §8.3, with its eight preconditions and its two-line
   posting — but with `M_Link` unified onto the surrogate key, a real reversal, and one
   transaction covering the header, the line, the source-state update, the voucher lines *and*
   the voucher header.
4. **The narration text** of §8.4, for ledger continuity.

What must not be ported:

- The `Kharid_U` / `Kharid_BU` / `Lab` / `Ghabz` / `Get_Serial` screens as they stand. `Kharid_U`
  and `PestehD_U` supply the *formula*, which becomes a service function; the screens themselves
  are unreachable and unfinished.
- The `Kh1_Code` … `Kh8_Code` settings, unless the rebuild actually makes the posting accounts
  configurable — in which case they replace the hard-coded `700-3-` / `301-1-` prefixes, and
  that is a behaviour change requiring approval (§15).

What is not recoverable from this repository and must be obtained elsewhere:

- The weighbridge application, which owns `NewRamz`, `NR_Vazn1`…`NR_Vazn5`, `NR_P3`…`NR_P12`,
  `StatusBts`, `B_SelectSerial`, `SP_SetRamz` and `Sp_NRSelectGhabz`. If the rebuild absorbs it,
  its rules must be re-specified from scratch (§14).
- The row contents of `Kinds`. The seven-value list in §8.1 comes from a source comment.


---

[← 8. The Pesteh (pistachio) specialisation (part b)](05-08-b-pesteh-pistachio-specialisation.md) | [index](00-index.md) | [9. Settlement (Tasfieh) (part a) →](05-09-a-settlement-tasfieh.md)
