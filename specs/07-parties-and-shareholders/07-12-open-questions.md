_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 12. Open questions

**Q1 — Tafsil-2 party placement is half-implemented.** `SahamdarConfig.SC_T > 0` is honoured by
`Jari_Rem` (`Dmu.dfm:8672`) and `QList` (`CardJariU.dfm:6517`) but never produced by `Sarfasl_Add`
(`SahamdarEditU.pas:325`, `CompanyEditU.pas:296`). Does production actually contain `SC_T > 0` rows?
If yes, how were those accounts created — manually via `SNewu`? If no, may we drop the Tafsil-2
branch entirely?

**Q2 — Moving a party between person and company does not remigrate its accounts**
(`SahamdarU.pas:136-176`). Should the rebuild move/close the old detail accounts, or is the legacy
"leave them where they are" behaviour intentional?

**Q3 — Currency.** No currency column exists; `ریال` (Rial) is hard-coded
(`Factorprint2U.pas:98`). Is the system Rial-only forever, or is Toman/multi-currency needed? (The
project name `arzi` literally means "currency/foreign-exchange", which is suspicious.)

**Q4 — `IsActive` on a newly created fiscal year** is cloned from the source row
(`MakeNewU.pas:117-118`). Should a new year always start open?

**Q5 — Is multi-company real?** Every `Base` row we can see differs only by year. Are there
production installations with two genuinely different `Co_Name` values sharing one database — and if
so, is cross-company data leakage (shared `Sarfasl` and `Sahamdar`) acceptable or a defect?

**Q6 — `EnteghalU.pas:~204-206`** builds the account tuple with `',0' + Taraf.ETa1.Text`. Confirm the
intent is "default empty to 0" so we can write it correctly.

**Q7 — `M_User = 68` hard-coded** in every rollover posting (`EnteghalU.pas:~253` ff.). Is 68 a
system/service account? Should the rebuild stamp the acting user instead?

**Q8 — Credit limits.** There is no credit-limit field or check anywhere. Is this genuinely not a
requirement, or is it managed outside the system?

**Q9 — Two account-code string formats coexist:** `Ko-Mo-Ta1-Ta2` unpadded
(`TarafU.pas:104-113`, `Dmu.pas:510-545`) and `Ta2-Ta1-Mo-Ko` zero-padded
(`Dmu.pas:1180-1229`). Which is canonical for users? We propose standardising on one.

**Q10 — Blank national ID blocks creation.** `SahamdarEditU.pas:263-271` treats an empty
`S_CodeMelli` as a duplicate once one blank row exists. Should blank IDs be exempt from the
uniqueness rule?

**Q11 — `CompanyEditU.pas:275` silently discards `محل تاسیس`** (place of incorporation) even though
the field is shown and editable. Bug or intentional?

**Q12 — `Sahamdar_Edit` uses `@Kind` = 0/1** (`SahamdarP.pas:134`) while the live code uses 1/2.
Confirm the procedure can be retired.

**Q13 — Shareholder equity and profit distribution are absent from `arzi`** (§5). Where do they
live? Should the rebuild absorb the `Saham` product, integrate with it, or ignore it? **This is the
single most important question for scoping this domain.**

**Q14 — Sign conventions differ within one screen:** `Jari_Rem` returns credit-positive
(`Dmu.dfm:8683`), the per-account grid rows are debit-positive-in-`R_Bed`
(`CardJariU.pas:157-160`). Which should the rebuild present?

**Q15 — `SahamdarInfo` is read-only in `arzi`** (buttons unwired, §6.6). Who maintains those rows
today?

**Q16 — `SahamdarConfig.SC_Tik` is a globally mutated scratch column** (§7.4). Confirm nothing else
depends on its persisted value so we can compute it per-request.

**Q17 — Unticking a control account does not remove the detail account** (§7.5). Should the rebuild
offer deactivation/closure, and what should happen if the account has entries?

**Q18 — `BastanHesab` writes to hard-coded `D:\Bed.GGS` / `D:\Bes.GGS`** (`BastanHesab.pas:45-46`).
Who consumes those files? Is the `.GGS` INI format an external contract we must preserve?

**Q19 — Stored-procedure bodies are not in the repository**: `Sarfasl_ADD`, `Sahamdar_Seek`,
`Sahamdar_Edit`, `Sahamdar_Show`, plus the SQL functions `Dbo.Make_L` / `Dbo.Make_R`
(`Dmu.pas:274-278`) and the `BastanHesab` procedure. We need a database script dump to port
`Sarfasl_ADD` faithfully.

**Q20 — Jalali date storage width** is 8 in `Sahamdar_Edit` (`YY/MM/DD`) but the live UI writes up to
10 (`YYYY/MM/DD`); `Dm.IsDate` accepts both and prefixes `'13'` to 8-char values
(`Dmu.pas:887-888`). Is there legacy 8-char data in production?

**Q21 — `Sahamdar_Show(@Id)`** has zero call sites. What did it return? (Possible remnant of the
equity feature.)

**Q22 — Are `Sarfasl.S_IS_Check` / `S_IS_Fish` / `S_IS_APArdakhti` / `S_IS_ADaryafti` live?** They
are displayed in `S_KolU.dfm:242-257` but every write path is commented out
(`Sarfasl_TakmilU.pas:75-82`). Are they enforced elsewhere (perhaps inside a stored procedure)?

**Q23 — `Sarfasl.S_Card` (Link 2) vs positional linkage (Link 1)** (§4.3). Only a manual tool writes
`S_Card`. Is it authoritative anywhere, or purely informational? Can we make one link canonical?

**Q24 — `Dm.userId = 68` gates the person⇄company move** (`SahamdarU.pas:101`). Should this become
a proper permission?

---


---

[← Previous](07-11-naming-map.md) · [Index](00-index.md) · [Next →](07-13-proposed-improvements.md)
