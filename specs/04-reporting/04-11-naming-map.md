_Part of [04-reporting](../04-reporting.md) — [index](00-index.md)_

## 11. Naming map

Conventions are those fixed in `01-glossary.md` §7: `snake_case` in Postgres and Rust, `camelCase` in
the API payload and React, Persian only in locale files. Where `02-data-model.md` already proposes a
name for a table or column, that name is reused verbatim and marked *(02)*.

### 11.1 Report identifiers — legacy unit → proposed English name

| Legacy unit / form | Persian title | Proposed report id | Proposed React route |
|---|---|---|---|
| `Taraz4Setooni_U` / `TTaraz4Setooni` | `تراز آزمايشي 4 ستوني` | `trial_balance_4col` | `/reports/trial-balance/4-column` |
| `Taraz6SetooniU` / `TTaraz6Setooni` | `تراز آزمایشی 6 ستونی` | `trial_balance_6col` | `/reports/trial-balance/6-column` |
| `DKolU` / `TDKolF` | `نمایش دفتر کل` | `general_ledger` | `/reports/general-ledger` |
| `DMoein` / `TDMoeinF` | `نمایش دفتر معین` | `subsidiary_ledger` | `/reports/subsidiary-ledger` |
| `TMoein` / `TTMoeinF` | `مشاهده دفتر معین تجمیعی` | `subsidiary_ledger_consolidated` | *(merged into `subsidiary_ledger`)* |
| `DaftarT_U` / `TDaftarT_F` | `دفتر تجمیعی` | `ledger_multi_account` | `/reports/ledger/multi-account` |
| `KolStateU` / `TKolState` | `وضعیت حسابهای کل` | `account_monthly_turnover` | `/reports/account-monthly-turnover` |
| `CardJariU` / `TCardJariF` | `فرم خلاصه اطلاعات جاری اشخاص` | `party_account_summary` | `/parties/:cardNo/summary` |
| `BedBes` / `TBedBesF` | `بدهکاران و بستانکاران` | `party_balance_list` | `/reports/party-balances` |
| `Report6U` / `TReport6F` | `کنترل شماره اسناد` | `voucher_number_gap_check` | `/reports/voucher-number-check` |
| `RoyatJU` / `TRoyatJF` | `رویت جامع اسناد معین` | `account_turnover_explorer` | `/reports/account-turnover` |
| `Report7U` / `TReport7F` | `رویت جامع حسابداری` | *(dead — do not port)* | — |
| `MoeinZipU` / `TMoeinZip` | `خلاصه اسناد معین` | `voucher_summary` | `/reports/voucher-summary` |
| `RooznamehViewU` / `TRooznamehView` | `اسناد روزنامه` | *(dropped — journal generation never built)* | — |
| `MoeinToRU` / `TMoeinToR` | `تبدیل اسناد معین به روزنامه` | *(dropped — journal generation never built)* | — |
| `MakeRooznamehU` / `TMakeRooznameh` | (no caption) | *(dropped — journal generation never built)* | — |
| `PrintMU` / `PrintM2U` / `PrintNu` | `چاپ سند معين` / `چاپ سند` | `voucher_print` (one endpoint, three templates) | `/vouchers/:id/print` |
| `SNewu` / `TSnew` | `سرفصلهای حسابداری` | `chart_of_accounts` | `/accounts` |
| `ListSarfaslu` / `S_KolU` | `ليست سرفصلها` | *(dead — do not port)* | — |
| `ToExcelDaraeiU` / `TToExcelDaraei` | `ذخیره سند در فایل اکسل` | `voucher_export_tax_authority` | `/exports/tax-authority` |
| `TanzimChapu` / `TTanzimChap` | (print settings) | `print_settings` | `/settings/printing` |

### 11.2 Result-set columns — legacy field → proposed name

**Ledger result set** (`DKolU` / `DMoein` / `TMoein` `Qs`):

