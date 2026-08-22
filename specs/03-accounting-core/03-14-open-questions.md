_Part of [03-accounting-core](../03-accounting-core.md) — [index](00-index.md)_

## 14. Open questions

Ordered by how much they block the rebuild.

### Blocking — must be answered before implementation

1. **The 30 stored procedures and 2 scalar functions are not in this repository.** Listed in §0.3.
   The most critical are `Sarfasl_ADD` (all account-creation validation), `Sarfasl_Deep` (account
   deletion), `Active_Set` (denormalisation rebuild), `Make_R` / `Make_L` (code string format), and
   `Moein_All`. **Extract the full DDL from the live SQL Server database.**
2. **No table DDL anywhere.** Column types, nullability, defaults, indexes, constraints and collations
   are all inferred from usage. In particular: is there a unique index on
   `Sarfasl(S_Ko, S_Mo, S_Ta1, S_Ta2)`? On `DMoein(DM_Coid, DM_Sanad)`? Are there any foreign keys at
   all? **Script the schema.**
3. **Is the chart of accounts genuinely global (§1.7)?** `Sarfasl.S_COID` exists but is ignored by
   every live query, and the code that would have copied the chart per year is commented out
   (`MakeNewU.pas:129-150`). Confirm with the users whether accounts are meant to be shared across
   fiscal years. This decides the entire `accounts` table design.
4. **What are the real digit widths?** `Base.No_Ko/No_Mo/No_Ta1/No_Ta2` are configurable, but
   `SNewu.pas:504`, `:522` hard-code padding to 3 and 4. Read the production `Base` row.
5. **`Base.IsActive` — who sets it?** It is read at `Dmu.pas:1008` to block all posting in an archived
   year, but **no screen in this repository writes it**. Is it maintained by hand in SQL? Is there a
   second application?
6. **Fiscal-year identity.** `EnteghalU` hard-codes "next year = `CO_ID + 1`" (`EnteghalU.pas:94`,
   `:287`). Is `CO_ID` guaranteed to be a dense ascending sequence? What happens with a short first
   year, or a re-opened year?

### Important — affects correctness

7. **`Sarfasl.S_Kind`** (§1.8) exists in the schema and is displayed by a dead unit, but nothing
   writes it. Was it ever populated in production data? If so, what do its values mean?
8. **`DMoein.DM_Atf`** ("عطف", folio) is displayed and printed but never written (§3.2). Is it
   populated by an external process? Should the rebuild generate it?
9. **`Sarfasl.S_Active`** is written as `'1'` on create and never read (§1.1). Dead, or is there an
   external consumer?
10. **`Sarfasl.S_Bed`, `S_Bes`, `S_Remi`, `S_Count`** are initialised to `'0'` and never maintained.
    Are they stale in production, and does any report read them?
11. **Is `M_Kind = 2` (journal) meant to share the voucher-number space with `M_Kind = 1`?** §8.1.
    `New_Sanad` reads `Moein` (all kinds), `MoeinToRU` reads `DMoein` (all kinds). Confirm.
12. **Which `M_Id` ranges are actually in use?** The classifier at `MergeSanad.pas:206-234` claims
    1–9 = sales, 11–19 = pistachio, 21–29 = treasury, but `MakeSanadU` writes 31–35 and the
    classifier has no branch for those. Query `SELECT DISTINCT M_Id FROM Moein` in production.
13. **`Moein.M_Ted`** (quantity) — is it used for anything beyond display? No report in the
    accounting core aggregates it.

### Defects found — confirm whether they are known and whether the data is affected

14. **Un-posting an inventory document never deletes `M_Id = 31` (opening-stock) lines** (§6.7).
    Check production for orphaned opening-stock lines and duplicate postings.
15. **`Is_Admin_Or_Valid_Sanad` denies access when the voucher header is missing** (§3.6), because
    `Result` is left at its initial `Admin` value. Non-admins cannot open `Moein`-only vouchers.
16. **`MergeSanad` does not update `TankhahMaster` or `FactorMaster.FM_SanadNo`** (§7.3). Check for
    dangling references.
17. **`Delete_Moein_ssn` does not refresh the header totals** (§4.6). `DM_TBed`/`DM_TBes`/`DM_Count`
    go stale after a single-line deletion from the legacy screen.
18. **`MoeinToRU` does not filter `M_Kind = 1`** when selecting the source range (§8.1), so a journal
    voucher inside the range is summarised again. `MakeRooznamehU` does filter. Check for
    double-counted journal vouchers.
19. **Journal generation is re-runnable with overlapping ranges** and nothing marks source vouchers as
    journalised (§8.1). Has this happened?
20. **`NewFinalu` validation 7** (destination must not be in the ticked list) fails open when the
    destination code has no `-` (§9.2).
21. **`EnteghalU` carries forward every account with a balance, including P&L accounts** (§9.3), with
    no enforcement that `NewFinalu` was run first. Verify the intended year-end procedure with the
    accountants.
22. **`EnteghalU` mutates the global `Dm.CO_ID`** to write the next year's header (§9.3). Not
    exception-safe.
23. **`MakeRooznamehU` reads `bigint` sums into 32-bit integers** (§8.2). Would fail on any realistic
    rial total. Is this unit ever used?
24. **`FinalU.pas`'s `Sum(M_Bes-M_Bes)` typo** (§9.4) made the dead unit produce zero credit balances.
    Confirm no production data came from it.
25. **`MakeSanadU.init22` appends grid rows before validating the account** (§6.5c), leaving empty
    lines in the buffer on error.
26. **No voucher-date-within-fiscal-year validation in `SanadEditU`** (§4.1). The check exists
    (`Dm.isValidDate`) but is not called. Are there out-of-range voucher dates in production?
27. **Account locks (`S_Lock`) are not enforced on the voucher-entry path** (§2.5). Only ledger and
    report screens call `Is_Admin_Or_Valid_Daftar`.
28. **`SNewu` enforces none of permissions 1102/1103/1104** (§13.2). Anyone who can open the chart of
    accounts can delete accounts.

### Unresolved semantics

29. **Two competing "special account" registries** (§1.9, §1.10): `base_config` (`BC_ID` 11–15) and
    `Base.C1081`/`C1082`. `BC_ID = 14`/`15` and `C1081`/`C1082` describe the same two concepts. Which
    does the treasury module actually read?
30. **`SahamdarConfig(SC_K, SC_M, SC_T)`** (§1.11) — is the "deepest code component = party card
    number" rule an invariant, or a convention that can break?
31. **`.GGS` file format** — is it still used operationally, or is it a legacy migration artefact?
32. **`SanadMoeinu` shows a hard-coded company name** in the `.dfm` (§12.16). Is this a single-tenant
    installation, or is multi-company support real?
33. **`FM_ID` values 14, 15, 16, 25, 26 have no voucher generator** (§6.3). Are production receipts
    and inter-warehouse transfers meant to be non-financial, or is this missing functionality?
34. **`Dm.MakeSanad_FishVariz` is an empty stub** (§6.1). Was deposit-slip posting ever implemented?
35. **The `Rem` (running balance) field** on `SanadEditU`'s `QS` dataset (`SanadEditU.pas:38`,
    `SanadEditU.dfm:930-933`) is declared but the dataset is never opened in the live path. Intended
    feature?
36. **Voucher line ordering** is not persisted (§5.3). Does the business rely on line order in printed
    vouchers?

---

_Prev: [03-13-permissions](03-13-permissions.md) | Next: [03-15-proposed-improvements-needs-user-approval](03-15-proposed-improvements-needs-user-approval.md)_
