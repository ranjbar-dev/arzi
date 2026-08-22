# 09 — Complete Unit / File Index

**Scope of this document.** This is the *coverage* document for the `arzi` Delphi codebase: one row for every `.pas` unit in the project root, plus the dependency graph, the dead-code list, and the non-source asset inventory. It deliberately stays at one sentence per unit — the deep business logic lives in the other domain specifications (data model, accounting core, reporting, inventory, treasury, parties, platform/security).

**Method.** Every row below is derived from material actually read out of the file: the `unit` header, the interface `uses` clause, the `T… = class(…)` declaration, the interface-section method names, and the top-level `Caption` property decoded from the matching `.dfm` (including the `#NNNN` character-escape form Delphi uses for Persian text, and the one binary `TPF0` DFM). Nothing here is inferred from the filename alone.

**Totals.** 126 `.pas` files · 123 `.dfm` files · 122 `.pas`↔`.dfm` pairs · 4 form-less library units (`INI`, `LibXL`, `LockUnit`, `Utility`) · 1 orphan `.dfm` with no unit (`GetPass.dfm`).

---

## 1. Master unit table

Sizes are KB. "Caption" is the verbatim top-level `Caption` property of the form in the `.dfm`; where the caption equals the form's own name it means the developer never set one and the Delphi default (the component name) is what the title bar shows — those are marked *(no caption set)*. Several dialogs set their caption at runtime instead, noted where that is the case.

