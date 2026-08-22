_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 3. Authentication

### 3.1 The `Password` table

Bound as `DM.Password : TADOTable`, `TableName = 'PassWord'` (`Dmu.dfm:597-603`,
declared `Dmu.pas:39`). Columns, inferred from every read/write site:

| Column | Type | Meaning | Evidence |
|---|---|---|---|
| `UserCode` | int, PK, manually assigned | User id | `Admin.pas:175-177`, `GetPassu.pas:72` |
| `UserName` | varchar(20) | Display name and **login identifier** | `Admin.pas:178`, `Admin.dfm:536`, max length from `GetString(...,20,...)` at `Admin.pas:170` |
| `Password` | varchar(20) | **Plaintext password** | `Admin.pas:289`, `ChangePasswordU.pas:67`, `GetPassu.pas:84`; column is `Visible = False` in the admin grid (`Admin.dfm:542-544`) but the value is read straight out |
| `Enabled` | int (0/1) | Account active flag | `Admin.pas:179`, `Admin.pas:262-264`, `GetPassu.pas:79` |
| `Supervisor` | int (0/1) | **Super-admin flag** — bypasses the whole permission matrix | `Admin.pas:180`, `Mainu.pas:957-961`, `GetPassu.pas:97`, `Admin.dfm:554-557` |

There is **no** `LastLogin`, `FailedAttempts`, `LockedUntil`, `PasswordChangedAt`,
`Email`, `CreatedAt`, `CreatedBy`, or any tenant/company scoping. A user is global
across all companies and fiscal years.

New user ids are allocated as `max(UserCode) + 1` read client-side after `Last`
(`Admin.pas:172-177`) — ⛔ racy, and it takes the value of whatever row happens to be
last in the cursor's sort order.

### 3.2 How credentials are stored

**Plaintext. No hash, no salt, no encryption, no KDF.**

- Written verbatim by the admin: `dm.Password.FieldByName('password').asstring := st;`
  (`Admin.pas:289`) where `st` came from a plain `GetString` prompt with no masking
  (`Admin.pas:287`).
- Written verbatim on self-service change: `FieldByName('Password').AsString := Trim(R2.Text)`
  (`ChangePasswordU.pas:67`).
- Read verbatim on login: `S1 := Q2.FieldValues['Password'];` (`GetPassu.pas:84`).

⛔ **DO NOT PORT.** Not the storage, not the comparison, not the recovery model.

### 3.3 Login flow (`GetPassu.pas`)

Form `GetPass`, caption `فرم ورود به سيستم` — "System Login Form"
(`GetPassu.dfm:5`). Three inputs:

| Control | Persian label | English | Binding |
|---|---|---|---|
| `CO_ID_IN` (`TDBLookupComboBox`) | `شرکت` | Company / fiscal year | `DM.Base_Q` — `Select *, LTrim(RTrim(Co_Name)) + ' = ' + LTrim(RTrim(Co_Sub)) As CO_DESC From Base Order By Co_ID` (`Dmu.dfm:362-375`); key `CO_ID`, list `CO_DESC` (`GetPassu.dfm:128-138`) |
| `ID` (`TDBLookupComboBox`) | `نام کاربر` | User name | `Q1`: `Select * From PassWord Where Enabled = 1 Order By UserCode` (`GetPassu.dfm:139-149`); key `UserCode`, list `UserName` |
| `Pass` (`TEdit`) | `رمز کاربر` | Password | `MaxLength = 20`, `PasswordChar = *` (`GetPassu.dfm:71-79`) |

**⛔ The user list is a dropdown of every enabled account.** Usernames are enumerable by
anyone who can open the app. Do not port.

`init` (`GetPassu.pas:45-60`): reopen `Base_Q` and `Q1`, clear password, preselect
company from `Base_Q`'s current row, focus the user combo, `ShowModal`.

`FormActivate` (`GetPassu.pas:118-127`) then overrides that: it restores window geometry
**and the previously used `ID` and `COID`** from the ini file, and moves focus to the
password box. So the last user to log in on that workstation is pre-selected by name.

`BitBtn1Click` — the actual authentication (`GetPassu.pas:62-108`):

1. `if CO_ID_IN.KeyValue < 1` → refocus company, exit (`:65-69`).
2. Build SQL by **string concatenation**:
   `'Select * From PassWord Where UserCode = ' + inttostr(Id.KeyValue)` (`:72`).
   (Here the value comes from a lookup key so it is an integer, but the *pattern* is
   used with user-typed strings elsewhere in the codebase — see §11.)
