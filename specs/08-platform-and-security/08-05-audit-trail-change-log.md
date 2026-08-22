_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 5. Audit trail / change log

### 5.1 `ChangesU.pas` is **not** a change log

Despite the name, `ChangesU.pas` / `TChangeS_F` is the **fiscal-year / company selector**.
It is opened from the ribbon button captioned `تغيير سال مالي` ("Change fiscal year")
(`Mainu.pas:421-431`), runs `Select * From Base Order By Co_ID` (`ChangesU.pas:62-67`),
and on double-click or on OK sets `DM.CO_ID` and `DM.RegSal` from the selected row
(`ChangesU.pas:48-54`, `:76-81`). Deletion is blocked by `Q1BeforeDelete → Abort`
(`ChangesU.pas:71-74`). The Persian glossary term *Taghirat* ("changes") misled the
original naming.

**There is no change-log or audit-trail feature in this application.**

### 5.2 What audit data *does* exist

Audit information is a handful of denormalised stamp columns on business tables,
written inline by the code that creates the row. There is no central log, no reader,
no retention policy, and no coverage of deletes or of security events.

| Table | Column | Meaning | Written at |
|---|---|---|---|
| `Moein` (ledger lines) | `M_User` | `DM.userId` of the creator | `ArticleMoeinu.pas:156`, `ArticleRooznamehU.pas:138`, and inline in ~12 `Insert Moein (…, M_User, …)` statements (`CheckBargashtu.pas:222`, `CheckDaryaft2U.pas:202`, `CheckDaryaftU.pas:330`, `CheckEsterdadU.pas:201`, `CheckVosoolU.pas:234`, `EnteghalU.pas:251-272`, `FISHDaryaftU.pas:467`, `FactorPesteh_U.pas:223`, `CheckEditU.pas:485`, …) |
| `Moein` | `M_Time` | Row timestamp | `CheckEditU.pas:485`, `FactorPesteh_U.pas:223` — **only two of the insert sites populate it** |
| `DMoein` (document headers) | `DM_MUser` / `DM_MDate` | Created-by / created-at | `Dmu.pas:834-835` (`DMoein_Make`, insert branch; `DM_MDate := GetDate()`) |
| `DMoein` | `DM_CUser` / `DM_CDate` | Last-modified-by / at | `Dmu.pas:831` (update branch; `DM_CDate := GetDate()`). On insert `DM_CUser` is set to `0` and `DM_CDate` is not set at all (`Dmu.pas:834-835`). |
| `CheckMaster` | `CM_UserID` | Creator | `CheckEditU.pas:417-438` |
| `Anbar_FactorMaster` | `FM_UserID` | Creator | `FactorPesteh_U.pas:198` |
| `DCheck` | `S_UserID` | Creator | field declared `Dmu.pas:67` |

`DM_MUser/DM_MDate/DM_CUser/DM_CDate` are surfaced read-only in the document browsers
(`SanadViewU.pas:41-44`, `RooznamehViewU.pas:34-37`, `SanadViewU.dfm:1362-1373`,
`RooznamehViewU.dfm:600-611`).

### 5.3 What is **not** recorded — at all

- Logins, failed logins, logouts.
- Password changes (self-service or admin-forced).
- User creation, enable/disable, rename.
- **Permission grants and revocations** — `Pass_Config` is deleted and rewritten with no
  history (`Admin.pas:192-214`).
- Deletions of any kind. There are no soft deletes and no tombstones; `Delete_Sanad_moein`
  (`Dmu.pas:1290+`) is a hard delete.
- Document lock/unlock (`SanadViewU.pas:605-635` just flips `DM_Lock`).
- Settings changes (`TanzimU.pas`, `TanzimChapu.pas` write straight to `Base` / `Tanzim`).
- Company / fiscal-year creation (`MakeNewU.pas`).
- Backups taken (`Backup_U.pas`, `Mainu.pas:393-414`).
- Licence activation (`testmainU.pas:53-65`).
- Data exports (`ToExcelDaraeiU`, `MoeinZipU`).
- Every "before" value — only the *latest* editor id survives.

⛔ The rebuild must add a real, append-only audit log. See §14.

---


---

Prev: [4. Authorization](08-04-authorization.md) · Next: [6. Settings](08-06-settings.md)
