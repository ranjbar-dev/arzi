_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 10. Shared dialog / frame catalogue

All of these are **pre-created at startup** (`arzi.dpr`) and shown with `ShowModal`.
In React they become controlled components driven by promise-returning hooks
(e.g. `const value = await prompts.number({...})`), not global singletons.

| Unit | Type | Purpose | Inputs | Outputs | Validation | React component |
|---|---|---|---|---|---|---|
| `WaitU.pas` | Modeless form `WaitF` | Progress / splash | `initForm(title, min, max)` (`:41-49`); `Gotonextposition` increments and pumps messages (`:34-39`) | none | — | `<ProgressOverlay title steps current />` — but the rebuild should not need a 47-step splash at all |
| `YesOrNo.pas` | Modal, `GetYes` function | Yes/No confirm | `GetYes(Ti, Ca1, Ca2 = '', Ca3 = '')` (`:21`) — up to 3 message lines into a `TRichEdit` | `Boolean` (`ModalResult = mrOk`) | — | `<ConfirmDialog />` returning `Promise<boolean>` |
| `SayMessage.pas` | Modal, `SayMSG` function | Information / acknowledge | `SayMSG(Ti, Ca1, Ca2 = '', Ca3 = '', keyCaption = 'تاييد ')` — the OK caption is overridable | `Boolean` | — | `<AlertDialog />` |
| `GetD.pas` | Modal, `GetDate` function | Prompt for **one** Jalali date | `GetDate(Ti, Ca, D)`; empty `D` defaults to `DM.Current_Date` (`:37-38`) | `String` (the original `D` on cancel); side-effect global `GetD_Ok` | OK enabled only while `Dm.isValidDate(N1.Text)` (`:46-49`) — inside the fiscal year | `<JalaliDatePicker />` returning `Promise<string \| null>` |
| `Get2D.pas` | Modal singleton `Get2D_F` | Prompt for a **date range** | `init(Ti, Ca1, Ca2, D1, D2)` (`:52-63`); read back with `GetResult(var D1, D2)` (`:41-49`) | two strings, or two `''` if cancelled | OK enabled only while both dates are `isValidDate` **and** `N1 < N2` (`:66-70`) | `<DateRangePicker />` |
| `GetN.pas` | Modal singleton, `GetNo` function | Prompt for **one** integer | `GetNo(Ti, Ca, I, EditLen = 8)` (`:28`) — `EditLen` caps the digit count | `Int64`; returns the input `I` unchanged on cancel | OK enabled only while `N1.IntValue > 0` (`:51-54`) — **zero is not accepted** | `<NumberPrompt />` |
| `GetN2N.pas` | Modal singleton, `Get2No` procedure | Prompt for an **integer range** | `Get2No(Ti, Ca1, Ca2, Ca3, var No1, No2)` (`:27`) | both set to **0** on cancel (`:50-51`) | OK enabled only while `No1 > 0`, `No2 > 0`, `No1 <= No2` (`:58-63`) | `<NumberRangePrompt />` |
| `GetS.pas` | Modal, `GetString` function | Prompt for a **string** | `GetString(Ti, Ca, L, var St, Align1Left2Right = 2)` (`:24`) — `L` sets `MaxLength`, the align flag picks `bdLeftToRight` (1) or `bdRightToLeft` (2) | `Boolean`; `St` mutated on OK | OK enabled only while `Trim(Edit1.Text)` is non-empty (`:60-63`) | `<TextPrompt dir maxLength />` — the manual pixel-width arithmetic at `GetS.pas:43-52` becomes CSS |
| `CodeNameU.pas` | Modal, `GetCodeName` function | Prompt for **a code + a name together** | `GetCodeName(Ti, Ca1, Ca2, L1, L2, var Co, var ST, Align = 2)` (`:26`) | `Boolean`; `Co` and `ST` mutated on OK | OK enabled only while name is non-empty **and** code > 0 (`:60-63`) | `<CodeNamePrompt />` |
| `FGetCodeU.pas` | **Frame** `TFGetCode` | The **account-code picker** — the single most reused composite in the product | `initCode(_K, _M, _T1, _T2)` (`:124-138`) | `IsComplete: Boolean` (`:67-96`) plus fields `SSN`, `Name`, `Make_L` (the dash-joined display code) | Four cascading `TEditInt` + `TDBLookupComboBox` pairs (Kol → Moein → Tafzil1 → Tafzil2). Each level's change handler clears all deeper levels and reparameterises the next query (`:145-234`). "Complete" means exactly one `Sarfasl` row matches. A browse button opens `Sarfasl_Select` (`:117-122`). | `<AccountPicker value={{ko,mo,ta1,ta2}} onChange />` — a cascading async-combobox with server-side lookup |
| `GetCodeStringU.pas` | Form `TGetCodeStringF` | **Empty shell.** Only geometry persistence (`:27-40`). No controls, no callers. | — | — | — | delete |
| `DateFrameU.pas` | Frame `TDateFrame` | **Empty shell.** One `TEdit` + one `TLabel`, no logic. | — | — | — | delete |
| `TanzimPU.pas` | Form `TTanzimP` | **Empty shell.** No members at all. | — | — | — | delete |
| `Ghabz.pas` | Frame `TGhabzF` | Weighbridge-ticket display; populated from `DM.B_SelectSerial` (`:62-70`) | — | read-only display | — | out of scope (weighbridge module) |
| `Get_Serial.pas` | Modal `GetSerialF` | Weighbridge-ticket lookup — see §7.4 | `init` clears both fields (`:35-43`) | ticket serial in `Tag`, `0` on cancel | Requires exactly one row **and** `SerialNoPsnBts = Jari` (`:59-72`) | out of scope |
| `GetPassword.pas` | Modal `GetPasswordF` | Numeric PIN — see §3.6 | `init(Caption)` (`:36-43`) | `Password: Int64` = `ElfHash(text)`; `0` on cancel | none | ⛔ do not port |

**Cross-cutting pattern to drop:** almost every dialog persists `Left`/`Top`/`Width`/`Height`
to the ini file in `FormActivate`/`FormClose`. In a web app this becomes either nothing
at all or a small `localStorage`-backed layout preference — it should not be a
server-side setting.

---


---

Prev: [9. `Utility.pas` function reference](08-09-utility-pas-function-reference.md) · Next: [11. Concurrency and multi-user behaviour](08-11-concurrency-and-multi-user-behaviour.md)