| # | Unit file | Form class / type | Persian caption → English | What it does (one sentence) | Domain | .pas | .dfm | Rebuild priority |
|---|---|---|---|---|---|---|---|---|
| 1 | `Admin.pas` | `TAdminF` (TForm) | `AdminF` *(no caption set)* | User & permission administration: a grid of `TsCheckBox` controls named `C1100`…`C2117`, each one a permission code, bound to the `Password` table in the data module. | platform | 8.1 | 35.3 | core |
| 2 | `Anbar_Amalkard.pas` | `TAnbar_AmalkardF` | `گزارش ورود و خروج انبار` → "Warehouse in/out report" | FastReport-backed report of goods movements in and out of a warehouse, with Excel export via `ComObj`. | reporting | 16.2 | 85.6 | secondary |
| 3 | `Anbar_MandehU.pas` | `TAnbar_MandehF` | `گزارش عملکرد انبار` → "Warehouse performance report" | Stock-balance / warehouse-activity report grid with date filter, print and a transfer (`B_Enteghal1`) action. | reporting | 6.7 | 57.3 | secondary |
| 4 | `AnbarCalaAddU.pas` | `TAnbarCalaAdd` | `مشخصات کالا` → "Goods details" | Add/edit form for one stock item within a warehouse (`init(CodeAnbar, CodeCala)`). | inventory | 8.2 | 6.4 | core |
| 5 | `AnbarCalaSelectU.pas` | `TAnbarCalaSelect` | `انتخاب کالا` → "Select goods" | Modal item-picker with name search, used by the stock-card and invoice-line forms. | shared-dialog | 2.4 | 3.1 | core |
| 6 | `AnbarCalaU.pas` | `TAnbarCala` | `انبار کالا` → "Warehouse goods" | Master grid of items held in one warehouse (`init(AnbarCode)`) with add/edit/delete popup. | inventory | 5.5 | 5.5 | core |
| 7 | `AnbarCardJensiU.pas` | `TAnbarCardJensi` | `کارت جنسي انبار` → "Warehouse stock card" | Per-item stock card (kardex) rendered through FastReport for a selected item. | inventory | 4.8 | 29.1 | core |
| 8 | `AnbarFactorAddU.pas` | `TAnbarFactorAdd` | `افزودن به فاکتور` → "Add to invoice" | Line-entry dialog for a warehouse invoice: item code, quantity/`Phi` (unit price) and discount. | inventory | 5.8 | 7.1 | core |
| 9 | `AnbarFactorU.pas` | `TAnbarFactor` | `فاکتور کالا` → "Goods invoice" | The warehouse invoice editor — header, line grid, totals, counterparty selection and save. | inventory | 27.0 | 19.3 | core |
| 10 | `AnbarListU.pas` | `TAnbarList_F` | `ليست فاکتورهاي انبار` → "Warehouse invoice list" | Browsable list of all warehouse invoices with type filters, search, renumber, settlement and three print routes. | inventory | 17.4 | 77.5 | core |
| 11 | `AnbarReportKharidU.pas` | `TAnbarReportKharid` | `گزارش ورود و خروج انبار` → "Warehouse in/out report" | Small parameter form that runs a purchase-oriented warehouse movement FastReport. | reporting | 2.5 | 16.5 | secondary |
| 12 | `AnbarReportU.pas` | `TAnbarReport_F` | `گزارش عملیات انبار پسته` → "Pistachio warehouse operations report" | Three-variant warehouse operations report (`B_Rep1/2/3`) filtered by warehouse and date. | reporting | 9.4 | 113.2 | secondary |
| 13 | `AnbarTanzimU.pas` | `TAnbarTanzim` | `تنظيمات انبار` → "Warehouse settings" | Per-warehouse configuration: linked purchase/sales account heads, VAT rate, default counterparty. | inventory | 7.1 | 14.4 | core |
| 14 | `ArticleMoeinu.pas` | `TArticleMoein` | `ارتيکل حسابداري` → "Accounting article" | Entry form for a single subsidiary-voucher line (kol/moein/tafsili 1/2, debit, credit, description). | accounting-core | 10.7 | 13.0 | core |
| 15 | `ArticleRooznamehU.pas` | `TArticleRooznameh` | `آرتيکل حسابداري` → "Accounting article" | The daybook (`rooznameh`) equivalent of the above — one journal line, debit/credit/description. | accounting-core | 5.5 | 7.2 | core |
| 16 | `Asnad_Daryaft_NewU.pas` | `TAsnad_Daryaft_New` | `دریافت چک` → "Cheque receipt" | Thin new-cheque-receipt form built on the `TFGetCode` account-picker frame. | treasury | 2.4 | 5.1 | secondary |
| 17 | `Backup_U.pas` | `TBackupForm` | `ايجاد پشتيبان از اطلاعات` → "Create data backup" | Copies every ADO table into a dated `.ABS` (Absolute Database) file named from the Persian date. | platform | 5.2 | 4.2 | secondary |
| 18 | `BankTanzim.pas` | `TBankTanzimF` | `تنظیمات بانک` → "Bank settings" | Maintains the bank-account list (IBAN/card validation lives in the data module). | treasury | 2.6 | 7.3 | core |
| 19 | `BastanHesab.pas` | `TBastanHesabF` | `انتقال حسابها به اختتامیه` → "Transfer accounts to the closing entry" | Moves selected account heads into the year-end closing voucher. | accounting-core | 3.6 | 1.5 | core |
| 20 | `BedBes.pas` | `TBedBesF` | `لیست بدهکاران و بستانکاران اشخاص و شرکتها` → "List of debtors and creditors — persons and companies" | Aged debtor/creditor listing across parties, drilling into `CardJariU` on double-click. | reporting | 5.1 | 34.6 | core |
| 21 | `CardJariU.pas` | `TCardJariF` | `فرم خلاصه اطلاعات جاری اشخاص` → "Party current-account summary form" | Party dashboard: current balance, transaction card and links into the subsidiary ledger and party master. | parties | 13.7 | 434.6 | core |
| 22 | `ChangePasswordU.pas` | `TChangePassword` | `تغییر رمز کاربر` → "Change user password" | Old/new password dialog writing back to the `Password` table. | platform | 2.8 | 4.0 | core |
| 23 | `ChangesU.pas` | `TChangeS_F` | `تغییر سال` → "Change fiscal year" | Grid of available fiscal years; double-click switches the active year. | platform | 2.1 | 3.9 | core |
| 24 | `CheckBargashtu.pas` | `TCheckBargashtF` | `برگشت چک از بانک` → "Cheque returned by the bank" | Records a bounced cheque and posts the reversing entry against the chosen account head. | treasury | 9.6 | 8.8 | core |
| 25 | `CheckDaryaft2U.pas` | `TCheckDaryaft2F` | `واگذاری چک به بانک` → "Assigning a cheque to the bank" | Records handing a received cheque over to a bank for collection. | treasury | 9.2 | 9.0 | core |
| 26 | `CheckDaryaftU.pas` | `TCheckDaryaftF` | `درریافت چک` *(sic — typo for `دریافت چک`)* → "Cheque receipt" | Full receive-a-cheque form: payer, bank, dates, amount, debit/credit heads, plus `new`/`Edit`/`Delete_Check`. | treasury | 14.1 | 7.9 | core |
| 27 | `CheckEditAddU.pas` | `TCheckEditAddF` | `انتخاب بدهکار و مبلغ` → "Select debtor and amount" | Allocation dialog splitting a cheque across one or more debtor accounts. | shared-dialog | 4.1 | 6.1 | core |
| 28 | `CheckEditU.pas` | `TCheckEditF` | `صدور چک` → "Issue a cheque" | Issued-cheque editor with allocation grid, two FastReport print layouts and save/delete. | treasury | 19.3 | 51.9 | core |
| 29 | `CheckEsterdadU.pas` | `TCheckEsterdadF` | `استرداد چک` → "Cheque return (to payer)" | Records giving a held cheque back to the party it came from. | treasury | 8.7 | 7.8 | core |
| 30 | `CheckListDU.pas` | `TCheckListDF` | `تنظیمات بانک` *(stale caption, copied from `BankTanzim`)* → "Bank settings" | The received-cheque register: master/detail grids driving new/edit/collect/return/bounce/bank-assign actions and printing. | treasury | 18.4 | 47.4 | core |
| 31 | `CheckListU.pas` | `TCheckListF` | `تنظیمات بانک` *(stale caption)* → "Bank settings" | The issued-cheque register, with view/print and a jump into the generated voucher. | treasury | 9.1 | 37.8 | core |
| 32 | `CheckVosoolU.pas` | `TCheckVosoolF` | `اعلام وصول چک` → "Declare a cheque cleared" | Marks a cheque as collected and books the bank-side entry. | treasury | 9.9 | 8.8 | core |
| 33 | `CodeNameU.pas` | `TCodeNameF` | `CodeNameF` *(no caption set — title supplied at runtime by `GetCodeName`)* | Reusable modal that asks for a numeric code plus a name, used by the chart-of-accounts editors. | shared-dialog | 1.4 | 1.9 | core |
| 34 | `CompanyEditU.pas` | `TCompanyEdit` | `ورود اطلاعات اشخاص حقوقی` → "Legal-entity (company) data entry" | Tabbed master form for corporate parties — registration number, national ID, addresses, contacts. | parties | 9.9 | 11.0 | core |
| 35 | `DaftarT_U.pas` | `TDaftarT_F` | `مشاهده دفتر تجمیعی` → "View the aggregate ledger" | Consolidated ledger browser with drill-down, level-up navigation and FastReport output. | reporting | 11.0 | 44.0 | core |
| 36 | `DateFrameU.pas` | `TDateFrame` (**TFrame**) | *(frames have no caption)* | Abandoned scaffold frame containing only a default `Label1`/`Edit1`; `TDateFrame` appears in no other `.dfm`. | dead-or-unused | 0.4 | 0.3 | drop |
| 37 | `DKolU.pas` | `TDKolF` | `نمایش دفتر کل` → "General-ledger view" | General-ledger (kol level) browser with date range, account picker, print and drill-through to the voucher editor. | reporting | 9.2 | 42.8 | core |
| 38 | `DMoein.pas` | `TDMoeinF` | `نمایش دفتر معین` → "Subsidiary-ledger view" | Subsidiary-ledger browser at kol/moein/tafsili1/tafsili2 depth; also exposes `Get_FullCode`/`Get_FullName`/`Get_Valid` helpers used by other forms. | reporting | 19.8 | 50.0 | core |
| 39 | `Dmu.pas` | `TDM` (**TDataModule**) | *(data module — no caption)* | The single global data module: every ADO connection, query, stored procedure, FastReport dataset, image list and shared helper (`Current_Date`, `isValidDate`, `IsValidShaba`, `IsValidKart`, registry access) in the application. | platform | 50.3 | 559.2 | core |
| 40 | `EditArticleMoeinU.pas` | `TEditArticleMoein` | `طرف حساب` → "Counterparty" | Edits one existing voucher line, resolving the 4-level account code as you type. | accounting-core | 11.0 | 9.1 | core |
| 41 | `EnteghalU.pas` | `TEnteghalF` | `بستن حسابها` → "Closing the accounts" | Year-end close: rolls balances forward onto a target account head, with progress via `WaitU`. | accounting-core | 12.7 | 8.7 | core |
| 42 | `FactorPesteh_U.pas` | `TFactorPesteh_F` | `لیست قبضهای  باسکول و خرید پسته` → "List of weighbridge receipts and pistachio purchases" | Grid of weighbridge/purchase receipts with new/delete and a FastReport print. | inventory | 11.4 | 61.5 | secondary |
| 43 | `Factorprint2U.pas` | `TFactorprint2` | `چاپ فاکتور رسمی` → "Print the official invoice" | FastReport wrapper printing the tax-office ("official") invoice layout. | reporting | 4.1 | 60.7 | secondary |
| 44 | `FactorPrint3U.pas` | `TFactorPrint3` | `چاپ فاکتور` → "Print invoice" | Third invoice-print layout, adds an A4-vs-A5 toggle and direct printer selection. | reporting | 6.6 | 109.8 | secondary |
| 45 | `FactorPrintU.pas` | `TFactorPrint` | `چاپ فاکتور` → "Print invoice" | The original invoice-print form, with a `RP1GetValue` callback feeding computed fields into the report. | reporting | 11.8 | 34.5 | secondary |
| 46 | `FGetCodeU.pas` | `TFGetCode` (**TFrame**) | *(frame — no caption)* | The reusable 4-level account-code picker frame (kol → moein → tafsili1 → tafsili2) embedded into `Asnad_Daryaft_NewU`, `FinalU`, etc. | shared-dialog | 6.9 | 7.5 | core |
| 47 | `FinalU.pas` | `TFinalF` | `بستن حسابها` → "Closing the accounts" | Earlier account-closing form using the `TFGetCode` frame; superseded in practice by `NewFinalu`. | accounting-core | 7.9 | 6.5 | secondary |
| 48 | `FISHDaryaftU.pas` | `TFishDaryaftF` | `دریافت وجه نقد - کارتخوان - واریز فیش بانکی` → "Cash receipt — card reader — bank deposit slip" | The non-cheque receipt form covering cash, POS and bank-slip deposits, with debit/credit head selection. | treasury | 17.1 | 7.3 | core |
| 49 | `FishListD.pas` | `TFishListDF` | `لیست واریزیها` → "List of deposits" | Register of the receipts created by `FISHDaryaftU`, with search, state filter and printing. | treasury | 12.0 | 42.6 | core |
| 50 | `Get2D.pas` | `TGet2D_F` | `   ورود اطلاعات` → "Data entry" | Modal that asks for a from/to pair of Persian dates and validates ordering via `DM.isValidDate`. | shared-dialog | 1.4 | 2.6 | core |
| 51 | `Get_Serial.pas` | `TGetSerialF` | `ورود اطلاعات` → "Data entry" | Prompts for a weighbridge receipt serial and validates it against `DM.B_SelectSerial`; only reachable from the dead `Lab` form. | dead-or-unused | 1.6 | 2.3 | drop |
| 52 | `GetCodeStringU.pas` | `TGetCodeStringF` | `GetCodeStringF` *(no caption set)* | Empty stub form whose only code saves and restores its own window geometry to the INI file. | dead-or-unused | 1.1 | 0.5 | drop |
| 53 | `GetD.pas` | `TGetD_F` | `GetD_F` *(runtime caption via `GetDate`)* | Modal single-Persian-date prompt; defaults to `DM.Current_Date` and validates before enabling OK. | shared-dialog | 1.0 | 1.8 | core |
| 54 | `GetN.pas` | `TGetN_F` | `GetN_F` *(runtime caption via `GetNo`)* | Modal single-integer prompt with configurable digit length. | shared-dialog | 1.7 | 1.7 | core |
| 55 | `GetN2N.pas` | `TGetN2N_F` | `GetN2N_F` *(runtime caption via `Get2No`)* | Modal from/to integer-range prompt, used for voucher-number ranges. | shared-dialog | 1.9 | 2.4 | core |
| 56 | `GetPassu.pas` | `TGetPass` | `فرم ورود به سيستم` → "System login form" | The login screen — user, password, company/fiscal-year selection; also drives the skin manager and splash. | platform | 3.6 | 4.7 | core |
| 57 | `GetPassword.pas` | `TGetPasswordF` | `کنترل رمز` → "Password check" | Hashes a typed password with `SysInfo.ElfHash`; referenced only by the dead `Lab` form. | dead-or-unused | 1.0 | 1.5 | drop |
| 58 | `GetS.pas` | `TGetS_F` | `GetS_F` *(runtime caption via `GetString`)* | Modal single-string prompt, auto-sizing itself and switching BiDi mode per the caller. | shared-dialog | 2.2 | 1.6 | core |
| 59 | `Ghabz.pas` | `TGhabzF` (**TFrame**) | *(frame — no caption)* | Frame that renders one weighbridge receipt (serial, in/out/net weight, status) from `DM.B_SelectSerial`; embedded only in the dead `Lab` form. | dead-or-unused | 2.5 | 4.9 | drop |
| 60 | `InFile.pas` | `TInFileF` | `خواندن اطلاعات فایل` → "Reading data from a file" | File-picker + count/description dialog that feeds bulk voucher-line import in `SanadMoeinu`. | accounting-core | 2.8 | 3.6 | secondary |
| 61 | `INI.pas` | *no form — library unit* (`TMyIni`, global `MyINI`) | — | Encrypted INI settings store: plain and XOR-encrypted string/integer/bool read-write, used by nearly every form to persist window geometry and configuration. | platform | 8.5 | — | core |
| 62 | `Kharid_BU.pas` | `TKharid_B` | `     اطلاعات پايه خريد و فروش پسته` → "Base data for pistachio buying and selling" | Configures the eight account heads used by the pistachio trading module. | inventory | 11.6 | 19.8 | secondary |
| 63 | `Kharid_U.pas` | `TKharid` | `خريد و فروش پسته` → "Pistachio buying and selling" | Grid-driven pistachio purchase/sale entry screen, delegating item detail to `PestehD_U`. | inventory | 6.7 | 8.5 | secondary |
| 64 | `KolSatateU.pas` | `TKolSatate` | `وضعیت حسابهای کل` → "Status of general accounts" | Abandoned duplicate of `KolStateU` — its whole implementation is `procesdure TKolSatate.init` (misspelled keyword), so the unit cannot compile; not in `arzi.dpr`, referenced by nothing. | dead-or-unused | 0.7 | 2.0 | drop |
| 65 | `KolStateU.pas` | `TKolState` | `وضعیت حسابهای کل` → "Status of general accounts" | Working general-account status screen with a FastReport output, opened from the main menu. | reporting | 2.8 | 12.0 | secondary |
| 66 | `Lab.pas` | `TLabF` | `ورود اطلاعات انس گذاري` → "Ounce-grading data entry" | Laboratory grading screen for a weighbridge receipt; auto-created in `arzi.dpr` but referenced by no `uses` clause anywhere, so unreachable. | dead-or-unused | 3.0 | 91.7 | drop |
| 67 | `LibXL.pas` | *no form — library unit* (`TXLBook`, `TXLSheet`, `TXLFont`, `TXLFormat`, …) | — | Hand-written Delphi header binding for the native `libxl.dll` XLS/XLSX writer; its only consumer is the dead `ToExcelU`. | dead-or-unused | 135.4 | — | drop |
| 68 | `ListSarfaslu.pas` | `TListSarfasl` | `ليست سرفصلها` → "List of account heads" | Browsable chart of accounts with create/edit, supplementary-info and party links, plus an Excel export via `ComObj`. | accounting-core | 9.6 | 17.5 | core |
| 69 | `LockUnit.pas` | *no form — library unit* (`TSysInfo`, global `SysInfo`) | — | Machine-fingerprint helpers for licensing: CPU ID/name, HD serial, BIOS and video ROM dates, system name, `ElfHash`, registry read/write. | platform | 5.4 | — | secondary |
| 70 | `Mainu.pas` | `TMain` | `حسابداري ارزي` → "Foreign-currency accounting" | The application shell — skinned main window, the whole menu/launcher surface, and a `uses` clause that pulls in essentially every feature form. | platform | 30.5 | 713.9 | core |
| 71 | `MakeNewU.pas` | `TMakeNew` | `ايجاد سال مالي جديد` → "Create a new fiscal year" | Creates the next fiscal-year dataset and seeds it from the current one. | platform | 4.3 | 3.4 | core |
| 72 | `MakeRooznamehU.pas` | `TMakeRooznameh` | `تبدیل اسناد معین به روزنامه` → "Convert subsidiary vouchers into daybook entries" | Batch conversion of subsidiary (moein) vouchers into daybook (rooznameh) vouchers. | accounting-core | 5.8 | 4.2 | core |
| 73 | `MakeSanadU.pas` | `TMakeSanadF` | `ادغام اسناد` → "Merging vouchers" | Builds a consolidated accounting voucher out of a set of selected invoices (the heavy generator behind `SodoorSanadU`). | accounting-core | 26.0 | 9.9 | core |
| 74 | `MergeSanad.pas` | `TMergeSanadF` | `ادغام اسناد` → "Merging vouchers" | Simpler two-voucher merge (`S1` into `S2`) invoked from the voucher browser. | accounting-core | 9.1 | 8.7 | secondary |
| 75 | `MoeinSearchU.pas` | `TMoeinSearch` | `      جستجو در دفتر معین` → "Search in the subsidiary ledger" | Full-text/amount search across subsidiary-ledger lines with debit/credit/description filter checkboxes. | reporting | 6.7 | 19.6 | core |
| 76 | `MoeinToRU.pas` | `TMoeinToR` | `      تبدیل اسناد معین به روزنامه` → "Convert subsidiary vouchers into daybook entries" | Range-based moein→rooznameh conversion launched from the daybook browser. | accounting-core | 8.6 | 5.9 | secondary |
| 77 | `MoeinZipU.pas` | `TMoeinZip` | `خلاصه اسناد معین` → "Summary of subsidiary vouchers" | Voucher-summary/compression report with grouping options, FastReport preview and Excel export; carries the largest `.dfm` in the project (embedded report templates). | reporting | 25.5 | 5924.6 | core |
| 78 | `NewFinalu.pas` | `TNewFinalF` | `بستن حسابها` → "Closing the accounts" | The current year-end closing screen — debit head selection, per-account grid, save. | accounting-core | 9.3 | 10.1 | core |
| 79 | `NewSarfaslu.pas` | `TNewSarfasl` | `ايجاد سرفصل` → "Create an account head" | Creates a new chart-of-accounts node at kol/moein/tafsili1/tafsili2 level. | accounting-core | 6.0 | 6.0 | core |
| 80 | `PestehD_U.pas` | `TPestehD` | `     مشخصات پسته` → "Pistachio details" | Detail entry for one pistachio lot (weights, basculating values) inside the trading screen. | inventory | 4.7 | 11.1 | secondary |
| 81 | `Print_Anbar15.pas` | `TPrint_Anbar15F` | `چاپ فرم تولید کالا` → "Print the goods-production form" | Prints the production (`print_Tolid`) document for warehouse type 15. | reporting | 5.7 | 28.5 | secondary |
| 82 | `Print_Anbar16.pas` | `TPrint_Anbar16F` | `چاپ فاکتور` → "Print invoice" | Prints the inter-warehouse relocation (`print_Jabejaei`) document for warehouse type 16 — a sibling of, not a duplicate of, `Print_Anbar15`. | reporting | 6.1 | 25.0 | secondary |
| 83 | `PrintM2U.pas` | `TPrintM2` | `چاپ سند معين` → "Print the subsidiary voucher" | Second FastReport layout for a subsidiary voucher (compact/alternate form). | reporting | 2.4 | 42.5 | secondary |
| 84 | `PrintMU.pas` | `TPrintM` | `چاپ سند معين` → "Print the subsidiary voucher" | Primary FastReport layout for a subsidiary voucher, with an `RP2GetValue` computed-field callback. | reporting | 4.0 | 61.8 | core |
| 85 | `PrintNu.pas` | `TPrintN` | `چاپ سند` → "Print voucher" | Generic voucher print/preview host; its `.dfm` embeds a very large report template. | reporting | 3.6 | 446.3 | secondary |
| 86 | `Report6U.pas` | `TReport6F` | `کنترل شماره اسناد` → "Voucher-number control" | Audit report over a voucher-number range checking for gaps and duplicates. | reporting | 4.0 | 14.6 | secondary |
| 87 | `Report7U.pas` | `TReport7F` | `رویت جامع حسابداری` → "Comprehensive accounting view" | Cross-cutting accounting review report with voucher/date toggles and drill-through. | reporting | 14.4 | 52.2 | secondary |
| 88 | `RooznamehViewU.pas` | `TRooznamehView` | `نمايش اسناد روزنامه` → "Display daybook vouchers" | The daybook voucher browser — list, lock, renumber, re-date, re-describe, delete, print, and conversion entry point. | accounting-core | 12.7 | 42.5 | core |
| 89 | `RoozViewU.pas` | `TRoozView` | `اسناد روزنامه` → "Daybook vouchers" | Bare grid-only daybook viewer whose entire code is one `FormClose` handler; not in `arzi.dpr`, referenced by nothing — an early draft of `RooznamehViewU`. | dead-or-unused | 0.9 | 5.5 | drop |
| 90 | `RoyatJU.pas` | `TRoyatJF` | `رویت جامع اسناد معین` → "Comprehensive view of subsidiary vouchers" | The big cross-voucher review screen with filter radio groups, a progress bar and drill-through into `DMoein`. | reporting | 23.0 | 92.0 | core |
| 91 | `S_KolU.pas` | `TS_KolF` | `سرفصلهای حسابداری` → "Accounting chart of accounts" | Older chart-of-accounts editor; not in `arzi.dpr` and referenced by nothing — superseded by `SNewu`, which has the same caption and a superset of the behaviour. | dead-or-unused | 13.4 | 19.2 | drop |
| 92 | `SahamdarEditU.pas` | `TSahamdarEdit` | `   ورود اطلاعات اشخاص` → "Person data entry" | Tabbed master form for natural-person parties (name, national/card ID, dates, contacts). | parties | 11.0 | 13.7 | core |
| 93 | `SahamdarInfoU.pas` | `TSahamdarInfo` | `حسابهای بانکی اشخاص` → "Bank accounts of persons" | Grid of a party's bank accounts, with a select-and-return mode used by the cheque allocation dialog. | parties | 3.4 | 8.3 | core |
| 94 | `SahamdarP.pas` | `TSahamdarP_F` | `فرم مشخصات سهامداران` → "Shareholder details form" | Shareholder-specific data entry (share counts / participation). | parties | 6.7 | 9.2 | secondary |
| 95 | `SahamdarU.pas` | `TSahamdar` | `اطلاعات تکميلي اشخاص` → "Supplementary information about persons" | The party master list — natural persons and companies side by side, with lock, bank-accounts and edit routes. | parties | 10.2 | 15.6 | core |
| 96 | `Sanad_NDU.pas` | `TSanad_ND` | `                   ايجاد سند جديد` → "Create a new voucher" | Tiny dialog collecting the number/date for a new voucher. | accounting-core | 1.9 | 2.4 | core |
| 97 | `SanadEditU.pas` | `TSanadEditF` | `نمایش سند معین` → "Display the subsidiary voucher" | The main voucher editor: line grid, add/edit/delete, import, replace, debit/credit balancing, navigation and two print routes — the largest business unit in the project. | accounting-core | 32.4 | 54.5 | core |
| 98 | `SanadMoeinu.pas` | `TSanadMoein` | `صدور ستد حسابداری` → "Issuing an accounting voucher" | Voucher-issuing screen driving `ArticleMoeinu`/`ArticleRooznamehU` line entry plus file import. | accounting-core | 14.3 | 13.1 | core |
| 99 | `SanadViewU.pas` | `TSanadView` | `نمايش اسناد` → "Display vouchers" | Subsidiary-voucher browser: list, new/edit/copy/delete, lock, renumber, re-date, merge, and print. | accounting-core | 25.2 | 48.9 | core |
| 100 | `Sarfasl_Kolu.pas` | `TSarfasl_Kol` | `     سرفصلهای کل حسابداری` → "General accounting heads" | Drill-down browser over the chart of accounts level by level (`Sarfasl_level` 1→4). | accounting-core | 2.9 | 5.8 | secondary |
| 101 | `Sarfasl_ListU.pas` | `TSarfasl_List` | `ليست سرفصلها` → "List of account heads" | Builds a board of `TsSpeedButton` tiles at runtime, one per top-level account head — a launcher panel. | accounting-core | 1.5 | 1.2 | cosmetic |
| 102 | `Sarfasl_SelectU.pas` | `TSarfasl_Select` | `         انتخاب سرفصل` → "Select an account head" | The most-used account-head picker modal, with all/current/103-only scope buttons. | shared-dialog | 9.1 | 3.4 | core |
| 103 | `Sarfasl_TakmilU.pas` | `TSarfasl_Takmil` | `اطلاعات تکمیلی` → "Supplementary information" | Extra attributes attached to an account head (company name, flags). | accounting-core | 5.0 | 7.2 | secondary |
| 104 | `SarfaslChap.pas` | `TSarfalsChapF` | `لیست سرفصلها` → "List of account heads" | Chart-of-accounts print grid; auto-created in `arzi.dpr` but no `uses` clause anywhere references it, so it is unreachable at runtime. | dead-or-unused | 1.7 | 4.0 | drop |
| 105 | `SayMessage.pas` | `TSayMessage_F` | `SayMessage_F` *(runtime caption via `SayMSG`)* | Three-line themed message box helper used instead of `MessageDlg` across the older forms. | shared-dialog | 1.0 | 1.0 | core |
| 106 | `SelectSarfasl.pas` | `TSelect_Sarfasl` | `انتخاب سرفصل` → "Select an account head" | The newer grid-based account-head picker used by the ledger and voucher editors (parallel to `Sarfasl_SelectU`). | shared-dialog | 4.3 | 3.8 | core |
| 107 | `SNewu.pas` | `TSNew` | `سرفصلهای حسابداری` → "Accounting chart of accounts" | The live chart-of-accounts tree editor — add, rename, recode, promote/demote level, lock, delete, print. | accounting-core | 22.3 | 34.8 | core |
| 108 | `SodoorSanadU.pas` | `TSodoorSanad` | `لیست فاکتورها جهت صدور سند` → "List of invoices for voucher issuance" | The bridge from inventory to accounting: approve invoices, issue the resulting vouchers, settle, and print production/relocation documents. | accounting-core | 11.9 | 51.5 | core |
| 109 | `TajmiU.pas` | `TTajmiF` | `     دفتر تجمیعی` → "Aggregate ledger" | Compact aggregate-ledger grid with calculated totals (the lightweight sibling of `DaftarT_U`). | reporting | 3.9 | 9.5 | secondary |
| 110 | `TankhahEdit.pas` | `TTankhahEditF` | `ليست تنخواه` → "Petty-cash (imprest) list" | Petty-cash document editor with allocation grid and two print layouts; **its `.dfm` is stored in binary `TPF0` form, not text**. | treasury | 17.8 | 19.2 | core |
| 111 | `TankhahEditAddu.pas` | `TTankhahEditAddF` | `انتخاب بدهکار تنخواه  و مبلغ` → "Select the petty-cash debtor and amount" | Allocation dialog for one petty-cash line. | treasury | 4.1 | 4.8 | core |
| 112 | `TankhahList.pas` | `TTankhahListF` | `لیست تنخواه ` → "Petty-cash list" | Register of petty-cash documents with new/edit/view/print and a jump into the generated voucher. | treasury | 8.3 | 19.8 | core |
| 113 | `TanzimChapu.pas` | `TTanzimChap` | `تنظیمات چاپ فاکتور` → "Invoice print settings" | Stores per-printer invoice layout offsets in the INI file. | platform | 3.5 | 6.5 | secondary |
| 114 | `TanzimPU.pas` | `TTanzimP` | `TanzimP` *(no caption set)* | Completely empty form — no controls, no code beyond `{$R *.dfm}`; not in `arzi.dpr`, referenced by nothing. | dead-or-unused | 0.4 | 0.3 | drop |
| 115 | `TanzimU.pas` | `TTanzimF` | `                تنظيمات برنامه  ` → "Program settings" | The global settings screen — company identity, registration/tax numbers, logo, paths, default account heads. | platform | 10.9 | 19.8 | core |
| 116 | `TarafU.pas` | `TTaraf` | `طرف حساب` → "Counterparty" | The canonical counterparty/account-code resolver form, exposing `Get_FullCode`, `Get_FullName`, `Get_FullCodeName`, `Get_SSn`, `Get_Valid`, `Set_FullCode` — used by 18 other units. | shared-dialog | 14.1 | 8.9 | core |
| 117 | `Taraz4Setooni_U.pas` | `TTaraz4Setooni` | `تراز آزمايشي 4 ستوني` → "Four-column trial balance" | Trial balance at a chosen level with a FastReport print and font-size/INI persistence. | reporting | 9.2 | 42.6 | core |
| 118 | `Taraz6SetooniU.pas` | `TTaraz6Setooni` | `تراز آزمایشی 6 ستونی` → "Six-column trial balance" | Six-column trial balance report (opening / period / closing). | reporting | 4.2 | 40.8 | core |
| 119 | `TasfiehFactor.pas` | `TTasfiehFactorF` | `تسویه حساب فاکتور` → "Invoice settlement" | Settles an invoice against cash receipts and cheques, pulling in the receipt and cheque dialogs directly. | treasury | 9.2 | 12.9 | core |
| 120 | `testmainU.pas` | `TTestF` | *(empty caption string)* | Developer key-generator: computes a machine-derived value via `LockUnit.MakeI` and writes `Base/CS3` into the INI — the licensing side-tool, also the entry form of the separate `test.dpr`. | platform | 5.5 | 4.5 | drop |
| 121 | `TMoein.pas` | `TTMoeinF` | `نمایش دفتر معین` → "Subsidiary-ledger view" | A second subsidiary-ledger viewer, opened specifically from the party card (`CardJariU`). | reporting | 8.1 | 40.7 | secondary |
| 122 | `ToExcelDaraeiU.pas` | `TToExcelDaraei` | `ذخیره سند در فایل اکسل` → "Save the voucher to an Excel file" | The live Excel export — FastReport XLS export plus a `ComObj` OLE-automation path. | reporting | 5.6 | 23.5 | secondary |
| 123 | `ToExcelU.pas` | `TToExcel` | `ذخیره سند در فایل اکسل` → "Save the voucher to an Excel file" | The older Excel export using the `LibXL` native binding; its `Application.CreateForm(TToExcel, ToExcel)` line is **commented out** in `arzi.dpr` and no unit `uses` it — superseded by `ToExcelDaraeiU`. | dead-or-unused | 10.5 | 33.4 | drop |
| 124 | `Utility.pas` | *no form — library unit* (`TUtil`) | — | Grab-bag helper class: symmetric encrypt/decrypt and base-64-ish encode/decode, file utilities, host/IP and NetBIOS lookups, CPU/disk identification, `IsFarsiDate`, `ElfHash`, registry access, Roman numerals. | platform | 46.3 | — | secondary |
| 125 | `WaitU.pas` | `TWaitF` | `    Green Gold ` → "Green Gold" (the product/vendor name) | Splash and progress form shown during startup (`Gotonextposition` is called between every `CreateForm` in `arzi.dpr`) and during long queries. | platform | 1.0 | 0.7 | cosmetic |
| 126 | `YesOrNo.pas` | `TYesOrNo_F` | `YesOrNo_F` *(runtime caption via `GetYes`)* | Three-line themed yes/no confirmation modal used instead of `MessageDlg` in the older forms. | shared-dialog | 0.8 | 1.4 | core |

