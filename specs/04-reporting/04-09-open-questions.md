_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 9. Open questions

Each item states **what is unknown**, **what artefact or query answers it**, and **what is blocked**
until it is answered. Ordered by how much of the rebuild depends on it.

### 9.1 Blocking — database artefacts that must be dumped

**Q1. Stored-procedure bodies.** None exist in this repository. Dump all of them:

```sql
SELECT o.name, m.definition
FROM sys.sql_modules m JOIN sys.objects o ON o.object_id = m.object_id
WHERE o.type IN ('P','FN','IF','TF','V')
ORDER BY o.type, o.name;
```

The ones this document depends on, by name:

| Object | Type | Called from | What it must tell us |
|---|---|---|---|
| `Taraz_6Sotooni` | proc | `Dmu.dfm:756`, `Taraz6SetooniU.pas:96` | **Everything about R02**: whether `Bed1`/`Bes1` are gross prior turnover or a netted opening balance; what `@kind` 1 vs 2 selects; how `@Sabt` 1/2/3 maps to `M_Tx`; whether `@Level` is cumulative or single; the exact output column list; **and whether it writes.** |
| `KolState` | proc | `KolStateU.dfm:105` | R07's filter — `M_kind`? `M_Tx`? date bounds? — and the meaning of `mahstr`. |
| `Select_Kol` | proc | `KolStateU.dfm:88` | The Kol lookup list; probably trivial. |
| `Taraz4Setooni` | proc | `Dmu.dfm:34-84`, **never opened** | Confirm it is genuinely orphaned before deleting the declaration. |
| `Dbo.Make_R(@Co,k,m,t1,t2)` | scalar UDF | `Taraz4Setooni_U.pas:145`, `RoyatJU.pas:367`, `Dmu.pas:278` (commented) | **The authority on account-code rendering.** Two call sites pass different `@Co` (picker value vs hard-coded `1`); the function's dependence on `@Co` is unknown. Blocks §11's `format_account_code`. |
| `Dbo.Make_L(@Co,…)` | scalar UDF | `Dmu.pas:274` (commented) | The padded/left variant; needed to decide whether `Sarfasl.M_L`/`M_R` are nested-set bounds or formatted codes. |
| `SP_XNEW` / anything else `Dmu.dfm` declares | proc | `Dmu.pas:1236` | Out of scope here; dump anyway. |

**Blocked:** R02 cannot be specified, reconciled or rebuilt. R07's semantics are guesswork. Account
code formatting is undefined across the whole rebuild.

**Q2. Column types, lengths and collation of every date column.** §5.1 can prove the *format* but not
the *type*.

```sql
SELECT c.name, t.name AS type_name, c.max_length, c.collation_name, c.is_nullable
FROM sys.columns c JOIN sys.types t ON t.user_type_id = c.user_type_id
WHERE c.object_id = OBJECT_ID('Moein') ORDER BY c.column_id;
```
plus the same for `DMoein`, `Base`, `Sarfasl`, `SahamdarConfig`, `Tanzim`.
**Blocked:** the Postgres DDL for `entry_date_jalali`, and whether a `CHECK` on the format is safe.

**Q3. Are there malformed dates in production?**

```sql
SELECT TOP 100 M_COID, M_Sanad, M_Date FROM Moein
WHERE M_Date NOT LIKE '[0-9][0-9][0-9][0-9]/[0-9][0-9]/[0-9][0-9]'
   OR LEN(M_Date) <> 10;
SELECT M_COID, COUNT(*) FROM Moein
WHERE M_Date NOT LIKE '[0-9][0-9][0-9][0-9]/[0-9][0-9]/[0-9][0-9]' GROUP BY M_COID;
```
Also check for Persian-Indic digits: `WHERE M_Date LIKE N'%[۰-۹]%'`.
**Blocked:** the migration's date-conversion step, and whether §5.2's four invariants hold.

**Q4. Does `Moein` actually balance?** The 4-column trial balance is self-proving only if it does
(§2.1), and nothing ever checks.

```sql
SELECT M_COID, M_kind, SUM(M_Bed) AS d, SUM(M_Bes) AS c, SUM(M_Bed) - SUM(M_Bes) AS diff
FROM Moein GROUP BY M_COID, M_kind ORDER BY M_COID, M_kind;

SELECT M_COID, M_Sanad, SUM(M_Bed) - SUM(M_Bes) AS diff
FROM Moein GROUP BY M_COID, M_Sanad HAVING SUM(M_Bed) <> SUM(M_Bes);
```
**Blocked:** whether the rebuild's mandatory balance assertion can be turned on at migration time or
must start as a warning.

