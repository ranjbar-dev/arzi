_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 5. Shareholder equity and profit distribution — **derivation of absence**

The brief expects this to be the most business-specific logic in the domain. It is not present.
Because "absent" is a strong claim, here is the derivation rather than the assertion.

### 5.1 Search evidence

| Probe | Scope | Result |
|---|---|---|
| Persian `سود` (profit) | all `*.pas` | 0 hits |
| Persian `زیان` / `زيان` (loss) | all `*.pas`, all `*.dfm` | 0 hits |
| Persian `سرمایه` / `سرمايه` (capital) | all `*.pas` | 1 hit — `Utility.pas:151`, a **bank name** (`بانک سرمایه`, Sarmayeh Bank) inside a BIN→bank lookup table |
| Persian `درصد` (percent) | all `*.pas` | 0 hits |
| Persian `سهام` (shares) | all `*.pas` | 3 hits, all in `CardJariU.pas:240,321,323` and all referring to *the external Saham application*, not to holdings |
| Persian `سهام` in `*.dfm` | all | `Mainu.dfm:4404` (`'5- حقوق صاحبان سهام'` = "5- Owners' equity", an item in an inert `TMemo` listing Kol-level account groups) and `SahamdarP.dfm:5,396,397` (the dead form) |
| Identifiers `Sood`, `Zian`, `Sarmaye`, `Profit`, `Dividend`, `Equity`, `Share`, `Percent` | all `*.pas` | 0 hits in a business sense |
| Column names `N_*` from `Saham.Dbo.NSaham` actually read | `CardJariU.pas:315-319` | `N_Name`, `N_Famil`, `N_Father`, `N_Mobile`, `N_CodeMelli` — **five identity fields, zero quantitative fields** |

### 5.2 Structural evidence

* `Sahamdar` has no numeric column other than `S_Card`, `S_IDNO`, `S_Kind`, `S_MaliatState`,
  `S_Lock` (§4.1). There is nowhere to store a share count or a nominal value.
* `SahamdarConfig` (§7) contains only chart-of-accounts coordinates and boolean flags.
* `SahamdarInfo` (§6.5) contains four free-text columns for bank details.
* No table named `Saham*` exists inside the `arzi` database; the only `Saham` reference is the
  cross-database qualifier `Saham.Dbo` (`Dmu.pas:758`).
* `Sahamdar_Show` (`Dmu.dfm:568-590`, `Dmu.pas:37`) — a stored procedure taking `@Id` — is
  **declared and never called** from any unit. It is the only plausible remnant of an equity feature.

### 5.3 What exists instead

The only shareholder-adjacent arithmetic in the codebase is the **current-account balance** of a
party card (`Jari_Rem`, §6.2) and the **year-end carry-forward** postings (§1.6). Both are documented
exhaustively with worked arithmetic in this document (§6.3, §1.6).

### 5.4 Conclusion for the rebuild

Profit/loss allocation among shareholders is **out of the `arzi` boundary**. Either:

1. the external `Saham` product owns it (most likely — it owns the share register, the certificates
   and the scanned documents), or
2. it is performed manually by an accountant posting ordinary vouchers to the `حقوق صاحبان سهام`
   ("Owners' equity") Kol group.

**Do not invent an allocation formula.** Port the current-account model exactly, and treat share
holdings as a future integration. This must be confirmed with the user before any equity feature is
designed — §12-Q13.

---


---

[← Previous](07-04-b-person-legal-entity-sahamdar-model.md) · [Index](00-index.md) · [Next →](07-06-a-party-current-account-jari.md)