### Domain totals

| Domain | Units |
|---|---|
| accounting-core | 24 |
| reporting | 27 |
| treasury | 16 |
| platform | 15 |
| shared-dialog | 14 |
| dead-or-unused | 13 |
| inventory | 11 |
| parties | 6 |
| **Total** | **126** |

### Rebuild-priority totals

| Priority | Units |
|---|---|
| core | 77 |
| secondary | 33 |
| drop | 14 |
| cosmetic | 2 |
| **Total** | **126** |

(The `drop` count is 14 `.pas` files: the 13 units in the `dead-or-unused` domain plus `testmainU.pas`, which is classified `platform` because it is the licence-key generator but is still not worth porting. Section 3 adds the orphan `GetPass.dfm`, which is not a `.pas` file.)

---

## 2. Dependency overview

Counts below are **reverse dependencies**: how many *other project units* name the unit in a `uses` clause (interface or implementation). Delphi RTL/VCL and third-party units are excluded.

### Top 20 most depended-upon units — the de-facto shared kernel

| Rank | Unit | Referenced by N units | What that tells you |
|---|---|---|---|
| 1 | `Dmu.pas` | 105 | Every screen talks to the single global `TDM` data module. This is the entire persistence layer; in the rebuild it becomes the Rust repository/service layer and there is no direct equivalent on the React side. |
| 2 | `INI.pas` | 90 | Every form persists its own geometry and reads configuration from the encrypted INI. Rebuild equivalent: server-side user-preferences table + client-side layout state. |
| 3 | `TarafU.pas` | 18 | The counterparty/account-code resolver — a genuine shared domain service (`Get_FullCode`, `Get_FullName`, `Get_Valid`). |
| 4 | `Sarfasl_SelectU.pas` | 12 | Primary account-head picker modal. |
| 5 | `GetS.pas` | 10 | Generic "prompt for a string" modal. |
| 5 | `YesOrNo.pas` | 10 | Generic confirmation modal. |
| 7 | `GetN.pas` | 9 | Generic "prompt for an integer" modal. |
| 8 | `SayMessage.pas` | 8 | Generic message box. |
| 8 | `WaitU.pas` | 8 | Splash/progress overlay. |
| 8 | `SanadEditU.pas` | 8 | The voucher editor is the universal drill-through target from every ledger and register. |
| 11 | `FGetCodeU.pas` | 7 | Embeddable account-code picker **frame** (as opposed to modal). |
| 11 | `SelectSarfasl.pas` | 7 | Second account-head picker (grid variant). |
| 13 | `Utility.pas` | 5 | Crypto / system-identification helpers. |
| 13 | `GetD.pas` | 5 | Generic Persian-date prompt. |
| 15 | `TanzimChapu.pas` | 4 | Print-offset settings, read by the invoice printers. |
| 16 | `AnbarFactorU.pas` | 3 | Warehouse invoice editor (drill-through target). |
| 16 | `DMoein.pas` | 3 | Subsidiary-ledger viewer (drill-through target). |
| 16 | `SahamdarU.pas` | 3 | Party master list. |
| 16 | `SahamdarInfoU.pas` | 3 | Party bank accounts. |
| 16 | `CheckEditAddU.pas` | 3 | Cheque/petty-cash allocation dialog. |
| 16 | `CheckDaryaftU.pas` / `CheckDaryaft2U.pas` / `CheckEditU.pas` / `FISHDaryaftU.pas` | 3 each | The treasury entry forms, each reachable from two registers plus settlement. |
| 16 | `LockUnit.pas` | 3 | Licensing/fingerprint helpers. |

