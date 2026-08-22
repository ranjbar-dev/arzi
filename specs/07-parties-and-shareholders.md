# 07 — Business Parties, Persons/Companies Register, and Company (Fiscal-Entity) Model

**Scope of this document (domain ownership).**
This is the reference specification for the *Business parties and shareholders* domain of the legacy
Delphi/VCL application `arzi`:

* `TarafU.pas` / `.dfm` — the account-code (counterparty) picker.
* `SahamdarU.pas`, `SahamdarEditU.pas`, `CompanyEditU.pas`, `SahamdarInfoU.pas`, `SahamdarP.pas` — the
  person / legal-entity register and its editors.
* `CompanyEditU.pas` — **legal-entity counterparty editor** (see the warning in §1.0; it is *not*
  the operating-company setup form).
* `Dmu.pas` / `Dmu.dfm` — `CO_ID`, `Base`, `Sahamdar*`, `SahamdarConfig`, `Jari_Rem`, `Saham_DB`,
  `Saham_F`.
* `TanzimU.pas`, `MakeNewU.pas`, `ChangesU.pas`, `EnteghalU.pas` — the actual company/fiscal-year
  identity, creation, selection and rollover (these turned out to be where the "company" model
  really lives).
* `Sarfasl_TakmilU.pas`, `SelectSarfasl.pas`, `SNewu.pas` — the counterparty attributes that live on
  the chart-of-accounts node, and the account-creation stored procedure.

**Explicitly out of scope / owned by another agent:** the reporting internals of `CardJariU.pas`
(the ledger grid, the `TMoeinF` / `DMoeinF` drill-downs, print layouts, `RoyatJU.pas`,
`DaftarT_U.pas`). This document covers **only** the party-linkage aspects of `CardJariU.pas`:
how a party card is resolved to a person record, to a set of chart-of-accounts nodes, and to a
current-account balance. Everything about how those rows are then *rendered or printed* is
deliberately not documented here.

> **The old code is a logic reference only.** All naming proposals below are new English names; the
> legacy identifiers are preserved only in the naming map (§11) and in verbatim quotes.

---

**This document has been split into per-section files** under
[`07-parties-and-shareholders/`](07-parties-and-shareholders/00-index.md) for readability. No content
was changed — only the file boundaries. Section references elsewhere in the form
`07-parties-and-shareholders.md §N` map to the files below. Start at the
[index](07-parties-and-shareholders/00-index.md).

## Table of contents

### §0 — Executive summary
* [07-00-executive-summary.md](07-parties-and-shareholders/07-00-executive-summary.md)

### §1 — Company / multi-tenancy model
* [07-01-a-company-multi-tenancy-model.md](07-parties-and-shareholders/07-01-a-company-multi-tenancy-model.md) (§1.0–1.4)
* [07-01-b-company-multi-tenancy-model.md](07-parties-and-shareholders/07-01-b-company-multi-tenancy-model.md) (§1.5–1.7)

### §2 — Counterparty (Taraf) model
* [07-02-a-counterparty-taraf-model.md](07-parties-and-shareholders/07-02-a-counterparty-taraf-model.md) (§2.1–2.2)
* [07-02-b-counterparty-taraf-model.md](07-parties-and-shareholders/07-02-b-counterparty-taraf-model.md) (§2.3–2.7)

### §3 — Counterparty / person CRUD validations — exhaustive
* [07-03-counterparty-person-crud-validations.md](07-parties-and-shareholders/07-03-counterparty-person-crud-validations.md)

### §4 — Person / legal-entity ("Sahamdar") model
* [07-04-a-person-legal-entity-sahamdar-model.md](07-parties-and-shareholders/07-04-a-person-legal-entity-sahamdar-model.md) (§4.1–4.3)
* [07-04-b-person-legal-entity-sahamdar-model.md](07-parties-and-shareholders/07-04-b-person-legal-entity-sahamdar-model.md) (§4.4–4.6)

### §5 — Shareholder equity and profit distribution — derivation of absence
* [07-05-shareholder-equity-profit-distribution.md](07-parties-and-shareholders/07-05-shareholder-equity-profit-distribution.md)

### §6 — Party current account (Jari)
* [07-06-a-party-current-account-jari.md](07-parties-and-shareholders/07-06-a-party-current-account-jari.md) (§6.1–6.3)
* [07-06-b-party-current-account-jari.md](07-parties-and-shareholders/07-06-b-party-current-account-jari.md) (§6.4–6.6)

### §7 — `SahamdarConfig` — party account configuration
* [07-07-sahamdarconfig-party-account-configuration.md](07-parties-and-shareholders/07-07-sahamdarconfig-party-account-configuration.md)

### §8 — Accounting integration
* [07-08-accounting-integration.md](07-parties-and-shareholders/07-08-accounting-integration.md)

### §9 — SQL and stored procedures — verbatim catalogue
* [07-09-sql-and-stored-procedures.md](07-parties-and-shareholders/07-09-sql-and-stored-procedures.md)

### §10 — Screen-by-screen UI specification for the React rebuild
* [07-10-screen-by-screen-ui-specification.md](07-parties-and-shareholders/07-10-screen-by-screen-ui-specification.md)

### §11 — Naming map
* [07-11-naming-map.md](07-parties-and-shareholders/07-11-naming-map.md)

### §12 — Open questions
* [07-12-open-questions.md](07-parties-and-shareholders/07-12-open-questions.md)

### PROPOSED IMPROVEMENTS (needs user approval)
* [07-13-proposed-improvements.md](07-parties-and-shareholders/07-13-proposed-improvements.md)
