_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 2. Application startup sequence

### 2.1 Order of operations (`arzi.dpr:125-297`)

1. **`Application.Initialize`** — `arzi.dpr:126`.
2. **`Myini := TMyini.Create('')`** — `arzi.dpr:127`. Resolves the settings file
   (`INI.pas:241-251`): `<exe path without ext> + 'X.ini'` (i.e. `arziX.ini`); if that
   file does not exist, it falls back to the hard-coded absolute path
   **`D:\Backup\Hesab.ini`**. ⛔ Hard-coded drive letter.
3. **`Application.CreateForm(TDM, DM)`** — `arzi.dpr:128`. This fires
   `TDM.DataModuleCreate` (`Dmu.pas:707-792`), which is where all the real startup work
   happens. See §2.2.
4. **`Application.CreateForm(TMain, Main)`** — `arzi.dpr:129`. Note `TMain` is created
   **twice** (`arzi.dpr:129` and again at `:156`); likewise `TSahamdarEdit`,
   `TCompanyEdit`, `TDKolF`, `TTMoeinF`, `TDaftarT_F`, `TCheckDaryaft2F`,
   `TCheckBargashtF`, `TAnbarReport_F`, `TChangeS_F`, `TToExcelDaraei`, `TSanadView`,
   `TRooznamehView`, `TMoeinToR`, `TDMoeinF`, `TBedBesF`, `TSahamdarInfo`, `TSahamdar`,
   `TKharid`, `TFishDaryaftF`. Only the **last** instance is reachable via the global
   variable; the earlier ones leak. ⛔ DO NOT PORT.
5. **Splash / progress**: `WaitF.initForm('... در حال آماد سازی ', 1, 47)`
   (`arzi.dpr:147`, `WaitU.pas:41-49`) shows a modeless progress window with 47 steps.
   `WaitF.Gotonextposition` is then called 46 times interleaved with form creation
   (`arzi.dpr:150-294`, `WaitU.pas:34-39`).
6. **~150 forms are pre-created eagerly** (`arzi.dpr:128-294`). Nothing is lazy. This is
   the entire reason a splash screen exists.
7. **`Application.Run`** — `arzi.dpr:296`.

### 2.2 What `TDM.DataModuleCreate` does (`Dmu.pas:707-792`)

| Step | Code | Behaviour |
|---|---|---|
| Settings file for the `PropSave` component | `Dmu.pas:711-713` | `PS.FileName := ChangeFilePath(ChangeFileExt(ParamStr(0), '.ini'), 'D:\BACKUP')` — the *effective* settings file is `D:\BACKUP\arzi.ini`, **not** the `arziX.ini` that `TMyIni.Create` computed. ⛔ Two different resolution rules for the same store. |
| Stamp vendor metadata | `Dmu.pas:715-719` | Writes `Program=Green Gold`, `Programer=Mohsen Ranjbar`, `Mobile=09131912805`, `Contact=MohsenRanjbar.1350@Gmail.com` into `[Base]` and saves. |
| Read grid font size | `Dmu.pas:720` | `GridFontSize := PS.ReadInteger('Base','GridFontSize', 8)` |
| Reset backup guard | `Dmu.pas:721` | `Backup := 0` |
| **Connect to SQL Server** | `Dmu.pas:722-732` | If `[Base] CS1 = '1'`, read `[Base] CS2`, decrypt it (`MyIni.ReadEncriptString`) and use it as the ADO connection string. Otherwise pop the **native Microsoft Data Link dialog** (`EditConnectionString(Ado)`) and let the operator type it. |
| Persist the connection | `Dmu.pas:735-740` | On success: write `CS1='1'` and `CS2 = Encrypt(ConnectionString)`; then `UpdateTable` (schema self-migration — currently entirely commented out, `Dmu.pas:244-...`). |
| Reset session | `Dmu.pas:741-749` | `UserID:=0; username:=''; CO_ID:=0; Admin:=false`; opens the `Base` table (companies/fiscal years). |
| **Feature discovery by database presence** | `Dmu.pas:758-782` | Runs one SQL batch using `DB_ID()` to test whether databases `Saham`, `Anbar`, `Rppc_Solution` exist. Sets `Saham_DB`, `Anbar_DB`, `Basc_DB` to `'<name>.Dbo'` or `''`. Also `Saham_F := '\\pesteh\SahamData\'` — a hard-coded UNC share — cleared if `Saham` is absent. |

