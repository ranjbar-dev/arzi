# 08 — Platform & Security

**Scope:** application shell, startup, authentication, authorization, settings, licensing,
backup/restore/new-company/import, cross-cutting utilities, shared dialogs, concurrency.

**Source of truth:** the Delphi/VCL project at `C:\Users\root\Desktop\arzi\arzi`
(`arzi.dpr`, `Mainu.pas/.dfm`, `Dmu.pas/.dfm`, `Admin.pas/.dfm`, `INI.pas`, `LockUnit.pas`,
`testmainU.pas`, `Utility.pas`, …).

**Reading rule for this document:** the legacy code is a *behaviour* reference only.
Naming, structure and — emphatically — the security design are **not** to be copied.
Every place where the legacy behaviour must **not** be reproduced is flagged
`⛔ DO NOT PORT`. Proposals for replacements live in the final section and require approval.

**Product identity (from `Mainu.dfm`):**

| Field | Value | Line |
|---|---|---|
| Window caption | `حسابداري ارزي` — "Arzi Accounting" | `Mainu.dfm:6` |
| Product Name | `Green Gold` | `Mainu.dfm:4222` |
| Version | `1.06` | `Mainu.dfm:4236` |
| Last Update | `1405/05/01` (Jalali) | `Mainu.dfm:4194` |
| Contact | `MohsenRanjbar.1350@gmail.com` | `Mainu.dfm:4208` |
| Title bar text | `سیستم حسابداری` — "Accounting System" | `Mainu.dfm:10844` |

---

## Sections

| Section | Title | File | Approx lines |
|---|---|---|---|
| 1 | The complete main-menu tree | [08-01-the-complete-main-menu-tree.md](08-01-the-complete-main-menu-tree.md) | 227 |
| 2 | Application startup sequence | [08-02-application-startup-sequence.md](08-02-application-startup-sequence.md) | 80 |
| 3 | Authentication | [08-03-authentication.md](08-03-authentication.md) | 126 |
| 4 | Authorization | [08-04-authorization.md](08-04-authorization.md) | 260 |
| 5 | Audit trail / change log | [08-05-audit-trail-change-log.md](08-05-audit-trail-change-log.md) | 55 |
| 6 | Settings | [08-06-settings.md](08-06-settings.md) | 161 |
| 7 | Licensing / copy protection | [08-07-licensing-copy-protection.md](08-07-licensing-copy-protection.md) | 91 |
| 8 | Backup / restore / new company / import | [08-08-backup-restore-new-company-import.md](08-08-backup-restore-new-company-import.md) | 124 |
| 9 | `Utility.pas` function reference | [08-09-utility-pas-function-reference.md](08-09-utility-pas-function-reference.md) | 233 |
| 10 | Shared dialog / frame catalogue | [08-10-shared-dialog-frame-catalogue.md](08-10-shared-dialog-frame-catalogue.md) | 32 |
| 11 | Concurrency and multi-user behaviour | [08-11-concurrency-and-multi-user-behaviour.md](08-11-concurrency-and-multi-user-behaviour.md) | 88 |
| 12 | What `test.dpr` / `testmainU.pas` is | [08-12-what-test-dpr-testmainu-pas-is.md](08-12-what-test-dpr-testmainu-pas-is.md) | 47 |
| 13 | Open questions | [08-13-open-questions.md](08-13-open-questions.md) | 93 |
| — | PROPOSED IMPROVEMENTS (needs user approval) | [08-14-proposed-improvements.md](08-14-proposed-improvements.md) | 109 |
