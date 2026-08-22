_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 9. `Utility.pas` function reference

`Utility.pas` declares a single class `TUtil` with a global singleton `Util`
(`Utility.pas:11-71`). **`Util` is never constructed anywhere in the codebase** — every
call goes through an uninitialised class reference and works only because none of the
methods touch instance state. ⛔ Do not reproduce this pattern; in Rust these are free
functions in modules.

Note also that several routines are **triplicated**: `ElfHash` exists in
`Utility.pas:396`, `LockUnit.pas:62` and `Dmu.pas` (declared `:124`); the
encryption suite exists in `Utility.pas:868-1016` and `INI.pas:43-170`;
`inttostr3` in `Utility.pas:1334` and `Dmu.pas:859`; `N23`/`Str2String` in
`Utility.pas:446-515` and `Dmu.pas:569-...`; `IsInteger` in `Utility.pas:81` and
`Dmu.pas:902`; the Shaba/card validators in `Utility.pas:90-126` and `Dmu.pas:196-240`.
**The duplicates are not always identical** — see the card-number note below. Consolidate
into one implementation each, and pick the variant the production code actually calls.

### 9.1 Jalali (Persian) calendar — ⚠️ must be byte-identical

Two *different, incompatible* Gregorian→Jalali conversions exist. Both are in
production use. The rebuild must reproduce each where it is used, or (preferably) get
sign-off to unify them.

#### `TUtil.DecodedateF(ADate: TDateTime; var AYear, AMonth, ADay: Word)` — `Utility.pas:413-431`

Custom, **arithmetic-approximation** algorithm; not a correct Jalali conversion.

```
AYear := 1279; AMonth := 1; ADay := 1;
R := Round(Int(Real(ADate))) - 80;          // Delphi day number, epoch 1899-12-30
for i := 1 to 1000 do begin
  if i mod 4 = 0 then Dec(R, 366) else Dec(R, 365);
  Inc(AYear);
  if R < 365 then Break;
end;
for i := 1 to 6 do if R > 31 then begin Inc(AMonth); Dec(R,31) end else begin ADay := R; Exit end;
for i := 1 to 6 do if R > 30 then begin Inc(AMonth); Dec(R,30) end else begin ADay := R; Exit end;
```

Edge cases and defects (reproduce exactly if this path is kept):

- Leap years are `i mod 4 = 0` where `i` is a **loop counter, not a year** — so the leap
  pattern is offset from any real calendar and drifts.
- The 33-year Jalali leap cycle is not modelled at all.
- Esfand 30 (leap day) is unreachable: after six 31-day and six 30-day months the
  routine falls off the end returning `AMonth = 13` with `ADay` unset.
- Dates before the epoch produce `AYear = 1280, AMonth = 1, ADay = <garbage>`.
- No validation of the input.

#### `TUtil.FarsiDate(ADate: TDateTime): String` — `Utility.pas:435-442`

Formats `DecodedateF` output as **`YY/MM/DD`** — a **2-digit year**
(`inttostr(AYear mod 100)`), zero-padded month and day built with `Char(x div 10 + 48)`.
So `1403/05/12` renders as `03/05/12`. Any consumer must re-prefix `13`.

#### `TDM.MiladiToShamsi(Date1: TDate): String` — `Dmu.pas:362-...`

A **different and much more nearly correct** algorithm: day-of-year accumulation over
`count_days = (31,28,31,30,31,30,31,31,30,31,30,31)`, `IsLeapYear(Year)` bump for
March onward, the 79-day pivot, `Year - 622` / `Year - 621`, then 31-day months for the
first six and 30-day for the rest. Returns a full 4-digit Jalali date.

**Rust equivalent:** use a maintained crate — `ptime` (Persian time, `strftime`-style) or
`jalali-date`, over `chrono`/`time`. But ⚠️ **do not swap the algorithm silently**: any
stored `varchar(10)` Jalali date in the existing database was produced by one of the two
routines above. Before migrating, run a full-table comparison of stored dates against
crate output and get the discrepancies signed off (see §13).

#### Jalali validators