**Q5. How far has `DMoein` drifted from `Moein`?**

```sql
SELECT d.DM_COID, d.DM_Sanad, d.DM_TBed, m.d, d.DM_TBes, m.c
FROM DMoein d
JOIN (SELECT M_COID, M_Sanad, SUM(M_Bed) d, SUM(M_Bes) c FROM Moein GROUP BY M_COID, M_Sanad) m
  ON m.M_COID = d.DM_COID AND m.M_Sanad = d.DM_Sanad
WHERE d.DM_TBed <> m.d OR d.DM_TBes <> m.c;
```
**Blocked:** whether `RooznamehViewU`'s displayed totals and the voucher prints' amount-in-words
(§1.7, §6.4) are currently wrong in production.

### 9.2 Blocking — semantics that only the customer can settle

**Q6. Should reports include unposted (state 0) vouchers?** Today: the 4-column trial balance, all
four ledgers, Card Jari, the party balance list, the voucher summary and **the tax-authority Excel
export** all include drafts; only the 6-column trial balance excludes them. The two trial balances
therefore never agree.
*Answer by:* asking the accountants, and by running
`SELECT M_COID, M_Tx, COUNT(*), SUM(M_Bed) FROM Moein GROUP BY M_COID, M_Tx;` to see how much money
sits in state 0 at any moment.
**Blocked:** the default value of `states` on every endpoint, and §10.1.

**Q7. Is the general ledger supposed to be a ledger of journal summaries?** `DKolU` reads only
`M_kind = 2` rows, which exist only after someone manually runs `MakeRooznamehU` or `MoeinToRU`
(§3.0). Is that the intended workflow, or has the general ledger been quietly broken for years?
*Answer by:* `SELECT M_COID, M_Kind, COUNT(DISTINCT M_Sanad) FROM Moein GROUP BY M_COID, M_Kind;` —
if `M_Kind = 2` voucher counts are low or absent for recent years, the answer is "broken".
**Blocked:** whether R03 is ported as-is or redefined as a roll-up of `M_kind = 1`.

**Q8. Which opening-balance boundary is correct?** The ledgers split at `< from_date`; the party
balance list splits at `<= from_date` (§1.2). One of them is wrong and they cannot both be kept.
*Answer by:* the accountants; then reconcile a sample party's ledger against its row in R09.
**Blocked:** §10.2.

**Q9. What does the tax authority's template actually require?** The export labels column A `ردیف`
while writing `M_Sanad` into it, omits Tafsil levels, and includes drafts (§7.2).
*Answer by:* obtaining the current official specification for `صورت حساب الکترونیکی` and one
previously accepted file.
**Blocked:** R25, which is the highest-consequence report in the system.

**Q10. Do the three voucher-signature keys 1011/1013/1014 mean what their labels say?** Ledger prints
use 1013, trial balances use 1014, vouchers use 1011, and 1012 is read by the dead `PrintMU` only
(§6.5). Is that a deliberate three-way split or an accident?
*Answer by:* `SELECT T_ID, T_Str, T_Desc FROM Tanzim ORDER BY T_ID;` on a live database.
**Blocked:** how many configurable signature blocks the rebuild needs.

### 9.3 Non-blocking — verifiable, but needs a live database or a runtime check

**Q11. Does `SahamdarConfig` contain duplicate `(SC_K, SC_M, SC_T)` triples with `SC_Rem = 1`?** If
so, `Dm.Jari_Rem` (no `GROUP BY`) and `RoyatJU`'s inline expansion double-count while `QList` does not
(§4.3, §1.4).
```sql
SELECT SC_K, SC_M, SC_T, COUNT(*) FROM SahamdarConfig WHERE SC_Rem = 1
GROUP BY SC_K, SC_M, SC_T HAVING COUNT(*) > 1;
```

**Q12. Is `SC_Rem` a boolean flag or an ordering key?** `Dm.Jari_Rem` filters `SC_Rem = 1`;
`QList` uses `-Min(SC_Rem)` as a sort key (§4.2). Both readings fit if the domain is `{0,1}`.
```sql
SELECT SC_Rem, COUNT(*) FROM SahamdarConfig GROUP BY SC_Rem;
```

