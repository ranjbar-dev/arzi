_Part of [08-platform-and-security](../08-platform-and-security.md) — [index](00-index.md)_

## 13. Open questions

**Authentication & users**

1. How many user accounts exist in production, and how many are `Supervisor = 1`?
2. Are passwords shared between people today? (Plaintext storage + no audit means we
   cannot tell from the data.) This determines whether we can migrate accounts at all or
   must force a full re-enrolment.
3. Since passwords are plaintext, migration is a policy decision: force-reset everyone,
   or hash the existing plaintext once and require a change on first login?
4. Do users need to be scoped to a company/tenant, or is the current global-user model
   intended?
5. Is `UserName` guaranteed unique today? Nothing enforces it (`Admin.pas:178`).

**Authorization**

6. Permissions **1129** (`خلاصه گردش اسناد`), **1130** (`تبدیل اسناد معین به روزنامه`)
   and **1415** (`Save To Disk`) have checkboxes but **no call sites**. Were these
   features removed, or is the enforcement missing? Should they exist in the rebuild?
7. **2119** is a gap in the numbering. Was there a permission there?
8. `FishListD.pas` maps the same three buttons to **two different permission triples**
   (2116/2117/2118 at `:139-141`, 2102/2103/2104 at `:165-167`). Which is correct?
9. Permission **1108** (delete current account) is checked but its checkbox is
   `Visible = False` — so no non-supervisor can ever have it. Intentional?
10. Permissions **1102** and **1103** are effectively dead (`ListSarfaslu.pas:78-80`
    overwrites, `Mainu.pas:909` hard-disables). Should "create account" and "amend
    account" be real, separately-grantable permissions in the rebuild?
11. Should `Supervisor` become a role in a proper RBAC model, and should the
    lock-bypass remain a supervisor power or become its own permission?
12. Should permissions be per-company/per-fiscal-year rather than global?

**Settings & data**

13. `Base.IsActive` gates all posting but has **no UI** (`Dmu.pas:1008`). How is a
    fiscal year archived today — direct SQL? Should the rebuild expose an open/close-period
    feature?
14. `Tanzim` (print parameters) is **not** keyed by company. Should these become
    per-tenant, per-company, or stay global?
15. `Base.No_Ko` / `No_Mo` / `No_Ta1` / `No_Ta2` (account-code digit widths) are read but
    have **no editor** in `TanzimU`. Are they ever changed after setup?
16. Should per-form window geometry and grid column widths (≈50 forms' worth) be
    preserved as user preferences, or dropped?

**Calendar & numbers — blocking for data fidelity**

17. **Which Jalali algorithm produced the dates in the live database?** `TUtil.FarsiDate`
    (`Utility.pas:435`, 2-digit year, arithmetic approximation) and
    `TDM.MiladiToShamsi` (`Dmu.pas:362`, 4-digit, near-correct) disagree. We need a full
    scan of stored `varchar(10)` dates against a correct implementation, and sign-off on
    every discrepancy before migration.
18. Are there stored dates in the **8-character `YY/MM/DD`** form as well as
    `YYYY/MM/DD`? `TDM.IsDate` accepts both and prefixes `'13'` (`Dmu.pas:888`).
19. `Str2String` caps at `تريليارد` (trillion) and silently truncates above
    (`Utility.pas:504-512`). Has this ever been hit? Do we need to extend the scale?
20. `N23`'s constant table uses Arabic `ي`/`ك` and has an inconsistent leading space on
    `'چهار'`. Confirm the printed output is *correct as-is* and must be preserved
    byte-for-byte, rather than being a long-standing typo to fix.
21. `Adj_Cent` pads but never truncates (`Dmu.pas:688`). Is `'1.234'` a real case?

**Backup / licensing / operations**

22. Is the `.ABS` manual backup actually used, given there is **no restore**? Or is
    recovery always from the SQL `.bak`?
23. The manual backup **drops the company logo** (`Backup_U.pas:116`) and silently drops
    unmapped column types. Is anyone relying on it as a true archive?
24. Backups go to `Base.BackupDir` — a path interpreted by SQL Server for the `.bak` and
    by the client for the `.ABS`. What are the real values in production?
25. New-fiscal-year creation does **not** copy the chart of accounts (`MakeNewU.pas:129-150`
    is commented out). How is a new year actually set up today?
26. Is `test.exe` part of the current sales/support process? What replaces it?
27. What licensing model does the business want for the hosted product: per-tenant
    subscription, per-seat, feature tiers (the current `Anbar`/`Saham`/`Rppc_Solution`
    database-presence gating), or usage caps (the current 200-row demo)?
28. The `Saham` (shareholder) and `Rppc_Solution` (weighbridge) databases and the
    `\\pesteh\SahamData\` share (`Dmu.pas:758-780`) — are these in scope for the rebuild?
29. Is the `عملیات خرید پسته` (pistachio) tab a customer-specific vertical, or core
    product?
30. What is `[Base] CS31` in `arzi.local.ini` (`108763797`)? No code reads or writes it.

**Environment**

31. `D:\BACKUP\arzi.ini` and `D:\Backup\Hesab.ini` are hard-coded (`Dmu.pas:711`,
    `INI.pas:246`). Is `D:` guaranteed on every workstation? Are these on a share?
32. The `Tools` unit (`TFullDate`, `TEditDate`, `TEditInt`, `TMyEdit`, `TFocusLabel`) is
    used pervasively but is **not in the project directory** — it is on an external
    library path. **`TFullDate` is the production Jalali date control**, and we do not
    have its source. Can it be obtained? Question 17 cannot be closed without it.
33. Same for the third-party components: AlphaControls, FastReport, Absolute Database,
    `rDBGrid`, `PropSave`, LibXL. Which behaviours do they contribute that we would have
    to reimplement?

---


---

Prev: [12. What `test.dpr` / `testmainU.pas` is](08-12-what-test-dpr-testmainu-pas-is.md) · Next: [PROPOSED IMPROVEMENTS (needs user approval)](08-14-proposed-improvements.md)