| Function | Signature | Behaviour | Rust |
|---|---|---|---|
| `TUtil.IsFarsiDate` | `(D: String): Boolean` — `Utility.pas:526-541` | Expects **`YY/MM/DD`**, length exactly 8, `/` at positions 3 and 6. Strips non-digits, needs ≥6. Valid iff `20 ≤ YY ≤ 99`, `1 ≤ MM ≤ 12`, `1 ≤ DD ≤ 31`, and **not** (`DD > 30` and `MM > 6`). Does not know about Esfand's 29/30 days. | Hand-port; the rule set is the spec |
| `TDM.IsDate` | `(D1: String): Boolean` — `Dmu.pas:883-900` | Accepts length **8 or 10**; if 8, prefixes `'13'`. Requires `/` at 5 and 8. Valid iff `1 ≤ MM ≤ 12`, `1 ≤ DD ≤ 31`, not (`MM > 6` and `DD > 30`), and **`1300 ≤ YYYY ≤ 1420`**. | Hand-port |
| `TDM.isValidDate` | `(D1: String): Boolean` — `Dmu.pas:911-919` | `IsDate` **and** string-compares within `[Base].FromDate … ToDate` of the current fiscal year. ⚠️ Compares as *strings* — correct only because the format is zero-padded `YYYY/MM/DD`. | Hand-port; compare as a typed date |
| `TUtil.Mah2str` | `(Mah: Integer): String` — `Utility.pas:1347-1364` | Jalali month name, **space-padded on both sides**: `' فروردين '`, `' ارديبهشت '`, `' خرداد '`, `' تير '`, `' مرداد '`, `' شهريور '`, `' مهر '`, `' آبان '`, `' آذر '`, `' دي '`, `' بهمن '`, `' اسفند '`. Out-of-range returns **uninitialised** `S` (`S` is never assigned `''`). | Static array; return `""` for out-of-range and note the behaviour change |

### 9.2 Number → Persian words — ⚠️ must be byte-identical

Used on printed cheques and invoices, so output is legally significant.

#### `TUtil.N23(N: Integer): String` — `Utility.pas:446-479`

Converts 0–999 to Persian words. Three constant arrays (`Utility.pas:447-457`):

- `n2s1[1..19]` = `' يک' ' دو' ' سه' 'چهار' ' پنج' ' شش' ' هفت' ' هشت' ' نه' ' ده'
  ' يازده' ' دوازده' ' سيزده' ' چهارده' ' پانزده' ' شانزده' ' هفده' ' هجده' ' نوزده'`
- `n2s2[1..9]` = `' ده' ' بيست' ' سي' ' چهل' ' پنجاه' ' شصت' ' هفتاد' ' هشتاد' ' نود'`
- `n2s3[1..9]` = `' يکصد' ' دويست' ' سيصد' ' چهارصد' ' پانصد' ' ششصد' ' هفتصد' ' هشتصد' ' نهصد'`

⚠️ **Reproduce these strings byte-for-byte**, including:

- Every entry except `'چهار'` (index 4) begins with a **leading ASCII space**. `'چهار'`
  does not. This asymmetry is load-bearing for the rendered output.
- The Arabic letter **`ي` (U+064A)**, not the Persian `ی` (U+06CC), throughout.
- `' سي'` (30) uses `ي`; `' يکصد'` for 100.

Algorithm: hundreds via `n2s3`; if the remainder > 19 use `n2s2[N div 10]` joined with
`' و'` and then `n2s1[N mod 10]` joined with `' و'`; otherwise `n2s1[N]` joined with
`' و'`. Joins are added only when the accumulator is non-empty. `N = 0` → `''`.
Negative or `N ≥ 1000` will index out of range.

#### `TUtil.Str2String(S: String): String` — `Utility.pas:483-515`

Groups the digit string into 3-digit chunks from the right and prepends scale words,
building the result right-to-left:

| Chunk | Scale word |
|---|---|
| 1st (units) | *(none)* |
| 2nd | `' هزار'` (thousand) |
| 3rd | `' ميليون'` (million) |
| 4th | `' ميليارد'` (billion) |
| 5th | **`' تريليارد'`** (trillion) |

Separator between groups is `' و'`, inserted **before** the accumulated tail when the
current chunk is non-zero and the tail is non-empty (`Utility.pas:496`, `:501`, `:506`,
`:511`).

⚠️ Edge cases to preserve: chunks are extracted with `StrToInt('0' + Copy(S, Length(S)-2, 3))`
— the `'0'` prefix makes an empty slice parse as 0. `S = '0'` → `''` (empty, not "صفر").
**Only five groups are handled — anything above 999 trillion is silently truncated.**
Non-digit input raises `EConvertError`. `TDM` carries a near-identical copy
(`Dmu.pas:604-...`).

#### `TUtil.No2String(N: Int64): String` — `Utility.pas:519-522`

`Str2String(IntToStr(N))`. Negative numbers hit the `'-'` on `StrToInt` and raise.

**Rust equivalent:** hand-port these three, do **not** use a crate. Back them with a
golden-file test that sweeps 0…999 exhaustively plus every power-of-ten boundary and
the existing production values, and asserts byte equality against the Delphi output.