Two structural observations that matter for the rebuild:

- **`Dmu` + `INI` are the whole platform layer.** 105 of 126 units reach into `TDM` directly and 90 read/write the INI. There is no repository boundary, no service layer and no dependency injection anywhere — SQL text, report datasets and UI state all live in one 50 KB unit with a 559 KB form file. Replacing these two units *is* the architectural work.
- **`Mainu` is a god-shell.** Its `uses` clause names 80 project units — essentially every feature form in the application. Nothing but `FactorPrint3U` references `Mainu` back (it reads a printer setting off the main form), so the dependency is one-directional and the shell can be replaced by a React router without touching feature code semantics.

### Units nothing else references (0 reverse dependencies)

| Unit | In `arzi.dpr`? | Verdict |
|---|---|---|
| `GetCodeStringU.pas` | no | dead |
| `KolSatateU.pas` | no | dead (does not compile) |
| `Lab.pas` | yes (auto-created) | dead — created but unreachable |
| `RoozViewU.pas` | no | dead |
| `S_KolU.pas` | no | dead |
| `SarfaslChap.pas` | yes (auto-created) | dead — created but unreachable |
| `TanzimPU.pas` | no | dead |
| `ToExcelU.pas` | no (`CreateForm` line commented out) | dead |

`Mainu.pas` has exactly one inbound reference (`FactorPrint3U`) and `Dmu.pas` is the root of everything, so neither appears here for the same reason.

