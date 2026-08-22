# 05 — Inventory (Anbar) & Purchase/Sale Documents (Factor)

> Domain specification derived from the legacy Delphi/VCL "arzi" application.
> Logic reference only — naming, structure and style proposals are new.
> Sibling docs: `01-glossary.md`, `02-data-model.md`, `03-accounting-core.md`,
> `06-treasury.md`, `07-parties-and-shareholders.md`, `08-platform-and-security.md`,
> `09-unit-index.md`.

This document was split into per-section files under [`05-inventory/`](05-inventory/00-index.md)
for readability. Cross-references elsewhere in the docs of the form `05-inventory.md §N` refer to
the section numbers below, which are unchanged from the original single-file version — only the
file boundaries are new. Start at the [index](05-inventory/00-index.md).

## Table of contents

- **§1** [Entity model](05-inventory/05-01-entity-model.md)
- **§2** Item master CRUD rules
  - [2.1–2.3](05-inventory/05-02-a-item-master-crud-rules.md)
  - [2.4–2.7](05-inventory/05-02-b-item-master-crud-rules.md)
- **§3** Document types
  - [3.1](05-inventory/05-03-a-document-types.md)
  - [3.2–3.4](05-inventory/05-03-b-document-types.md)
- **§4** The invoice (Factor) lifecycle
  - [4.0–4.2.1](05-inventory/05-04-a-invoice-factor-lifecycle.md)
  - [4.2.2](05-inventory/05-04-b-invoice-factor-lifecycle.md)
  - [4.2.3–4.5](05-inventory/05-04-c-invoice-factor-lifecycle.md)
- **§5** Stock quantity mathematics
  - [5.0–5.1](05-inventory/05-05-a-stock-quantity-mathematics.md)
  - [5.2–5.4](05-inventory/05-05-b-stock-quantity-mathematics.md)
- **§6** Costing and valuation
  - [6.0–6.3](05-inventory/05-06-a-costing-and-valuation.md)
  - [6.4–6.8](05-inventory/05-06-b-costing-and-valuation.md)
- **§7** Pricing
  - [7.0–7.2](05-inventory/05-07-a-pricing.md)
  - [7.3–7.6](05-inventory/05-07-b-pricing.md)
- **§8** The Pesteh (pistachio) specialisation
  - [8.0–8.2](05-inventory/05-08-a-pesteh-pistachio-specialisation.md)
  - [8.3](05-inventory/05-08-b-pesteh-pistachio-specialisation.md)
  - [8.4–8.7](05-inventory/05-08-c-pesteh-pistachio-specialisation.md)
- **§9** Settlement (Tasfieh)
  - [9.0–9.3](05-inventory/05-09-a-settlement-tasfieh.md)
  - [9.4–9.7](05-inventory/05-09-b-settlement-tasfieh.md)
- **§10** Accounting integration
  - [10.0–10.1](05-inventory/05-10-a-accounting-integration.md)
  - [10.2–10.6](05-inventory/05-10-b-accounting-integration.md)
- **§11** [Stock card and stock balance](05-inventory/05-11-stock-card-and-balance.md)
- **§12** SQL and stored procedures
  - [12.0–12.4](05-inventory/05-12-a-sql-and-stored-procedures.md)
  - [12.5–12.7](05-inventory/05-12-b-sql-and-stored-procedures.md)
- **§13** Screen specifications
  - [13.0–13.6](05-inventory/05-13-a-screen-specifications.md)
  - [13.7–13.13](05-inventory/05-13-b-screen-specifications.md)
  - [13.14–13.20](05-inventory/05-13-c-screen-specifications.md)
- **§14** [Open questions](05-inventory/05-14-open-questions.md)
- **§15** [PROPOSED IMPROVEMENTS (needs user approval)](05-inventory/05-15-proposed-improvements.md)
- **§16** Naming map
  - [16.1–16.8](05-inventory/05-16-a-naming-map.md)
  - [16.9–16.13](05-inventory/05-16-b-naming-map.md)