| Legacy | Persian caption | Proposed API field | Proposed Postgres | Notes |
|---|---|---|---|---|
| `RN` | — | `sequence_no` | *(computed)* | ordering key, not persisted |
| `M_Date` | `تاریخ` | `entryDate` / `entry_date_jalali` | `entry_date` + `entry_date_jalali` *(02)* | |
| `M_Sanad` | `سند` | `voucherNo` | `voucher_no` *(02)* | `0` on the opening row |
| `Article` | `شرح` | `description` | `description` *(02)* | |
| `M_Bed` | `بدهکار` / `مبلغ بدهکار` | `debit` | `debit` *(02)* | |
| `M_Bes` | `بستانکار` / `مبلغ بستانکار` | `credit` | `credit` *(02)* | |
| `Rem` | `مانده` | `runningBalance` | *(computed)* | credit-positive in legacy; **emit signed + `side`** |
| *(script `D2`)* | `تش` | `side` | *(computed)* | `بس`→`Credit`, `بد`→`Debit`, blank→`Zero` |
| `M_Ted` | `مقدار` | `quantity` | `quantity` *(02)* | |
| `M_Tx` | `وضعیت` | `voucherState` | `state` *(02)* | 0/1/2 → `Draft`/`Confirmed`/`Posted` |
| `M_Id` | `صادر کننده` | `issuedByUserId` | `created_by` *(02)* | |
| `'مانده از قبل '` | — | `isOpeningRow: true` | *(computed)* | do not ship a magic description string |

**4-column trial balance** (`#R`, §2.1) — semantics corrected, Persian captions preserved:

| Legacy | Persian caption | Proposed API field | Notes |
|---|---|---|---|
| `St` | — | `level` | 1=Kol, 2=Moein, 3=Tafsil1, 4=Tafsil2 |
| `CodeStr` | `کد` | `accountCode` | from `Dbo.Make_R`; replace with a canonical formatter |
| `K` / `M` / `T1` / `T2` | — | `kol` / `moein` / `tafsil1` / `tafsil2` | |
| `Name` | `نام` | `accountName` | strip the fake indentation spaces; send `level` instead |
| `TBed` | `گردش بدهکار` | **`cumulativeDebit`** | caption says turnover; it is inception-to-date (§2.1) |
| `TBes` | `گردش بستانکار` | **`cumulativeCredit`** | |
| `RBed` | `مانده بدهکار` | `balanceDebit` | clamped unsigned |
| `RBes` | `مانده بستانکار` | `balanceCredit` | clamped unsigned |

**6-column trial balance** (`Taraz_6Sotooni`, §2.2) — note the legacy component names are shifted by
one relative to the fields; port the **fields**, never the component names:

| Legacy field | Persian caption | Proposed API field |
|---|---|---|
| `S_Ko`/`S_Mo`/`S_Ta1`/`S_Ta2` | `کد` | `kol`/`moein`/`tafsil1`/`tafsil2` (+ `accountCode`) |
| `S_name` | `نام کد` | `accountName` |
| `Bed1` | `گردش قبل از دوره` / `گردش بدهکار` | `openingTurnoverDebit` |
| `Bes1` | `گردش قبل از دوره` / `گردش بستانکار` | `openingTurnoverCredit` |
| `Bed2` | `گردش طی دوره` / `گردش بدهکار` | `periodTurnoverDebit` |
| `Bes2` | `گردش طی دوره` / `گردش بستانکار` | `periodTurnoverCredit` |
| `RBed` | `مانده پایان دوره` / `مانده بدهکار` | `closingBalanceDebit` |
| `RBes` | `مانده پایان دوره` / `مانده بستانکار` | `closingBalanceCredit` |
| *(memo names `Bed`,`Bes`,`Bed1`,`Bes1`,`BedT`,`BesT`,`TBed`,`TBes`)* | — | **do not port** |

**Party account summary** (`CardJariU.VT1`, §4):

