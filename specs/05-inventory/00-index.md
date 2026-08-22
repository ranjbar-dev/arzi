# 05 — Inventory (Anbar) & Purchase/Sale Documents (Factor)

> Domain specification derived from the legacy Delphi/VCL "arzi" application.
> Logic reference only — naming, structure and style proposals are new.
> Sibling docs: `01-glossary.md`, `02-data-model.md`, `03-accounting-core.md`,
> `06-treasury.md`, `07-parties-and-shareholders.md`, `08-platform-and-security.md`,
> `09-unit-index.md`.

This document was split into per-section files for readability. Cross-references elsewhere in
the docs of the form `05-inventory.md §N` refer to the section numbers below, which are unchanged
from the original single-file version.

| Section | Title | File | Approx lines |
|---|---|---|---|
| 1 | Entity model | [05-01-entity-model.md](05-01-entity-model.md) | 251 |
| 2 (a) | Item master CRUD rules — §2.1–§2.3 | [05-02-a-item-master-crud-rules.md](05-02-a-item-master-crud-rules.md) | 224 |
| 2 (b) | Item master CRUD rules — §2.4–§2.7 | [05-02-b-item-master-crud-rules.md](05-02-b-item-master-crud-rules.md) | 160 |
| 3 (a) | Document types — §3.1 | [05-03-a-document-types.md](05-03-a-document-types.md) | 125 |
| 3 (b) | Document types — §3.2–§3.4 | [05-03-b-document-types.md](05-03-b-document-types.md) | 218 |
| 4 (a) | The invoice (Factor) lifecycle — §4.0–§4.2.1 | [05-04-a-invoice-factor-lifecycle.md](05-04-a-invoice-factor-lifecycle.md) | 124 |
| 4 (b) | The invoice (Factor) lifecycle — §4.2.2 | [05-04-b-invoice-factor-lifecycle.md](05-04-b-invoice-factor-lifecycle.md) | 124 |
| 4 (c) | The invoice (Factor) lifecycle — §4.2.3–§4.5 | [05-04-c-invoice-factor-lifecycle.md](05-04-c-invoice-factor-lifecycle.md) | 249 |
| 5 (a) | Stock quantity mathematics — §5.0–§5.1 | [05-05-a-stock-quantity-mathematics.md](05-05-a-stock-quantity-mathematics.md) | 245 |
| 5 (b) | Stock quantity mathematics — §5.2–§5.4 | [05-05-b-stock-quantity-mathematics.md](05-05-b-stock-quantity-mathematics.md) | 215 |
| 6 (a) | Costing and valuation — §6.0–§6.3 | [05-06-a-costing-and-valuation.md](05-06-a-costing-and-valuation.md) | 195 |
| 6 (b) | Costing and valuation — §6.4–§6.8 | [05-06-b-costing-and-valuation.md](05-06-b-costing-and-valuation.md) | 150 |
| 7 (a) | Pricing — §7.0–§7.2 | [05-07-a-pricing.md](05-07-a-pricing.md) | 123 |
| 7 (b) | Pricing — §7.3–§7.6 | [05-07-b-pricing.md](05-07-b-pricing.md) | 187 |
| 8 (a) | The Pesteh (pistachio) specialisation — §8.0–§8.2 | [05-08-a-pesteh-pistachio-specialisation.md](05-08-a-pesteh-pistachio-specialisation.md) | 247 |
| 8 (b) | The Pesteh (pistachio) specialisation — §8.3 | [05-08-b-pesteh-pistachio-specialisation.md](05-08-b-pesteh-pistachio-specialisation.md) | 237 |
| 8 (c) | The Pesteh (pistachio) specialisation — §8.4–§8.7 | [05-08-c-pesteh-pistachio-specialisation.md](05-08-c-pesteh-pistachio-specialisation.md) | 203 |
| 9 (a) | Settlement (Tasfieh) — §9.0–§9.3 | [05-09-a-settlement-tasfieh.md](05-09-a-settlement-tasfieh.md) | 136 |
| 9 (b) | Settlement (Tasfieh) — §9.4–§9.7 | [05-09-b-settlement-tasfieh.md](05-09-b-settlement-tasfieh.md) | 179 |
| 10 (a) | Accounting integration — §10.0–§10.1 | [05-10-a-accounting-integration.md](05-10-a-accounting-integration.md) | 102 |
| 10 (b) | Accounting integration — §10.2–§10.6 | [05-10-b-accounting-integration.md](05-10-b-accounting-integration.md) | 234 |
| 11 | Stock card and stock balance | [05-11-stock-card-and-balance.md](05-11-stock-card-and-balance.md) | 291 |
| 12 (a) | SQL and stored procedures — §12.0–§12.4 | [05-12-a-sql-and-stored-procedures.md](05-12-a-sql-and-stored-procedures.md) | 168 |
| 12 (b) | SQL and stored procedures — §12.5–§12.7 | [05-12-b-sql-and-stored-procedures.md](05-12-b-sql-and-stored-procedures.md) | 162 |
| 13 (a) | Screen specifications — §13.0–§13.6 | [05-13-a-screen-specifications.md](05-13-a-screen-specifications.md) | 233 |
| 13 (b) | Screen specifications — §13.7–§13.13 | [05-13-b-screen-specifications.md](05-13-b-screen-specifications.md) | 222 |
| 13 (c) | Screen specifications — §13.14–§13.20 | [05-13-c-screen-specifications.md](05-13-c-screen-specifications.md) | 153 |
| 14 | Open questions | [05-14-open-questions.md](05-14-open-questions.md) | 81 |
| 15 | PROPOSED IMPROVEMENTS (needs user approval) | [05-15-proposed-improvements.md](05-15-proposed-improvements.md) | 251 |
| 16 (a) | Naming map — §16.1–§16.8 | [05-16-a-naming-map.md](05-16-a-naming-map.md) | 191 |
| 16 (b) | Naming map — §16.9–§16.13 | [05-16-b-naming-map.md](05-16-b-naming-map.md) | 120 |

Sections 2, 3, 4, 5, 6, 7, 8, 9, 10, 12, 13 and 16 exceeded ~300 lines in the original and were
further split at their `###` (or `####`) subheadings; the section numbering (`§N.x`) is unchanged,
only the file boundaries are new.