**Q13. Does the `Jari` ADO parameter truncate card numbers?** Declared `ftWideString, Size = 4` with
`Prepared = True` (`CardJariU.dfm:6496-6502`); the same pattern appears on `BedBes.Q1.Coid`
(`.dfm:403-406`).
*Answer by:* `SELECT MAX(S_Card), MAX(LEN(CAST(S_Card AS varchar))) FROM Sahamdar;` and by running
Card Jari against a ≥5-digit card on the legacy binary.

**Q14. Is `Sarfasl.LineName` a real column, and how does it differ from `S_Name` and `FullName`?**
Read by `CardJariU.pas:143`, `SanadEditU.pas:373`, `PrintM2U.dfm:69`, `FactorPrint3U.dfm:3328`, and
**absent from `02-data-model.md`'s `Sarfasl` inventory**.
```sql
SELECT TOP 20 S_Ko,S_Mo,S_Ta1,S_Ta2, S_Name, LineName, FullName, M_L, M_R FROM Sarfasl;
```
**Also:** how stale are `M_R` / `FullName`, given their maintenance is commented out
(`02-data-model.md` §4.1.3)? `SELECT COUNT(*) FROM Sarfasl WHERE M_R IS NULL OR M_R = '';`

**Q15. How many `temp_RJ_*` / `temp_R7_*` tables exist in production?**
```sql
SELECT name, create_date FROM sys.tables WHERE name LIKE 'temp[_]%' ORDER BY name;
```
Confirms §1.4's claim that they accumulate and are never cleaned up, and tells us how many users have
ever run the report.

**Q16. Do voucher numbers collide between `M_Kind = 1` and `M_Kind = 2`?** If they do,
`RooznamehViewU`'s print (`:177`, no `M_kind` filter) and its date-change `UPDATE` (`:328`) touch the
wrong rows, and `Report6U`'s gap check mixes the two sequences.
```sql
SELECT M_COID, M_Sanad FROM Moein GROUP BY M_COID, M_Sanad
HAVING COUNT(DISTINCT M_Kind) > 1;
```

**Q17. Is `Tools.pas` recoverable?** Only the compiled unit is present; `Farsi_Date`, `Farsi_Valid`,
`Farsi_day/month/year` and `SetToDate` are used across ~40 units (§5.1).
*Answer by:* asking the original vendor, checking backups, or — failing both — decompiling
`Tools.dcu` far enough to extract the leap-year rule and epoch offset. Until then, treat the Jalali
conversion as **unspecified** and validate the rebuilt converter against production data rather than
against the legacy source.

**Q18. Which fonts are installed on the workstations?** `B Yekan` and `WeblogmaYekan` drive the
Persian-digit toggle (§6.7); `B Nazanin`, `B Titr` and `Vazir` appear in layouts. If any is missing,
reports have been silently rendering in a fallback font.

**Q19. Does `Anbar_Amalkard`'s unconditional `UPDATE Anbar_FactorD SET AFD_Customer = (…)` still
match every row?** (§1.11.)
```sql
SELECT COUNT(*), COUNT(DISTINCT AFD_Customer) FROM Anbar_FactorD;
```
Owned by `05-inventory.md`, but it is a *report* doing it, so it is flagged here.

**Q20. Has anyone used the preview's `pbEdit` / `pbLoad` buttons to alter a report layout?** (§6.2.)
FastReport stores designer state under `IniFile = '\Software\Fast Reports'` in the registry; check
`HKCU\Software\Fast Reports` on a few workstations, and look for stray `.fr3` files next to the
executable. If layouts have been edited in the field, the `.dfm` layouts in this repository are not
what users actually see.

### 9.4 Questions this document deliberately does not answer

- The `DMoein` header/`Moein` line write model, voucher state transitions and locking — see
  `03-accounting-core.md`.
- Inventory and invoice report internals — see `05-inventory.md`.
- Cheque, deposit and petty-cash report internals — see `06-treasury.md`.
- `Sahamdar` / `SahamdarConfig` as a data model, and the `Saham.Dbo` cross-database dependency — see
  `07-parties-and-shareholders.md`.
- The `Pass_Config` permission-key inventory in full, the INI/registry settings model and the
  connection-string obfuscation — see `08-platform-and-security.md`.


---

[← SS8 Rebuild recommendations](04-08-rebuild-recommendations.md) | [Index](00-index.md) | [SS10 PROPOSED IMPROVEMENTS (1/2) →](04-10-a-proposed-improvements.md)