| Legacy | Persian caption | Proposed API field |
|---|---|---|
| `S_SSN` | — | `rowNo` |
| `S_Ko`/`S_Mo`/`S_Ta1`/`S_Ta2` | — | `accountId` (resolve to the real key) |
| `S_R` (from `Sarfasl.M_R`) | `کد حساب` | `accountCode` — **stale in legacy; recompute** |
| `S_Name` (from `Sarfasl.LineName`) | `نام حساب` | `accountName` |
| `G_Bed` | `گردش بدهکار` | `ytdDebit` |
| `G_Bes` | `گردش بستانکار` | `ytdCredit` |
| `R_Bed` | `مانده بدهکار` | `balanceDebit` |
| `R_Bes` | `مانده بستانکار` | `balanceCredit` |
| `S_Rem` ← `Jari_Rem.Remind` | `مــانده نهایی` | `finalBalance` (restricted to `SC_Rem = 1` accounts) |
| `T_Rem` caption `بدهکار` | — | `finalBalanceSide` |

**Party balance list** (`BedBes.Q1`, §1.2):

| Legacy | Persian caption | Proposed API field |
|---|---|---|
| `Jari` | `جاری` | `partyCardNo` |
| `S_Name` | `نام` / `مشخصات` | `accountName` |
| `Rem1` | `اول دوره` | `openingBalance` |
| `GBed` | `گردش بدهکار` | `periodDebit` |
| `GBes` | `گردش بستانکار` | `periodCredit` |
| `Rem2` | `مانده نهایی` | `closingBalance` |
| `@BedBes` | `لیست بدهکاران` / `لیست بستانکاران` | `side: Debtors \| Creditors` |
| `@GType` | `همه موارد` / `با گردش` / `بدون گردش` | `movement: All \| WithMovement \| WithoutMovement` |
| `@M1` / `@M2` | `از مبلغ` / `تا مبلغ` | `amountMin` / `amountMax` |

**Account turnover explorer** (`RoyatJU.temp_RJ_<uid>`, §1.4):

| Legacy | Persian caption | Proposed API field |
|---|---|---|
| `IsLast` | `سطح آخر` | `isLeaf` |
| `M_L` | `کد حساب` | `accountCode` |
| `M_Name` | `نام حساب` | `accountName` |
| `SS_Ko`/`SS_Mo`/`SS_Ta1`/`SS_Ta2` | — | `kolName`/`moeinName`/`tafsil1Name`/`tafsil2Name` |
| `TBed` / `TBes` | `گردش بدهکار` / `گردش بستانکار` | `turnoverDebit` / `turnoverCredit` |
| `RBed` / `RBes` | `مانده بدهکار` / `مانده بستانکار` | `balanceDebit` / `balanceCredit` |
| `_R1.._R4` | `همه اسناد این دوره` / `بر حسب شماره سند` / `بر حسب تاریخ` / `حسابهای یک جاری` | `scope: AllPeriod \| VoucherRange \| DateRange \| Party` |
| `_V1.._V4` | `نمایش بر حسب سطح حساب` / `نمایش آخرین سطح` / `نمایش سطح کل و معین` / `نمایش تمام سطوح` | `display: Drilldown \| LeavesOnly \| KolAndMoein \| AllLevels` |
| `temp_RJ_<userId>` | — | *(no table — a query or materialised view; see §8)* |

**Monthly turnover** (`KolState;1`, §3.5): `Sal` → `fiscalYear`, `mahstr` → `monthName`
(`ماه`), `M_Bed` → `turnoverDebit` (`گردش بدهکار`), `M_Bes` → `turnoverCredit` (`گردش بستانکار`).

**Voucher-number gap check** (`Report6U`, §1.3): `M_Sanad` → `voucherNo`, `M_Date` → `entryDate`,
`M_Bed`/`M_Bes` → `totalDebit`/`totalCredit`, `Article` → `description`,
`States` → `state` with the Persian labels `درحال تحریر` / `تایید شده` / `ثبت شده` mapped to
`Draft` / `Confirmed` / `Posted` and `UnKnown` → `Unknown`.

### 11.3 Filter and parameter identifiers