### 2.3 Login and licence gate (`TMain.FormActivate` → `TMain.Reload`)

`FormActivate` (`Mainu.pas:296-330`) runs on every activation but short-circuits if a
user is already signed in (`if Dm.userId <> 0 Then Exit;` — `Mainu.pas:310`):

1. Load skin directory/name from `[Base] SkinDirectory` / `[Base] SkinName`, defaulting
   to `<exe dir>\Skins\` (`Mainu.pas:302-306`).
2. Hide the splash (`Mainu.pas:309`).
3. Clear the session (`Mainu.pas:312-315`).
4. **`GetPass.init`** — the modal login dialog (`Mainu.pas:317`, `GetPassu.pas:45-60`). See §3.
5. Set the company/fiscal-year caption from `Dm.Base` (`Mainu.pas:318-322`).
6. **`Reload`** (`Mainu.pas:323` → `Mainu.pas:884-968`):
   - Re-reads `[Base] CS3` from a **freshly constructed** `TMyIni` (`Mainu.pas:891-893`).
   - **Nightly auto-backup**: `DoBackup` (`Mainu.pas:894` → `Mainu.pas:393-414`).
   - **Licence check** (`Mainu.pas:895-905`):
     - if `CS3 = 306239` → *demo mode*: opens `Moein` and allows startup only while
       `RecordCount < 200`.
     - else → `TestF.Test`; if false, show `TestF` (the activation dialog) and re-test.
     - **`if not E then Close;`** — a failed licence check closes the main form, which
       triggers the "are you sure you want to exit?" confirmation (`Mainu.pas:344-348`).
   - Applies the ~35 permission-driven `.Enabled` assignments (`Mainu.pas:907-961`).
7. Hide the pistachio tab if `Anbar` DB is absent (`Mainu.pas:325-329`).

### 2.4 What fails startup

| Failure | Effect | Code |
|---|---|---|
| No `arziX.ini` **and** no `D:\Backup\Hesab.ini` | `TMyIni` still constructs; reads return defaults. `PS.FileName` (`D:\BACKUP\arzi.ini`) is separately created by `PS.SaveFile`. | `INI.pas:241-251`, `Dmu.pas:711-719` |
| SQL Server unreachable / bad `CS2` | `Ado.Open` raises → unhandled exception at `Dmu.pas:728`. There is **no try/except**. | `Dmu.pas:722-732` |
| `Base` table missing | `Base.Active := True` raises. | `Dmu.pas:747` |
| Login cancelled | `BitBtn2Click` → `Application.Terminate; halt;` — process exits immediately. | `GetPassu.pas:110-116` |
| No company selected (`CO_ID_IN.KeyValue < 1`) | Login button silently refocuses the company combo. | `GetPassu.pas:65-69` |
| Unknown user code | `کد کاربر اشتباه است` (message text is mojibake in source — the file is Windows-1256) | `GetPassu.pas:74-78` |
| Disabled user (`Enabled = 0`) | Message; login refused. | `GetPassu.pas:79-83` |
| Wrong password | Message; password box cleared and refocused. **No counter, no lockout, no delay.** | `GetPassu.pas:88-93` |
| Licence test fails and user cancels activation | `Main.Close` → exit-confirmation dialog. If the user answers "No" the app **stays open, fully unlicensed but with permissions applied** — `Reload` returns and the ribbon is live. ⛔ The licence gate is not fail-closed. | `Mainu.pas:905`, `Mainu.pas:344-348` |
| Fiscal year archived (`Base.IsActive <> 1`) | Not a startup failure — blocks *new document creation* only, with the message `سال مالی مورد نظر بایگانی شده است` | `Dmu.pas:997-1015` |

---


---

Prev: [1. The complete main-menu tree](08-01-the-complete-main-menu-tree.md) · Next: [3. Authentication](08-03-authentication.md)
