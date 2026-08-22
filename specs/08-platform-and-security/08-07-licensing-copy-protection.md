_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 7. Licensing / copy protection

Described at the level needed to plan a replacement. **No key material, no derivation
inputs beyond what is structurally necessary, and no activation procedure is documented
here.**

### 7.1 Mechanism, functionally

- The scheme is a **per-machine node lock**. `testmainU.pas` derives five 4-digit
  numbers from stable hardware/OS identifiers of the workstation (`TTestF.MakeD`,
  `testmainU.pas:121-160`, using `TSysInfo` probes in `LockUnit.pas:26-244`: volume
  serial, CPU identity, BIOS/video dates, system product name, plus the executable's own
  path). Each is reduced with the same non-cryptographic `ElfHash`
  (`LockUnit.pas:62-75`) and normalised to exactly four digits.
- The vendor turns those five numbers into five response numbers (`TTestF.makeR`,
  `testmainU.pas:173-211`). This step is **not** a secret algorithm — it is the same
  `ElfHash` applied to concatenated pairs — and it is guarded only by a password gate
  (`testmainU.pas:79-100`).
- The customer's five response numbers are concatenated and hashed once more into a
  single integer (`TTestF.MakeI`, `testmainU.pas:162-171`) which is stored as
  **`[Base] CS3`** in the ini file (`testmainU.pas:61`).
- At every login, `TTestF.Test` (`testmainU.pas:213-233`) recomputes the expected value
  and compares it with the stored `CS3`.

### 7.2 Where it is checked

`TMain.Reload` (`Mainu.pas:884-905`), once per login:

| `CS3` value | Behaviour | Line |
|---|---|---|
| `306239` | **Demo mode**: opens `Moein` and permits startup only while `RecordCount < 200`. Beyond 200 ledger rows the app closes. | `Mainu.pas:895-900` |
| anything else | `TestF.Test`; on failure show the activation dialog `TestF.init`, then re-test. | `Mainu.pas:900-904` |
| test still false | `Main.Close` → exit-confirmation dialog | `Mainu.pas:905`, `Mainu.pas:344-348` |

### 7.3 Structural weaknesses (for planning, not exploitation)

1. **A blanket bypass exists.** `TTestF.Test` returns true if *either* the per-machine
   check passes **or** a second value derived from the executable's install path equals
   a hard-coded constant (`testmainU.pas:228-231`). Any installation at the "blessed"
   path is licensed unconditionally.
2. **The gate is not fail-closed.** A failed check calls `Main.Close`, which routes
   through `TMain.FormClose` and its "are you sure?" prompt (`Mainu.pas:344-348`).
   Declining leaves the application running with the ribbon already enabled by
   `Reload`'s permission pass.
3. **The check runs once, client-side, at login.** No re-validation, no server contact,
   no expiry, no revocation.
4. **The licence value lives in the same plaintext ini file as everything else** and is
   read by constructing a fresh `TMyIni` (`Mainu.pas:891-893`).
5. **The generator ships with the product.** `testmainU.pas` is compiled into `arzi.exe`
   (`arzi.dpr:46`, `arzi.dpr:210`) and its unlock button is merely `Visible := dm.admin`
   (`testmainU.pas:117`). The separate `test.exe` (§12) is the same code with no host app.
6. **`ElfHash` is a 32-bit non-cryptographic string hash** with abundant collisions; it
   provides no authenticity.
7. **The fingerprint is brittle**: replacing a disk, a motherboard or a GPU driver
   changes the derived numbers and invalidates the licence, requiring re-activation.

### 7.4 `Get_Serial.pas` and `CS2` — commonly confused, unrelated

- **`Get_Serial.pas` is not product licensing.** It is a *weighbridge-ticket lookup*:
  the user enters a ticket serial (`Serial`) and a current-account number (`Jari`); the
  app opens a **second** connection `ADO_RPPCSOLUTION` to the `Rppc_Solution` database,
  runs the stored procedure bound to `B_SelectSerial` with `@GhabzNo`, and requires
  exactly one row whose `SerialNoPsnBts` matches `Jari`
  (`Get_Serial.pas:51-76`). Its only caller is the lab/weighbridge form
  (`Lab.pas:41-64`). The result is stashed in the form's `Tag`.
- **`CS2` is the encrypted ADO connection string** (`Dmu.pas:726`, `Dmu.pas:737`) —
  §6.1. It has nothing to do with licensing. `CS1` is the "have I got one?" flag,
  `CS3` is the licence, `CS31` is dead.

### 7.5 What this implies for the online rebuild

The desktop model — one node lock per workstation, checked locally, tied to hardware —
has **no meaningful analogue in a hosted web application** and should not be
reconstructed. Decisions the customer must make (see §13):

- **Tenancy.** Today a "licence" is per *machine* and a "company" (`Base.Co_ID`) is per
  *fiscal year*, not per organisation. `RegName` is set from `Base.Co_Name`
  (`GetPassu.pas:102`), so what looks like a licensee name is really just the company
  label on the selected fiscal year. A real tenant concept does not exist in the data
  model and has to be introduced.
- **Seats.** There is no seat concept at all. The `Password` table is unbounded and
  unscoped; two people can hold the same account simultaneously with no detection.
- **Feature gating.** Today this is done by *database presence* (`Anbar`, `Saham`,
  `Rppc_Solution` — `Dmu.pas:765-777`) and by the demo row-count cap
  (`Mainu.pas:895-900`). Both are effectively product tiers that should become explicit
  subscription entitlements.
- **Enforcement point.** Any replacement must be evaluated server-side, per request,
  and must fail closed.

---


---

Prev: [6. Settings](08-06-settings.md) · Next: [8. Backup / restore / new company / import](08-08-backup-restore-new-company-import.md)