| Legacy | Where | Proposed |
|---|---|---|
| `D1` / `_D1` / `Date1` / `@D1` | everywhere | `fromDate` (`as_of` for the 4-column TB) |
| `D2` / `_D2` / `Date2` / `@D2` | everywhere | `toDate` |
| `COID` / `Coid` / `@Coid` / `@Sal` / `M_COID` | everywhere | `fiscalYearId` *(02: `fiscal_year_id`)* |
| `CO_ID = 0` (`همه دوره های مالی`) | `DKolU`, `DMoein`, `TMoein` | `fiscalYearId: null` — "all fiscal periods" |
| `ST` (`در سطح کل` … `در سطح تفضیل 2`) | `Taraz4Setooni_U` | `depth: Kol \| Moein \| Tafsil1 \| Tafsil2` |
| `@Level` (`RX1..RX4`) | `Taraz6SetooniU` | `level` (same enum, single value not cumulative) |
| `@Sabt` (`CH1` `اسناد تاييد شده`, `Ch2` `اسناد ثبت شده`) | `Taraz6SetooniU` | `states: Set<VoucherState>` |
| `R0`/`R1`/`R2` (`اسناد در حال تحریر` / `اسناد تایید شده` / `اسناد ثبت دائم شده`) | `Taraz4Setooni_U` — **dead** | `states: Set<VoucherState>` (implement or delete, §2.1) |
| `@kind` (`Rx0` `… از روي اسناد روزنامه`) | `Taraz6SetooniU` | `source: Subsidiary \| Journal` |
| `M_Kind` = 1 / 2 | `Moein` | `line_source`: `Subsidiary` / `JournalSummary` |
| `_RS` / `_RD` | `MoeinZipU` | `scope: VoucherRange \| DateRange` |
| `_N1` / `_N2` / `_S1` / `_S2` (`از سند` / `تا سند`) | several | `voucherNoFrom` / `voucherNoTo` |
| `_J1` (`شماره عضویت`) | `RoyatJU`, `CardJariU` `S_Card` | `partyCardNo` |
| `F_Size` (`سایز 6`..`سایز 15`) | `Taraz4Setooni_U`, `CardJariU` | *(client-side preference, not an API parameter)* |
| `F_Type` (`اعداد فارسی` / `اعداد انگلیسی`) | `Taraz4Setooni_U` | `numeralSystem: Persian \| Latin` (§6) |
| `SC_Rem = 1` | `SahamdarConfig` | `include_in_party_balance` |
| `SC_K` / `SC_M` / `SC_T` | `SahamdarConfig` | `template_kol` / `template_moein` / `template_tafsil` |

### 11.4 Report infrastructure identifiers

| Legacy | Meaning | Proposed |
|---|---|---|
| `Rp1` / `RP1` / `RP2` / `RP3` / `Rp_TarazMoein` / `RP_Kol` | `TfrxReport` instances | template ids: `<report_id>.<variant>` |
| `DB1` / `DB` (`TfrxDBDataset`) | report dataset binding | *(no equivalent — data is JSON)* |
| `MasterData1` | detail band | table body |
| `PageHeader1` | repeating column strip | `<thead>` with `position: sticky` |
| `Footer1` | grand-total band | `<tfoot>` |
| `T1`, `T2`, `T3`, `T4`, `T5`, `T6`, `_Name`, `_CName`, `_D1`, `_D2`, `_D3`, `_T1`.. | runtime-injected header memos | structured `reportHeader { organisation, title, subtitle, period, page }` |
| `_Total` | signature block memo | `reportFooter.signatureBlock` |
| `Dm.Get_paramstr(1011)` | voucher signature block | `print_settings.signature_block.voucher` |
| `Dm.Get_paramstr(1013)` | ledger signature block | `print_settings.signature_block.ledger` |
| `Dm.Get_paramstr(1014)` | trial-balance signature block | `print_settings.signature_block.trial_balance` |
| `Dm.RegName` | organisation letterhead line | `organisation.display_name` |
| `Dm.RegSal` | fiscal-year letterhead line | `fiscal_year.display_name` |
| `[Page#]` / `[TotalPages#]` / `[line#]` | FastReport built-ins | `pageNumber` / `pageCount` / `rowNumber` |
| `Dbo.Make_R(@Co,k,m,t1,t2)` | account-code string builder | `format_account_code(fiscal_year, segments)` |
| `Dbo.Make_L(...)` | left/padded code variant | `format_account_code_padded(...)` |
| `Dm.Str2String(n)` | amount spelled out in Persian words | `amount_to_persian_words(i64)` |
| `Dm.IsEnabel(user, key)` | permission check | `has_permission(user_id, permission)` |
| `Dm.Is_Admin_Or_Valid_Daftar(k,m,t1,t2)` | per-account lock check | `can_view_account_ledger(user, account_id)` |
| `Dm.Is_Admin_Or_Valid_Jari(card)` | per-party lock check | `can_view_party(user, party_id)` |
| `temp_RJ_<userId>` / `temp_R7_<userId>` | per-user result tables | *(eliminated — see §8)* |
| `#R` / `#P` | SQL Server temp tables | CTEs |
| `MyINI` keys `Left`/`Top`/`Width`/`Height`/`G1C<n>`/`G1FontSize` | per-form UI state | `localStorage` under `ui.<reportId>.*` |

