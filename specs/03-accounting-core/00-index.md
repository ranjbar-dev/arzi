# 03 — Accounting Core: Chart of Accounts and Vouchers (Sanad)

Specification extracted from the legacy Delphi/VCL application at `C:\Users\root\Desktop\arzi\arzi`.
This is a **behavioural specification for the Rust + React + PostgreSQL rebuild**. The Delphi code is
the source of truth for *logic only*; its naming, structure and UI idioms are not to be copied.

Read `docs/01-glossary.md` first. All proposed English names in this document come from there.

Every behavioural claim is cited as `file.pas:line`. Persian strings are quoted verbatim followed by
an English translation.

This document was split into per-section files for readability. Each file below carries the original
`## N. Title` heading verbatim; nothing was reworded or dropped in the split.

| Section | Title | File | Approx lines |
|---|---|---|---|
| 0 | Reading notes, encoding, and dead code | [03-00-reading-notes-encoding-dead-code.md](03-00-reading-notes-encoding-dead-code.md) | 68 |
| 1 | The account hierarchy model (1/2: §1.1–1.5) | [03-01-a-account-hierarchy-model.md](03-01-a-account-hierarchy-model.md) | 188 |
| 1 | The account hierarchy model (2/2: §1.6–1.11) | [03-01-b-account-hierarchy-model.md](03-01-b-account-hierarchy-model.md) | 226 |
| 2 | Account CRUD rules (1/2: §2.1–2.3) | [03-02-a-account-crud-rules.md](03-02-a-account-crud-rules.md) | 189 |
| 2 | Account CRUD rules (2/2: §2.4–2.6) | [03-02-b-account-crud-rules.md](03-02-b-account-crud-rules.md) | 130 |
| 3 | The voucher (Sanad) model (1/3: §3.1–3.4) | [03-03-a-voucher-sanad-model.md](03-03-a-voucher-sanad-model.md) | 154 |
| 3 | The voucher (Sanad) model (2/3: §3.5–3.6) | [03-03-b-voucher-sanad-model.md](03-03-b-voucher-sanad-model.md) | 249 |
| 3 | The voucher (Sanad) model (3/3: §3.7–3.9) | [03-03-c-voucher-sanad-model.md](03-03-c-voucher-sanad-model.md) | 128 |
| 4 | Voucher validation rules | [03-04-voucher-validation-rules.md](03-04-voucher-validation-rules.md) | 174 |
| 5 | Voucher line editing behaviour (1/2: §5.1–5.6) | [03-05-a-voucher-line-editing-behaviour.md](03-05-a-voucher-line-editing-behaviour.md) | 228 |
| 5 | Voucher line editing behaviour (2/2: §5.7–5.9) | [03-05-b-voucher-line-editing-behaviour.md](03-05-b-voucher-line-editing-behaviour.md) | 138 |
| 6 | Automatic voucher generation (1/2: §6.1–6.4) | [03-06-a-automatic-voucher-generation.md](03-06-a-automatic-voucher-generation.md) | 150 |
| 6 | Automatic voucher generation (2/2: §6.5–6.8) | [03-06-b-automatic-voucher-generation.md](03-06-b-automatic-voucher-generation.md) | 202 |
| 7 | Merging vouchers — `MergeSanad.pas` | [03-07-merging-vouchers-mergesanad-pas.md](03-07-merging-vouchers-mergesanad-pas.md) | 82 |
| 8 | Journal (Rooznameh) generation | [03-08-journal-rooznameh-generation.md](03-08-journal-rooznameh-generation.md) | 232 |
| 9 | Period close and year-end (1/3: intro, §9.1–9.2) | [03-09-a-period-close-and-year-end.md](03-09-a-period-close-and-year-end.md) | 216 |
| 9 | Period close and year-end (2/3: §9.3) | [03-09-b-period-close-and-year-end.md](03-09-b-period-close-and-year-end.md) | 223 |
| 9 | Period close and year-end (3/3: §9.4–9.7) | [03-09-c-period-close-and-year-end.md](03-09-c-period-close-and-year-end.md) | 157 |
| 10 | Aggregation / consolidation — `TajmiU.pas` | [03-10-aggregation-consolidation-tajmiu-pas.md](03-10-aggregation-consolidation-tajmiu-pas.md) | 91 |
| 11 | Index of all SQL in the accounting core | [03-11-index-of-all-sql-in-the-accounting-core.md](03-11-index-of-all-sql-in-the-accounting-core.md) | 247 |
| 12 | Screen-by-screen UI specification (1/3: §12.1–12.3) | [03-12-a-screen-by-screen-ui-specification.md](03-12-a-screen-by-screen-ui-specification.md) | 265 |
| 12 | Screen-by-screen UI specification (2/3: §12.4–12.7) | [03-12-b-screen-by-screen-ui-specification.md](03-12-b-screen-by-screen-ui-specification.md) | 178 |
| 12 | Screen-by-screen UI specification (3/3: §12.8–12.17) | [03-12-c-screen-by-screen-ui-specification.md](03-12-c-screen-by-screen-ui-specification.md) | 193 |
| 13 | Permissions | [03-13-permissions.md](03-13-permissions.md) | 192 |
| 14 | Open questions | [03-14-open-questions.md](03-14-open-questions.md) | 104 |
| 15 | PROPOSED IMPROVEMENTS (needs user approval) | [03-15-proposed-improvements-needs-user-approval.md](03-15-proposed-improvements-needs-user-approval.md) | 69 |
| 16 | Naming map | [03-16-naming-map.md](03-16-naming-map.md) | 239 |