3. `RecordCount = 0` → "user code is wrong" (`:74-78`).
4. `Enabled = 0` → "user is disabled" (`:79-83`).
5. Compare: `UpperCase(Trim(stored))` vs `UpperCase(Trim(typed))`, then also compare
   lengths (`:84-93`).
   ⛔ **Passwords are case-insensitive and whitespace-insensitive.** `" secret "` and
   `"SECRET"` are the same password. The extra length comparison is redundant.
6. On success set the *entire* session in globals on the data module (`:94-97`):
   `DM.UserId`, `DM.UserName`, `DM.CO_ID`, `DM.Admin := (Supervisor = 1)`.
7. Reopen `Base`, `Locate` the chosen company, cache `DM.RegName` (company name) and
   `DM.RegSal` (fiscal-year label) (`:99-103`).
8. Show the splash and close (`:106-107`).

`BitBtn2Click` (Cancel) → `Application.Terminate; halt;` (`GetPassu.pas:110-116`).

`FormClose` persists geometry **plus the chosen user id and company id** to the ini file
(`GetPassu.pas:129-137`) — this is what `arzi.local.ini` `[GetPass] ID=2 COID=1399`
records.

### 3.4 Failed-attempt handling

**There is none.** No counter, no exponential backoff, no lockout, no audit entry, no
rate limit, no CAPTCHA. `GetPassu.pas:88-93` simply clears the box and returns focus.
The dialog can be retried indefinitely. Combined with the enumerable username dropdown
and case-insensitive plaintext comparison, this is trivially brute-forceable by anyone
at the keyboard.

### 3.5 Password change (`ChangePasswordU.pas`)

Form `ChangePassword`, opened from the ribbon `تغییر رمز` button
(`Mainu.pas:599-602`). Three edits: `R1` = current, `R2` = new, `R3` = confirm.

`init` (`:89-101`): refuses if `Dm.userId = 0` with `ابتدا وارد شوید` ("log in first").
Clears all three fields, focuses `R1`, `ShowModal`.

`BOKClick` (`:43-71`) validations, in order:

| # | Check | Failure message | Line |
|---|---|---|---|
| 1 | stored `password` **exactly** equals `R1.Text` (no trim, no case fold — **stricter than login**) | `رمز فعلی را وارد کنید` "enter the current password" | `:48-53` |
| 2 | `Length(Trim(R2.Text)) > 0` | `رمز جدید را وارد کنید` "enter the new password" | `:54-59` |
| 3 | `Trim(R2.Text) = Trim(R3.Text)` | `کنترل رمز را صحیح وارد کنید` "confirm the password correctly" | `:60-65` |

On success: `Password := Trim(R2.Text)`, `Post`, message `رمز جدید جایگزین شد`
("new password replaced"), close (`:66-70`).

**Absent:** minimum length, complexity, history, expiry, reuse prevention, re-auth,
notification, and any bound on length beyond the DB column width. Note the asymmetry:
you can *log in* with a case-mismatched password, but you cannot *change* it unless you
type the current one with exact case — a genuine usability bug.

### 3.6 Other password-shaped things in the codebase (not user auth)

| Thing | What it is | Location |
|---|---|---|
| `GetPasswordF` | A numeric-PIN dialog that hashes the typed text with `SysInfo.ElfHash` and exposes it as `Password: int64`. Its only caller compares against the **hard-coded constant `234384`** (which the comment says is `6060`). | `GetPassword.pas:36-54`, caller `Lab.pas:124-127` |
| `TestF.B_Calc` gate | The licence *generator* is unlocked by typing a password whose `Util.Encrypt` output must equal the literal `'d+B6Y52L6r0dU2UPhjhf'`. A source comment names the plaintext. | `testmainU.pas:79-100` |
| Backup archive password | `Db.Password := 'Mohsen' + inttostr(68411) + inttostr(211)` — a hard-coded literal on the Absolute Database backup file. | `Backup_U.pas:141` |

⛔ All three are hard-coded secrets committed to source. None may be ported.

---


---

Prev: [2. Application startup sequence](08-02-application-startup-sequence.md) · Next: [4. Authorization](08-04-authorization.md)
