_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

### 1.9 R16–R20 — voucher printing (`PrintMU`, `PrintM2U`, `PrintNu`, `SanadEditU`, `SanadViewU`)

Three interchangeable voucher-print layouts, selected by configuration rather than by the user:
`PrintMU` (`چاپ سند معين`), `PrintM2U` (`چاپ سند معين`), `PrintNu` (`چاپ سند`). Each is a thin form
whose only job is to load the voucher, push header/footer text into memos and call `ShowReport`.
`PrintM2U` and `PrintNu` each contain two `ShowReport` sites (portrait/landscape or with/without
analytic columns). Mechanics, page setup and the `TanzimChapu` settings that choose between them are
documented in **§6**; the voucher data model belongs to `03-accounting-core.md`.

`SanadEditU` (`نمایش سند معین`, key 1121) and `SanadViewU` (`نمايش اسناد`, keys 1127/1125/1126 for the
three states) are the voucher **editor** and the voucher **list by state**. Both own a `TfrxReport` and
both write extensively (`SanadViewU.pas:404-410` re-dates `DMoein`, `Moein`, `CheckMaster` and
`TankhahMaster` in one transaction). They are reports only incidentally; see `03-accounting-core.md`.

### 1.10 R21–R22 — chart-of-accounts listings (`ListSarfaslu`, `SNewu`)

- **`ListSarfaslu`** (`ليست سرفصلها`, two `ShowReport` sites) is **unreachable**: its only launcher,
  `ListSarfasl.init`, is commented out at `Mainu.pas:572`, replaced on the line above by
  `Snew.init` (`:570`). Dead; do not port.
- **`SNewu`** (`سرفصلهای حسابداری`) is the live chart-of-accounts browser, launched from
  `Mainu.pas:568-571` (menu `Sarfasl_List`, key 1101). It owns one `TfrxReport` for the printed
  account list and also performs account maintenance, so it **writes**. The account model is
  `03-accounting-core.md`'s; only its print path is in scope here (§6).

Note `Mainu.pas:575-580` — `Sarfasl_AddClick` begins with a bare `exit;` before `NewSarfasl.init`,
and `Mainu.pas:911` forces `Sarfasl_Add.Enabled := False` with the `IsEnabel(…,1102)` call commented
out. Account creation from the main menu is doubly disabled.

### 1.11 R24, R26–R37 — inventory and invoice reports

Owned by `05-inventory.md`. Catalogued in §1.1 with launcher, permission key and reachability. Two
findings belong here because they are *reporting* defects:

- **`Anbar_Amalkard` — a report that performs an unguarded `UPDATE`.** `Anbar_Amalkard.pas:168`,
  `:189` and `:215` each run `UPDATE Anbar_FactorD SET AFD_Customer = (…)` **with no `WHERE` clause**,
  rewriting a column on every row of the table each time the report is run. Confirmed by sibling
  agents. This is the most dangerous single line in the reporting surface and must not be ported.
- **`AnbarReportKharidU`** is **unreachable** — the only launcher, `AnbarReportKharid.init`, is
  commented out at `Mainu.pas:564` in favour of `Anbar_AmalkardF.init` (`:563`).

### 1.12 R38–R42 — treasury reports

Owned by `06-treasury.md`. `CheckListU` (issued cheques, key 2110), `CheckListDU` (received cheques,
key 2101), `CheckEditU` (cheque issue + print), `FishListD` (deposits, key 2115), `TankhahEdit` (petty
cash, reached via `TankhahList`, key 2120). All five own `TfrxReport` components and all five write —
they are transaction screens with a print button, not reports in the §2–§4 sense.

### 1.13 Dead and unreachable report units

| Unit | Persian | Why dead | Evidence |
|---|---|---|---|
| `S_KolU` | `سرفصلهای حسابداری` | superseded by `SNewu`; no call site | `Mainu.pas:570` calls `Snew.init` |
| `ListSarfaslu` | `ليست سرفصلها` | launcher commented out | `Mainu.pas:572` |
| `Report7U` | `رویت جامع حسابداری` | no call site anywhere | grep; `uses` at `Mainu.pas:284` only |
| `AnbarReportKharidU` | `گزارش ورود و خروج انبار` | launcher commented out | `Mainu.pas:564` |
| `ToExcelU` | — | form creation commented out in `arzi.dpr`; `SMoein5Click` body is two commented lines | `Mainu.pas:993-996` |
| `LibXL` | — | only consumer was `ToExcelU` | — |
| `RoozViewU` | — | no call site | — |
| `KolSatateU` | — | contains `procesdure`; never compiled | — |
| `Lab.pas` | `ورود اطلاعات انس گذاري` | no call site; two orphan `ShowReport` sites | — |
| `SarfaslChap.pas` | — | no call site | — |
| `Taraz4Setooni_U.RP_Kol1` | — | second report laid out but never printed | `Taraz4Setooni_U.pas:167-175` uses `RP_Kol` only |
| `DM.SP_Taraz4Setooni` | — | declared on the data module, never opened | `Dmu.dfm:34-84`; see §2 preamble |
| `Mainu` `_Report9` / `TajmiF.initM` | `مشاهده دفتر تجمیعی` | menu item has `Visible = False` | `Mainu.dfm:10721-10724` |
| `Mainu` `SRooz1/2/3/4` | journal voucher views and print | handler bodies entirely commented out | `Mainu.pas:475-488`, `:588-594` |

Also dead **within** live reports: `Taraz4Setooni_U`'s `B_Save` (no `OnClick`, `Enabled = False`) and
`SQ: TADOQuery` (§2.1); `CardJariU`'s `SP_Size` and its whole `POP_Size` font menu, and
`ADOConnection1` (§4.8); `DaftarT_U`'s `GridFontSizeChangingEx` body (`:290-291`); `RoyatJU`'s
`G1KeyPress` up-navigation, marked `// not work !!!!` by the author (`:152`).


---

[← SS1 Report catalogue (2/3)](04-01-b-report-catalogue.md) | [Index](00-index.md) | [SS2 Trial balances in depth (1/2) →](04-02-a-trial-balances-in-depth.md)