### Units that are live but absent from `arzi.dpr`

Nine units are absent from `arzi.dpr` (126 − 117). Six of them are dead (listed above). The remaining three compile in only because some other unit `uses` them; they were simply never added to the IDE project file:

- `BankTanzim.pas` — used by `Mainu`
- `FinalU.pas` — used by `Mainu`
- `LibXL.pas` — used by `ToExcelU` (which is itself dead, so `LibXL` is dead by transitivity)

`arzi.dpr` lists 117 units; every `X in 'X.pas'` entry resolves to a file that exists — there are no dangling project references.

---

## 3. Units that appear unused or vestigial

Fourteen `.pas` units are recommended for **drop**, plus one orphan `.dfm`. Ten of the units are directly unreachable; three (`Get_Serial`, `GetPassword`, `Ghabz`) are reachable only from another dead unit; one (`testmainU`) is a vendor-side tool rather than an application feature.

| Unit | Evidence | Port it? |
|---|---|---|
| `KolSatateU.pas` | Name is a transposition typo of `KolStateU` (`Satate` vs `State`). Its entire implementation section is `procesdure TKolSatate.init;` — a misspelled `procedure` keyword, so the unit **cannot compile**. Not listed in `arzi.dpr`; zero reverse dependencies. `KolStateU.pas` has the identical Persian caption `وضعیت حسابهای کل`, a real implementation, a FastReport, and is used by `Mainu`. | **No.** Port `KolStateU` only. |
| `S_KolU.pas` | Caption `سرفصلهای حسابداری` is byte-identical to `SNewu.pas`. Both declare a chart-of-accounts grid over the same tables with the same helper set (`gets`, `GetN`, `Sarfasl_TakmilU`, `CodeNameU`, `initTaf1`). `SNewu` additionally has level promote/demote, lock, print and grid-font persistence. `S_KolU` is absent from `arzi.dpr` and referenced by nothing; `SNewu` is used by `Mainu`. | **No.** `SNewu` is the survivor. |
| `ToExcelU.pas` | Its `Application.CreateForm(TToExcel, ToExcel);` line exists in `arzi.dpr` but is **commented out**; the unit is not in the `.dpr` uses list and no unit `uses` it. `ToExcelDaraeiU.pas` has the identical caption `ذخیره سند در فایل اکسل`, is auto-created, and is the one `Mainu` calls. The difference is the export engine: `ToExcelU` drives the native `LibXL` DLL, `ToExcelDaraeiU` uses FastReport's XLS exporter plus OLE automation. | **No** — but note the `LibXL` path produced real `.xlsx`, which the rebuild should match with a Rust crate (`rust_xlsxwriter`). |
| `LibXL.pas` | 135 KB of hand-written `external 'libxl.dll'` declarations. Sole consumer is the dead `ToExcelU`. | **No.** Replace the capability, not the binding. |
| `RoozViewU.pas` | Whole unit is a `TForm` with an ADO query, a grid, and one `FormClose` that closes the query. Not in `arzi.dpr`, zero references. `RooznamehViewU.pas` (caption `نمايش اسناد روزنامه`) is the full-featured daybook browser that `Mainu` actually opens. Clearly a first sketch. | **No.** |
| `TanzimPU.pas` | Form has zero controls in the `.dfm` and zero code in the `.pas`. Caption is the default `TanzimP`. Not in `arzi.dpr`, zero references. Presumably an abandoned start on a "print settings" form later realised as `TanzimChapu`. | **No.** |
| `GetCodeStringU.pas` | Form has no controls beyond the defaults; its only two methods save and restore window geometry to the INI. Not in `arzi.dpr`, zero references. | **No.** |
| `DateFrameU.pas` | `TFrame` containing only Delphi's default `Label1` and `Edit1`. `TDateFrame` occurs in no other `.dfm`, so it is never embedded. It survives only because `Mainu`'s `uses` clause still names it. | **No.** |
| `Lab.pas` | Caption `ورود اطلاعات انس گذاري` ("ounce-grading data entry"). It **is** in `arzi.dpr` and **is** auto-created (`Application.CreateForm(TLabF, LabF)`), but no unit anywhere `uses Lab`, so `TLabF.New` can never be invoked — the form is instantiated and then orphaned. Its 91.7 KB `.dfm` is mostly an embedded FastReport template. | **No**, unless the pistachio grading workflow is explicitly in scope — in which case treat it as a fresh feature, not a port. |
| `Ghabz.pas` | `TFrame` rendering one weighbridge receipt. Embedded only in `Lab.dfm`. Dead with `Lab`. | **No.** |
| `Get_Serial.pas` | Weighbridge-receipt serial prompt validated against `DM.B_SelectSerial`. Referenced only by `Lab`. Dead with `Lab`. | **No.** |
| `GetPassword.pas` | Password prompt hashing through `SysInfo.ElfHash`. Referenced only by `Lab`. The live password flow is `GetPassu` (login) + `ChangePasswordU` (change). Dead with `Lab`. | **No.** |
| `SarfaslChap.pas` | Caption `لیست سرفصلها`. In `arzi.dpr` and auto-created, but no `uses` clause anywhere names it — same orphan pattern as `Lab`. `ListSarfaslu.pas` carries the same caption and is the live chart-of-accounts list. | **No.** |
| `testmainU.pas` | The entry form of the separate `test.dpr` project. Writes a machine-derived integer into `Base/CS3` of the INI — this is the licence-key generator, a vendor-side tool, not an end-user feature. It leaks into the main app only because `Mainu` `uses testmainU`. | **No.** Licensing is replaced wholesale by web auth. |
| `GetPass.dfm` | **A `.dfm` with no `.pas`.** The live login form is `GetPassu.pas` + `GetPassu.dfm` (class `TGetPass`, caption `فرم ورود به سيستم`). `GetPass.dfm` is a leftover from before the unit was renamed and is not compiled into anything (Delphi links `.dfm` by unit name, and no unit is named `GetPass`). | **No.** Delete. |

