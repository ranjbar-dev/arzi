_Part of [07-parties-and-shareholders](../07-parties-and-shareholders.md) — [index](00-index.md)_

## PROPOSED IMPROVEMENTS (needs user approval)

> Everything above is a description of existing behaviour and the default plan is **port as-is**.
> The items below are *suggestions only*; none should be implemented without explicit approval.

**I1 — Make `SC_Tik` a computed value, not a stored column.**
Today every open of a party editor runs `UPDATE SahamdarConfig SET SC_Tik = 0` followed by a
correlated `UPDATE … SET SC_Tik = 1` across the whole table (§7.4). Under concurrent use this
corrupts what each user sees. In the rebuild, compute "does this party already have an account
here?" per request with a `LEFT JOIN` / `EXISTS`, and drop the column. *No behaviour change for a
single user; fixes a real multi-user defect.*

**I2 — Split `Base` into `organization` and `fiscal_period`.**
The entity's legal identity (name, tax IDs, address, logo) is duplicated across every fiscal-year row
and is silently cloned by `MakeNewU.pas:117-118`. Normalising it makes multi-company genuinely
possible later without changing the period model. *Requires a data migration; behaviour-preserving.*

**I3 — Remove client-side file-system concerns from the domain model.**
`BackupDir`, `Saham_F` (`\\pesteh\SahamData\`), the hard-coded `D:\Bed.GGS` / `D:\Bes.GGS`
(`BastanHesab.pas:45-46`) and `D:\BACKUP\*.ini` (`Dmu.pas:711`) are desktop artifacts. In a web
rebuild these become object storage, download endpoints, and server config.

**I4 — Bind parameters everywhere.**
Every query in this domain is string-concatenated (`SahamdarEditU.pas:251,263,290-312`,
`SahamdarU.pas:149,170`, `Dmu.pas:975,1412,1423`, `EnteghalU.pas` throughout). Rust + `sqlx` should
use bound parameters exclusively. *Pure implementation change; no behaviour change.*

**I5 — Implement the missing CRUD that the legacy UI advertises but does not deliver:**
party delete/deactivate (`SahamdarU.dfm:422-437` — visible button, no handler) and bank-account
create/edit/delete (`SahamdarInfoU.dfm:162-191` — three visible buttons, no handlers). Deletion
should be a soft *deactivate* guarded by "has journal entries".

**I6 — Wire the existing IBAN and card validators into the party bank-account form.**
`Dmu.pas:196-214` (`IsValidShaba`, IR + mod-97) and `Dmu.pas:216-240` (`IsValidKart`, Luhn) are
already written and correct, but nothing calls them. Add an Iranian national-ID checksum too.

**I7 — Unify the two account-code string formats** (§12-Q9) behind one canonical representation, with
the display format driven by the configured segment widths.

**I8 — Replace the `Kol/Moein/Tafsil1/Tafsil2` fixed 4-level model with a proper account tree**
(`parent_id` + materialised path). The current model already carries `M_L`/`M_R` nested-set columns
(`Dmu.pas:274-278`), so the intent existed. This would let a party live at any depth uniformly and
would make §12-Q1 moot. *Large change; only worth it if the 4-level cap is a known pain point.*

**I9 — Model party↔account linkage as one explicit table** (`party_id`, `account_id`,
`control_account_config_id`) instead of relying on "the Tafsil code happens to equal the card
number". That convention silently constrains card numbers to the Tafsil code space and makes
renumbering impossible.

**I10 — Add an explicit `party_role` classification** (customer / supplier / employee / tenant /
shareholder / other) alongside — not replacing — the positional classification. Today a party's role
is inferable only from which control accounts it happens to have (§7.3), which makes simple questions
like "list all suppliers" require a chart-of-accounts join.

**I11 — Fix the Cancel button on the fiscal-year switcher** (`ChangesU.pas:76-81`), which currently
applies the change. *Behaviour change — needs explicit sign-off.*

**I12 — Surface the person/share-register sync state as structured data** rather than the two magic
strings written into a name field (`CardJariU.pas:297`, `:321-322`).

**I13 — Replace the hard-coded `Dm.userId = 68` gate** (`SahamdarU.pas:101`) and the hard-coded
`M_User = 68` rollover stamp (`EnteghalU.pas:~253`) with real permissions and the acting user.

**I14 — Add audit columns** (`created_at`, `created_by`, `updated_at`, `updated_by`) to `party`,
`account`, and `party_bank_account`. The legacy tables have none, so no history of party changes
exists at all.

---

[← Previous](07-12-open-questions.md) · [Index](00-index.md)
