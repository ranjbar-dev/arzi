_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## 0. Executive summary of the domain (read this first)

Three findings reshape this domain relative to the brief, and everything below depends on them.

**Finding A — `CO_ID` is a *fiscal year*, not a tenant.**
`CO_ID` indexes the `Base` table, one row per **fiscal year** (`سال مالی`). Each row also carries the
operating entity's letterhead identity (name, tax ID, registration number, address, logo). There is
exactly one physical database; all fiscal years coexist in it and are separated by a `*_COID` column
stamped on every transactional table. Master data — chart of accounts (`Sarfasl`) and the person
register (`Sahamdar`) — carries **no** `CO_ID` and is therefore shared across all years.
Multi-company is *emulated* by creating additional `Base` rows with a different `Co_Name`; there is
no tenant isolation. See §1.

**Finding B — `TarafU` is not a counterparty master; it is an account-code picker.**
`TTaraf` (`طرف حساب` = "counterparty") is a modal 4-segment code entry widget
(Kol / Moein / Tafsil1 / Tafsil2) over the `Sarfasl` chart of accounts. It has no CRUD, no table of
its own, and no persisted state. A *counterparty* in this system **is a leaf node of the chart of
accounts**, optionally joined to a person record. See §2.

**Finding C — `Sahamdar` ("shareholder") is a misnomer: it is the person/legal-entity register, and
this codebase contains no equity, no share holdings and no profit distribution.**
The table holds natural persons (`S_Kind=1`, `اشخاص`) and legal entities (`S_Kind=2`, `شرکتها`).
There is no share count, no nominal value, no ownership percentage, no join/exit date, and no
profit-allocation code anywhere in the 200+ units of this project. Share registry lives in a
**separate application and database** (`Saham.Dbo`, files under `\\pesteh\SahamData\`), which `arzi`
only *reads* for display (`CardJariU.pas:304-324`). See §5 for the exhaustive derivation of this
absence.

Consequently the domain that actually needs porting is:

```
Base (fiscal year + entity identity)
  └─ scopes ─► Moein (journal lines), DMoein (voucher headers), Anbar_Factor, DCheck, …

Sarfasl (chart of accounts, 4 levels, NOT year-scoped)
  ├─ leaf node = a postable account = a counterparty when it sits under a control account
  ├─ extended party attributes (address, tel, fax, national ID, economic code, reg. no.)
  └─ S_Card ──────────────┐
                          │  (also: S_Ta1 or S_Ta2 == S_Card by construction)
Sahamdar (person / legal-entity register, NOT year-scoped)
  ├─ S_Card = business key ("card number" / شماره شناسايي)
  ├─ S_Kind = 1 natural person | 2 legal entity
  └─ S_Lock = current-account lock

SahamdarConfig (which control accounts a party card gets a detail account under)
SahamdarInfo   (bank accounts / cards / IBANs per party card)
Jari_Rem       (current-account balance rollup for one party card in one fiscal year)
```

---


---

[Index](00-index.md) · [Next →](07-01-a-company-multi-tenancy-model.md)