### Pairs that look like duplicates but are *not* — do port both

| Pair | Why they differ |
|---|---|
| `CheckDaryaftU` vs `CheckDaryaft2U` | Different captions and different business events: `دریافت چک` (receive a cheque from a party) vs `واگذاری چک به بانک` (hand a received cheque to a bank for collection). Both are referenced by `CheckListDU`, `FishListD` and `TasfiehFactor`. |
| `Print_Anbar15` vs `Print_Anbar16` | `Print_Anbar15` implements `print_Tolid` (goods-production document, caption `چاپ فرم تولید کالا`); `Print_Anbar16` implements `print_Jabejaei` (inter-warehouse relocation, caption `چاپ فاکتور`). Both are called from `SodoorSanadU` for different document types. |
| `FactorPrintU` / `Factorprint2U` / `FactorPrint3U` | Three distinct print templates, wired to three distinct menu items in `AnbarListU` (`AR_Chap`, `AR_Chap2`, `AR_Chap3`) and all three referenced by `Mainu`. `Factorprint2U` is the official/tax invoice (`چاپ فاکتور رسمی`); `FactorPrint3U` adds A4/A5 selection and direct printer control. In the rebuild these collapse into one report with three templates. |
| `PrintMU` vs `PrintM2U` | Two FastReport layouts for the same subsidiary voucher, both offered from `SanadEditU` and `Mainu` as separate print buttons. |
| `MakeSanadU` vs `MergeSanad` | `MakeSanadU` (26 KB) generates a consolidated voucher from selected invoices and is called by `SodoorSanadU`; `MergeSanad` (9 KB) merges two existing vouchers and is called by `SanadViewU`. Same caption `ادغام اسناد`, different operations. |
| `DMoein` vs `TMoein` | Same caption `نمایش دفتر معین`, but `TMoein` is the variant opened from the party card (`CardJariU`) while `DMoein` is the general-menu one and additionally exports `Get_FullCode`/`Get_SSn`/`Get_Valid` helpers used by `RoyatJU` and `CardJariU`. `TMoein` is a genuine near-clone — a merge candidate, not a drop. |
| `DaftarT_U` vs `TajmiU` | Both concern the aggregate ledger (`دفتر تجمیعی`), but `DaftarT_U` (11 KB) is the full browser with drill-down and reporting while `TajmiU` (3.9 KB) is a compact totals grid. Both are opened from `Mainu`. |
| `Sarfasl_SelectU` vs `SelectSarfasl` | Two account-head pickers with the same purpose and near-identical captions (`انتخاب سرفصل`). Split by generation: the older `Sarfasl_SelectU` is used by the inventory/treasury forms (12 callers), the newer `SelectSarfasl` by the ledger/voucher forms (7 callers). **Merge into one component in the rebuild.** |
| `EnteghalU` / `FinalU` / `NewFinalu` | Three forms all captioned `بستن حسابها` ("closing the accounts"), all three referenced only by `Mainu` — i.e. all three are still on the menu. `NewFinalu` is the newest (grid-driven, `rDBGrid`), `FinalU` the `TFGetCode`-frame generation, `EnteghalU` the balance-rollforward variant with progress reporting. Confirm with the accounting-core spec which one is authoritative before dropping the other two. |
| `CheckListU` vs `CheckListDU` | Both carry the stale caption `تنظیمات بانک` (copied from `BankTanzim`), but `CheckListU` is the *issued*-cheque register (opens `CheckEditU`) and `CheckListDU` is the *received*-cheque register (opens `CheckDaryaftU`, `CheckVosoolU`, `CheckEsterdadU`, `CheckBargashtu`). The rebuild must give them correct, distinct titles. |
| `KolStateU` vs `KolSatateU` | See the dead-code table — this one **is** a real duplicate; only `KolStateU` survives. |