### 9.3 Number formatting

| Function | Signature | Behaviour | Rust |
|---|---|---|---|
| `TUtil.inttostr3` | `(I: Int64): String` — `Utility.pas:1334-1343` | Thousands separators, but by a **positional hack**, not a loop: inserts `,` before the last 3, then before the last 7, then 11, then 15. Correct for values up to 10^18. `0` → `'0'`. Negatives get a comma inserted into the sign group for long values. | `format!` with manual grouping, or `num-format`. **Verify negatives against the original.** |
| `TDM.inttostr3` | `(N: Int64; DefaluZero: String = '0'): String` — `Dmu.pas:859-867` | Same, but stops at the 11-digit step (**no 15**) and returns `DefaluZero` when `N = 0`, letting callers render blank instead of `0`. | Same function with an `Option`/default |
| `TDM.Adj_Cent` | `(var St: String)` — `Dmu.pas:688-699` | Normalises a decimal string to exactly two places by **appending**: no `.` → append `'.00'`; `.` at `Length-1` → append `'0'`; `.` at `Length` → append `'00'`. It **never truncates**, so `'1.234'` stays `'1.234'`. | `rust_decimal::Decimal` with `round_dp(2)` — ⚠️ that is *different* behaviour; confirm before changing |
| `TDM.Number_Extract` | `(S: String): String` — `Dmu.pas:1264-1271` | Keeps only `0-9` and `.`. Multiple dots survive. | `s.chars().filter(...)` |

### 9.4 Legacy Persian encoding conversion (DOS ↔ Windows)

Needed only for migrating pre-Unicode data. Both routines were used in the one-shot
migrations on the main form (`Mainu.pas:730`, `:748`, `:790`, `:836`).

| Function | Signature | Behaviour |
|---|---|---|
| `TUtil.Win2Dos` | `(Str: String): String` — `Utility.pas:1181-1263` | Windows-1256 → DOS/Iranian System code page. Builds a `[0..255, 0..3]` glyph table where the second index is a **contextual form** computed by the nested `MakeOrder(C, L, N)` (`:1188-1196`): `+1` if the current char can join to the previous, `+2` if it can join to the next → 0=isolated, 1=final, 2=initial, 3=medial. Output is assembled **right-to-left** (`S1 := glyph + S2 + S1`, `:1253`) with ASCII runs buffered in `S2`. A post-pass fuses `#145 #243` (alef + lam) into the lam-alef ligature `#242` (`:1258-1261`). |
| `TUtil.Dos2Win` | `(Str: String): String` — `Utility.pas:1267-1315` | Inverse, via a flat `[0..255]` table of 1–2 char strings. Swaps `[`/`]` and `(`/`)` (mirroring for RTL). Maps DOS digits `#128..#137` → `'0'..'9'`. Maps all four contextual forms of `ع`/`غ` back to one letter. `#242` → `'لا'`. **Contextual final forms carry a trailing space** (e.g. `#146 → 'ب '`), which is why the result is passed through `Remove2SP`. ⚠️ It maps to **`ك` (Arabic kaf, U+0643)** and **`ي` (Arabic yeh, U+064A)**, *not* the Persian `ک`/`ی` — so migrated text needs a normalisation pass. |
| `TUtil.Remove2Sp` | `(inString: String): String` — `Utility.pas:1320-1330` | Collapses every run of consecutive spaces to one, by repeated `Pos('  ', …)`. O(n²). | `s.split_whitespace().join(" ")` |

**Rust equivalent:** `encoding_rs` for the code-page tables, plus a hand-ported
contextual-form pass. Only needed for a one-time migration; do not put it in the
runtime.

### 9.5 Validation