### 11.5 Persian strings that must survive verbatim

These are user-visible and the rebuild must render the same words. Listed with their `fa` locale key.

| Persian | English | Locale key |
|---|---|---|
| `مانده از قبل` | balance brought forward | `report.ledger.openingRow` |
| `بد` / `بس` | Dr / Cr indicator | `report.side.debit` / `report.side.credit` |
| `تش` | side column header | `report.column.side` |
| `ردیف` | row number | `report.column.rowNo` |
| `صـفـحـه : … از …` | page N of M | `report.footer.page` |
| `از تاریخ :` / `تا تاریخ :` | from / to date | `filter.fromDate` / `filter.toDate` |
| `سال مالی` | fiscal year | `filter.fiscalYear` |
| `همه دوره های مالی` | all fiscal periods | `filter.fiscalYear.all` |
| `گزارش خالی است` | the report is empty | `report.error.empty` |
| `چیزی پیدا نشد` | nothing found | `report.error.notFound` |
| `موردی یافت نشد` | no item found | `report.error.noMatch` |
| `حداقل یک مورد را انتخاب کنید` | select at least one item | `report.error.selectAtLeastOne` |
| `لطفا حد اقل یک مورد را انتخاب کنید.` | please select at least one item | `report.error.selectAtLeastOne` *(duplicate wording — unify)* |
| `تاریخ را وارد کنید` | enter the date | `report.error.dateRequired` |
| `تاریخ را به درستی وارد کنید` | enter the date correctly | `report.error.dateInvalid` |
| `رنج تاریخ را به درستی وارد کنید` / `رنج تاریخ را وارد کنید` | enter the date range correctly | `report.error.dateRangeInvalid` *(two wordings — unify)* |
| `حداقل یکی از سه حالت را انتخاب کنید` | select at least one of the three states | `report.error.selectAtLeastOneState` |
| `شماره سند را وارد کنید` | enter the voucher number | `report.error.voucherNoRequired` |
| `شماره عضویت را وارد کنید` | enter the membership number | `report.error.partyCardRequired` |
| `این کد زیر شاخه ندارد` | this code has no children | `report.error.noChildren` |
| `مشاهده دفتر فقط در اختیار مدیر سیستم است` | ledger viewing is reserved for the administrator | `report.error.ledgerLocked` — **wording is wrong, see §3.1; propose `این حساب قفل شده است`** |
| `مشاهده اطلاعات فقط تو سط مدیر سیستم مجاز است` | viewing is permitted only to the administrator | `report.error.partyLocked` — same problem, and `تو سط` is a typo for `توسط` |
| `درحال تحریر` / `تایید شده` / `ثبت شده` | draft / confirmed / posted | `voucher.state.draft` / `.confirmed` / `.posted` |
| `لیست جا خالی اسناد` | list of voucher gaps | `report.voucherGap.title` |
| `مبلغ گردش به ریال` / `مبلغ مانده به ریال` | turnover amount in rials / balance amount in rials | `report.trialBalance.group.turnover` / `.balance` |
| `جاری در قسمت اشخاص وارد نشده است` | not entered in the persons section | `party.error.notInRegister` |
| `جاری در برنامه سهام به روز نشده` / `بروزرسانی انجام شود` | not updated in the share program / perform an update | `party.error.shareRegisterStale` |


---

[← SS10 PROPOSED IMPROVEMENTS (2/2)](04-10-b-proposed-improvements.md) | [Index](00-index.md)