---

## 4. Non-source assets inventory

### Files in the project root

| File | Size | What it is | Source or build output | Rebuild needs an equivalent? |
|---|---|---|---|---|
| `arzi.dpr` | 13.3 K | Delphi project file: the `uses` list of 117 units and 119 `Application.CreateForm` calls interleaved with `WaitF.Gotonextposition` splash ticks. | **source** | No — but it is the authoritative list of what shipped. Read it as the app manifest. |
| `arzi.dproj` | 41.9 K | MSBuild project (Delphi XE-era) holding compiler options and the unit search paths (`D:\Borland\fast report\…`, `D:\Borland\Alpha Control`, `D:\Embarcadero\AbsoluteDatabase\Source`). | **source** | No. Documents the third-party dependency locations. |
| `arzi.dproj.local` | 31.0 K | Per-developer IDE state (open files, breakpoints). | build/IDE output | No. |
| `arzi.cfg` | 1.3 K | Legacy `dcc32` compiler switch file (`-$A8`, `-$B-`, …). | build output (mirror of `.dof`) | No. |
| `arzi.dof` | 2.5 K | Delphi 6/7-era options file — the pre-`.dproj` form of the same settings. | build output | No. Evidence the codebase predates 2007. |
| `arzi.res` | 3.0 K | Compiled Win32 resource: application icon and version info. | build output (from `arzi_Icon.ico` + version block) | Only the icon — see below. |
| `arzi_Icon.ico` | 766 B | The application icon. | **source** | Yes — becomes the web app favicon / PWA icon. |
| `arzi.identcache` | 4.3 K | IDE symbol cache. | build output | No. |
| `arzi.stat` | 186 B | IDE usage statistics (`CompileSecs=3845739`, `StartTime=10/28/2017`). Confirms active development from at least 2017. | build output | No. |
| `arzi.ini` | 0 B | Empty — the runtime settings file, created on first run. | runtime data | Conceptually yes: user/app settings move to a database table. |
| `arzi.ini1` | 626 B | A saved/pristine copy of the settings file. Contains `[Base] Program=Green Gold`, the author's name and contact, and the encrypted licence fields `CS1`/`CS2`/`CS3`. | **source-ish** (seed config) | The `[Base]` product identity yes; the licence fields no. |
| `arzi.local.ini` | 2.4 K | A populated runtime settings file: licence fields plus per-form `Left/Top/Width/Height` sections (`[GetPass]`, `[CodeDate]`, `[GetN2N_F]`, …) and `ID`/`COID` (active user and company). | runtime data | Yes — this is the de-facto user-preferences schema. Worth reading before designing the settings table. |
| `arzi.abs` | 92.4 K | **Absolute Database** file (magic `ABS0LUTEDATABASE`). The backup/secondary datastore written by `Backup_U`. | data | The *capability* yes (export/backup); the format no. |
| `rppc.abs` | 84.4 K | A second Absolute Database file — the `RPPCSOLUTION` dataset the weighbridge code (`DM.ADO_RPPCSOLUTION`, `DM.B_SelectSerial`) reads from. | data | Only if the weighbridge/`Lab` workflow is in scope. |
| `Lib.dll` | 502.5 K | A native DLL of Persian input dialogs (`Get2Date`, `GetPassword`, `GetNewPassword`, …). | binary dependency | **No.** Its declarations are in `Lib.inc` and all but two are commented out; the surviving Delphi forms (`GetD`, `GetN`, `GetS`, `GetPassword`) replaced it. |
| `Lib.inc` | 1.6 K | The `external 'lib.dll'` declarations for the above — mostly commented out. | **source** | No. Historical evidence only; it is not `{$I}`-included by any unit. |
| `libxl.dll` | 6.5 M | The commercial LibXL native Excel writer, bound by `LibXL.pas`. | binary dependency | Replace the capability (XLSX export), not the DLL. |
| `example.xls` | 80.5 K | A LibXL sample workbook. | vendor sample | No. |
| `example.dproj` | 26.0 K | The LibXL vendor demo project, left in the tree. | vendor sample | No. |
| `test.dpr` | 320 B | Second, tiny project: `testmainU` + `LockUnit` + `INI` + `Utility`. The licence-key generator. | **source** | No — web auth replaces licensing entirely. |
| `test.dproj` / `test.dproj.local` / `test.cfg`-equivalents | 25.6 K / 933 B | Project files for the above. | build/IDE output | No. |
| `test.exe` | 10.7 M | Compiled licence-key generator. | build output | No. |
| `test.res` | 57.8 K | Its compiled resource. | build output | No. |
| `test.identcache` / `test.stat` | 170 B / 172 B | IDE caches for the test project. | build output | No. |
| `test.INI` | 23 B | Output of the key generator: `[Base] CS3=207687952`. | runtime data | No. |
| `SodoorSanadU.vlb` / `TasfiehFactor.vlb` | 663 B / 3.1 K | Delphi "visual layout of non-visual components" files — the x/y coordinates of `TADOQuery`/`TDataSource`/`TVirtualTable` icons on the form designer surface. Pure IDE cosmetics. | build/IDE output | No. |
| `tmp8EA4.tmp` | 1.9 K | Fragment of the Windows SDK `verrsrc.h` (`VS_FF_DEBUG`, `VOS_*` constants) left behind by the resource compiler. | build output | No. Delete. |
| `docs/` | — | Where this and the sibling specifications live. | **source** | — |
| `.remember/` | — | Tooling state, not part of the application. | — | No. |