| Function | Signature | Behaviour | Rust |
|---|---|---|---|
| `TUtil.IsInteger` | `(S: String): Boolean` — `Utility.pas:81-88` | All chars in `'0'..'9'`. ⚠️ **`''` returns `True`** (empty loop). Same bug in `TDM.IsInteger` (`Dmu.pas:902-909`). | `!s.is_empty() && s.chars().all(char::is_ascii_digit)` — **behaviour change, flag it** |
| `TUtil.IS_ShabaNo` | `(S: String): Boolean` — `Utility.pas:90-109` | IBAN mod-97 for Iranian IBANs. Uppercases, strips **all spaces**, requires `IR` prefix and length 26, rearranges to `Copy(S,5,24) + '1827' + Copy(S,3,2)`, checks digits-only, then mod-97 == 1. | Hand-port or `iban_validate` |
| `TDM.IsValidShaba` | `(S: String): Boolean` — `Dmu.pas:196-214` | Same, but **does not strip spaces** and uses `Copy(S,5,22)`. Functionally equivalent given length 26. | — |
| `TUtil.Is_CardNo` | `(S: String): Boolean` — `Utility.pas:111-126` | Luhn, length 16. Doubles **odd** positions (1-indexed), subtracts 9 if > 9, sum mod 10 == 0. Requires digits-only. | `luhn` crate |
| `TDM.IsValidKart` | `(S: String): Boolean` — `Dmu.pas:216-240` | ⚠️ **Different.** Also requires the first digit to be `4`, `5` or `6`; doubles odd positions as `2*k` or `2*k-9`; adds even positions raw; **does not** check digits-only, so `StrToInt` will raise on a non-digit. | — |
| `TUtil.Bank_CardNo` | `(S: String): String` — `Utility.pas:128-173` | Maps the 6-digit BIN to an Iranian bank name. **43 entries**; several banks have multiple BINs (Parsian ×3, Pasargad ×2, Karafarin ×2, Keshavarzi ×2, Mellat ×2, Tejarat, Saderat, …). Returns `''` if unknown or `Length(S) < 6`. | `phf::Map<&str, &str>` — copy the table verbatim from `Utility.pas:133-172` |

### 9.6 Cryptography / hashing — ⛔ none of this is fit for purpose

| Function | Signature | Behaviour | Rust |
|---|---|---|---|
| `TUtil.ElfHash` | `(const Value: String): Integer` — `Utility.pas:396-409` | The classic ELF/PJW 32-bit string hash: `h = (h shl 4) + Ord(c)`; `x = h and $F0000000`; if `x <> 0` then `h = h xor (x shr 24)`; `h = h and (not x)`. Duplicated verbatim in `LockUnit.pas:62-75`. Used for licensing (§7) and a PIN check (`GetPassword.pas:52`). | Hand-port for compatibility only; **never** for security |
| `TUtil.Encrypt` / `Decrypt` | `(const S: AnsiString; Key: Word = 53269): AnsiString` — `Utility.pas:1005-1016` | `PostProcess(InternalEncrypt(S, Key))` and inverse. `InternalEncrypt` (`:984-1003`) is the Borland stream cipher: `c := c xor (Seed shr 8); Seed := (c + Seed) * 52845 + 22719`. `PostProcess`/`Encode` (`:948-983`) is a 3-byte→4-char Base64-like packing over `A-Za-z0-9+/` with **no padding**. Byte-identical twin in `INI.pas:43-170`. | Only to *read* legacy `CS2` during migration. Replace with a secrets manager / AES-GCM. |
| `TUtil.EncryptText` / `DecryptText` | `(Text: String; Key1..Key4: Integer): String` — `Utility.pas:1035-1180` | A 2×2-matrix Hill-style cipher over hex digits. Keys must be 1…120 and `Key1*Key4 ≠ Key2*Key3`, else returns `''` (`:1043`, comment `:1020-1034`). **No call sites** — dead code. | Delete |
| `TUtil.GetCheckSum` | `(FileName: String): DWORD` — `Utility.pas:203-233` | Windows `MapFileAndCheckSumA` (imagehlp). | `sha2` |

### 9.7 Hardware / OS fingerprint (licence inputs — see §7)

Present in both `Utility.pas` and `LockUnit.pas`. Only the `LockUnit` copies are wired
to the licence check (`testmainU.pas:124-152`).