**Net:** the only assets the rebuild genuinely inherits are `arzi_Icon.ico`, the `[Base]` product identity out of `arzi.ini1`, the per-form geometry schema visible in `arzi.local.ini`, and — if the weighbridge workflow is in scope — the data inside `rppc.abs`. Everything else is build output, IDE state, vendor samples, or licensing machinery that web authentication makes obsolete.

### Third-party libraries visible in the `uses` clauses

Counts are the number of project units that name at least one unit of that library.

| Library | Units seen (top) | Files touched | What it provides | Rust / React equivalent |
|---|---|---|---|---|
| **`Tools`** | `Tools` | 87 | Not a third-party product but a shared *external* unit — it is not in the project root and is resolved off the IDE search path. It supplies the Persian-calendar and numeric edit controls the whole app relies on (`TFarsi_Date`/`Farsi_Date`, `IntValue`/`IntLength` on edits, `SetToDate`). **Its source is not in this repository** and must be located separately, or its behaviour re-derived from usage. | A Jalali date library (`ptime`-style crate server-side; a Persian-calendar date picker client-side) plus formatted numeric inputs. |
| **AlphaControls** (`s*` and `ac*` prefixes) | `sPanel`, `sBitBtn`, `sLabel`, `sEdit`, `sSpeedButton`, `sSkinManager`, `sSkinProvider`, `acAlphaImageList`, `acPNG`, `acDBGrid`, `acProgressBar`, `acSelectSkin`, `sPageControl`, `sFrameBar`, … | ~110 | The skinning framework — every visible control is a skinned variant of a stock VCL control, with the skin chosen at runtime through `TsSkinManager` (`B_SkinClick` in `Mainu`). | Pure presentation. A React component library + CSS custom-property theming replaces the whole thing; the runtime skin picker becomes a theme selector. |
| **FastReport VCL** (`frx*`) | `frxClass`, `frxDBSet`, `frxExportXLS`, `frxExportXLSX`, `frxExportPDF`, `frxExportCSV`, `frxExportDOCX`, `frxExportXML`, `frxExportImage`, `frxExportBaseDialog` | 46 | Banded report engine. **Report templates are stored inline inside the `.dfm` files**, which is why report-heavy forms have enormous form files (`MoeinZipU.dfm` 5.9 MB, `PrintNu.dfm` 446 KB, `CardJariU.dfm` 435 KB). Each `frxDBDataset` binds a report band to a query on `TDM`. | Server-side rendering: a typed report model in Rust → HTML/CSS print stylesheets for preview and `printpdf`/`typst`/headless-Chromium for PDF; `rust_xlsxwriter` for XLSX; `csv` for CSV. The template extraction from those `.dfm` blobs is a discrete, sizeable work item. |
| **rDBGrid** (`rDBGrid`, `rDBGrid_MS`, `rDBGridSorter_ADO`, `rImprovedComps`) | `rDBGrid` | 47 | Enhanced right-to-left data grid with multi-select, column sorting against ADO, per-column fonts and the `GridFontSizeChangingEx` persistence hook that appears throughout. | A virtualised RTL data-grid component (TanStack Table + a virtualiser), with sort/filter pushed down to the API. |
| **ADO** (`Data.Win.ADODB`, `ADODB`, `AdoConEd`) | `TADOQuery`, `TADOStoredProc`, `TADOConnection` | ~100 | The data access layer — SQL Server via OLE DB, plus named stored procedures (`SP_Taraz4Setooni`, `B_SelectSerial`, `Anbar_Mandeh`, …) exposed as components on `TDM`. | `sqlx` or `sea-orm` against PostgreSQL. The stored procedures need auditing individually — several encode real business rules. |
| **Absolute Database** (`ABSMain`) | `ABSMain` | 3 (`Dmu`, `Backup_U`, `AnbarFactorU`) | Single-file embedded database used for backups (`arzi.abs`) and the weighbridge dataset (`rppc.abs`). | Nothing direct. Backup becomes `pg_dump`/logical backup; the embedded-file role disappears in a server architecture. |
| **LibXL** (`LibXL.pas` + `libxl.dll`) | `LibXL` | 1 (dead `ToExcelU`) | Native XLS/XLSX writer, commercial licence. | `rust_xlsxwriter`. |
| **Indy** (`IdHTTP`, `IdTCPClient`, `IdTCPConnection`, `IdBaseComponent`, `IdComponent`) | `IdHTTP` | 1 (`Mainu`) | HTTP client — used for a single outbound call from the main form (update check / licence ping). | `reqwest`, or delete outright. |
| **Excel OLE automation** (`ExcelXP`, `ComObj`) | `ComObj` | 4 (`MoeinZipU`, `ToExcelDaraeiU`, `Anbar_Amalkard`, `ListSarfaslu`) | Drives a locally installed Excel through COM to build worksheets — requires Excel on the client machine. | `rust_xlsxwriter` server-side; no client dependency. |
| **PropSave** (`PropSaveMain`) | `PropSaveMain` | 2 (`Dmu`, `SanadEditU`) | Persists component properties between sessions (complementing the manual INI geometry code). | Client-side layout state persisted per user. |
| **MemDS / VirtualTable** | `MemDS`, `VirtualTable` | ~8 | In-memory datasets used as scratch buffers for voucher lines and allocation grids before commit. | Ordinary client-side state; no library needed. |

---

## 5. Coverage confirmation

- **Total `.pas` files in the project root: 126.**
- **All 126 appear in the master table in section 1**, rows 1–126, in case-insensitive alphabetical order. This was verified mechanically: the sorted output of `ls *.pas` was diffed against the sorted list of table keys and the two are identical.
- **Every row is evidence-based.** For each unit the following were read: the `unit` header, the interface `uses` clause, every `T… = class(…)` declaration, the interface-section method names, and the decoded top-level `Caption` from the paired `.dfm`. Units with no obvious purpose from that alone (`Ghabz`, `DateFrameU`, `TanzimPU`, `RoozViewU`, `GetCodeStringU`, `SayMessage`, `YesOrNo`, `CodeNameU`, `Get2D`, `GetD`, `GetN`, `GetN2N`, `GetS`, `Get_Serial`, `GetPassword`, `KolSatateU`, `Lab`, `SarfaslChap`, `Sarfasl_ListU`, `Sarfasl_Kolu`, `InFile`, `testmainU`, `Admin`, `Backup_U`, `INI`, `LockUnit`) had their implementation sections read directly.
- **`.dfm` coverage:** 123 `.dfm` files. 122 pair with a `.pas`; the odd one out is `GetPass.dfm`, which has no unit (see section 3). The four form-less library units (`INI`, `LibXL`, `LockUnit`, `Utility`) correctly have no `.dfm`. One `.dfm` — `TankhahEdit.dfm` — is stored in Delphi's **binary `TPF0` format** rather than text; its caption (`ليست تنخواه`) was decoded from the binary stream.
- **Unclassified: none.** Every `.pas` file has been assigned a form class, a caption (or an explicit "no caption set" / "library unit" note), a description, a domain and a rebuild priority.