| Function | Location | Returns |
|---|---|---|
| `GetHardDiskVolumeSerial(DriveLetter)` | `Utility.pas:545-559` | Volume serial as Int64 |
| `GetHardDiskVolumeLabel(DriveLetter)` | `Utility.pas:560-575` | Volume label |
| `GetHardDiskPartitionType(DriveLetter)` | `Utility.pas:324-341` | File-system name |
| `TSysInfo.GetHDSerialNumber` | `LockUnit.pas:26-37` | `GetVolumeInformation('C:\')` serial, as a string. **Hard-coded to `C:\`.** |
| `GetCpuID` | `Utility.pas:234-255`, `LockUnit.pas:78-96` | Inline `CPUID` leaf 1 → `EAX-00000000-ECX-EDX` hex. Wrapped in `try/except` with the fallback literal `'0000-D342-F921-M068'`. |
| `GetCPUName` | `Utility.pas:307-323`, `LockUnit.pas:133-244` | `CPUID` leaves `$80000002..4` → brand string. `LockUnit`'s version indexes `s2[Length(s2)]` without a length guard. |
| `GetCPUIdentifier`, `GetCpuSpeed` | `Utility.pas:289-306`, `:272-288` | Registry `HARDWARE\DESCRIPTION\System\CentralProcessor\0` |
| `GetSystemBiosDate` / `GetBiosDate` | `Utility.pas:383-395`, `LockUnit.pas:100-109` | `HKLM\HARDWARE\DESCRIPTION\SYSTEM` → `SystemBiosDate` |
| `GetVideoBiosDate` / `GetVideoDate` | `Utility.pas:370-382`, `LockUnit.pas:111-120` | same key → `VideoBiosDate` |
| `GetSystemName` | `LockUnit.pas:122-131` | `…\SYSTEM\Bios` → `SystemProductName` |

**Rust equivalent:** none needed. Node-locking has no place in the hosted rebuild.

### 9.8 Network / machine identity

| Function | Location | Behaviour | Rust |
|---|---|---|---|
| `GetComputerName` | declared `Utility.pas:37` | Win32 `GetComputerName` | `hostname` crate |
| `Get_HostName` | `Utility.pas:599-620` | Winsock `gethostname` | `hostname` |
| `GetDomainName` | `Utility.pas:781-789` | `GetNetParam(…)` via WNet | — |
| `Get_IPAddress` | `Utility.pas:621-691` | First local IPv4 via Winsock | `local-ip-address` |
| `Get_MACAddress(AdapterNo = 0)` | `Utility.pas:692-710` | NetBIOS (`NB30`) adapter status | `mac_address` |
| `HostToIP(sHost; var sIP)` | `Utility.pas:790-823` | `gethostbyname` | `std::net::ToSocketAddrs` |
| `IsWordInstalled` | `Utility.pas:256-271` | Probes the `Word.Application` COM class | drop |
| `LogOffWindows` | `Utility.pas:592-598` | `ExitWindowsEx` | drop |
| `UpTime` | `Utility.pas:175-202` | `GetTickCount` formatted `d:hh:mm:ss` | `sysinfo` |

### 9.9 Files

| Function | Location | Behaviour | Rust |
|---|---|---|---|
| `GetFileSize(FName)` | `Utility.pas:576-591` | `FindFirst`, combines high/low words | `std::fs::metadata().len()` |
| `GetFileLastAccessTime(sFileName)` | `Utility.pas:840-867` | `FindFirst` + `FileTimeToDateTime` | `metadata().accessed()` |
| `DeleteFileWithUndo(sFileName)` | `Utility.pas:824-839` | `SHFileOperation` with `FOF_ALLOWUNDO` → Recycle Bin | `trash` crate |
| `Get_CurrentDirectory` | `Utility.pas:1367-1373` | `GetDir(0, …)` | `std::env::current_dir` |

### 9.10 Misc

| Function | Location | Behaviour | Rust |
|---|---|---|---|
| `IntToRoman(num: Cardinal)` | `Utility.pas:711-780` | Arabic → Roman numerals. **No call sites** — dead. | delete |
| `GetNetParam(AParam)` | `Utility.pas` (private) | WNet helper for `GetDomainName` | — |
| `TDM.Add_String(St1, St2)` | declared `Dmu.pas:119` | Concatenation helper | `format!` |
| `TDM.Split_Code(_Code; var _Ko, _Mo, _Ta1, _Ta2)` | `Dmu.pas:510-543` | Parses a dash-separated account code `KO-MO-TA1-TA2`. Stops at the first empty or zero segment. `StrToIntDef(…, 0)` so bad input silently becomes 0. | `split('-')` + `parse().unwrap_or(0)` |
| `TDM.Sarfasl_SSN_CODEName(SSN)` | `Dmu.pas:1180-1229` | Formats an account SSN as a zero-padded, dash-joined display code, using the per-fiscal-year digit widths `No_Ko`/`No_Mo`/`No_Ta1`/`No_Ta2` from `Base`. Segments are emitted **right-to-left** (`S := S1 + '-' + S`). Returns `'! Unkhnown !'` *(sic)* if the SSN is ≤ 0 or not found. | Hand-port; the padding widths are per-tenant configuration |
| `TDM.N23` / `Str2String` / `ElfHash` / `IsInteger` / `inttostr3` | `Dmu.pas:569`, `:604`, decl `:124`, `:902`, `:859` | Duplicates of the `TUtil` versions | consolidate |

---


---

Prev: [8. Backup / restore / new company / import](08-08-backup-restore-new-company-import.md) · Next: [10. Shared dialog / frame catalogue](08-10-shared-dialog-frame-catalogue.md)
