_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

## 16. Naming map

Legacy identifier → proposed English name. Conventions from `docs/01-glossary.md` §7:
PostgreSQL `snake_case`, plural tables, singular columns, `id` primary key, `<singular>_id`
foreign keys. Names already fixed by `docs/02-data-model.md` are reused, not reinvented.

### 16.1 Tables

| Legacy | Subsystem | Proposed | Note |
|---|---|---|---|
| `Anbar_Config` | A | `warehouses` | merged with `Anbar.dbo.Anbar` under §15 C1 |
| `Anbar.dbo.Anbar` | B | `warehouses` | |
| `Anbar_Jens` | A | `items` | merged with `Cala` under C1 |
| `Anbar.dbo.Cala` | B | `items` | |
| *(new)* | — | `item_warehouses` | replaces `Cala.C_Anbar`, the comma-delimited list |
| `Anbar_Vahed` | A | `units_of_measure` | |
| `Kinds` | A | `pistachio_grades` | **not** account types — §8.1 |
| `Anbar_Factor` | A | `inventory_documents` | merged with `FactorMaster` under C1 |
| `Anbar.dbo.FactorMaster` | B | `inventory_documents` | |
| `Anbar_FactorD` | A | `inventory_document_lines` | merged with `FactorDetail` under C1 |
| `Anbar.dbo.FactorDetail` | B | `inventory_document_lines` | |
| `Anbar.dbo.FactorKind` | B | `document_types` | or a Rust enum — §3.4 |
| `Rppc_Solution.dbo.NewRamz` | weighbridge | `pistachio_deliveries` | |
| `Base` (`Kh1_Code` … `Kh8_Code`) | A | `pistachio_account_settings` | only if §15 makes the accounts configurable; otherwise drop — §8.5 |
| `Moadian` | — | `tax_einvoice_submissions` | read-only from this domain — §10.4 |
| `Moein` / `DMoein` | — | `voucher_lines` / `vouchers` | see `docs/03-accounting-core.md` |
| `Sarfasl` | — | `accounts` | a counterparty **is** a leaf account — `docs/01-glossary.md` §6b |
| `DFish` / `DCheck` | — | `deposit_slips` / `cheques` | see `docs/06-treasury.md` |

### 16.2 `Anbar_Config` → `warehouses`

| Legacy | Proposed | Type |
|---|---|---|
| `AC_ID` | `id` | serial |
| `AC_Name` | `name` | text |
| `AC_DMaliat` | `vat_rate_pct` | numeric(5,2) |
| `AC_Kharid` | `purchase_account_id` | fk → `accounts` |
| `AC_BKharid` | `purchase_return_account_id` | fk |
| `AC_Foroosh` | `sales_account_id` | fk |
| `AC_BForoosh` | `sales_return_account_id` | fk |
| `AC_Kasr` | `discount_account_id` | fk |
| `AC_Maliat` | `vat_account_id` | fk |
| `A_Code` (subsystem B) | `id` | |
| `A_Aval` | `opening_stock_account_id` | fk |
| `A_Kharid` / `A_Foroosh` / `A_BForoosh` / `A_Kasr` / `A_Maliat` | as above | |
| *(new, §15 B3-adjacent)* | `is_active` | boolean |

### 16.3 `Anbar_Jens` → `items`

| Legacy | Proposed | Note |
|---|---|---|
| *(new)* | `id` | surrogate; `AJ_Code` becomes an attribute — §15 C2 |
| `AJ_Code` | `code` | unique |
| `AJ_ID` | `warehouse_id` | fk; becomes `item_warehouses` under C1 |
| `AJ_Name` | `name` | |
| `AJ_Prop` | `specification` | |
| `AJ_Vahed` | *(dropped)* | denormalised label; join instead |
| `AJ_VahedC` | `unit_of_measure_id` | fk |
| `AJ_Phi` | `sale_price` | **not cost** — §6.0 |
| `AJ_Alarm` | `min_stock` | |
| `AJ_Maliat` | `is_taxable` | boolean |
| `AJ_Manfi` | `allow_negative_stock` | boolean, **default true** — §2.2.2 |
| `AJ_UserID` | `updated_by` | |
| `AJ_Net` | `updated_from_host` | |
| `AJ_DateTime` | `updated_at` | |
| `SSTID` | `tax_item_code` | the 13-char tax-authority identifier |
| `AJ_PhiS` (view column) | *(dropped)* | formatting belongs in the UI |
| *(new)* | `is_active` | §15 B3 |
| `C_Code` / `C_Name` / `C_Prop` / `C_Vahed` (subsystem B) | `code` / `name` / `specification` / `unit_of_measure_id` | |
| `C_Anbar` | *(dropped)* | → `item_warehouses` |

### 16.4 `Anbar_Vahed` → `units_of_measure` · `Kinds` → `pistachio_grades`

| Legacy | Proposed |
|---|---|
| `AV_Code` | `id` |
| `AV_Name` | `name` |
| `K_id` | `id` |
| `K_name` | `name` |

### 16.5 `Anbar_Factor` / `FactorMaster` → `inventory_documents`

| Legacy A | Legacy B | Proposed | Note |
|---|---|---|---|
| `AF_SSN` | `FM_SSN` | `id` | |
| `AF_COID` | `FM_COID` | `fiscal_year_id` | **fiscal year, not company** |
| `AF_Type` | `FM_ID` | `document_type` | see §16.9 |
| — | `FM_InOut` | *(dropped)* | derivable from `document_type` |
| `AF_Factor` | `FM_Factor` | `document_number` | mutable today; a plain unique attribute after C2 |
| `AF_Date` | `FM_Date` | `document_date` + `document_date_jalali` | |
| — | `FM_Anbar` | `warehouse_id` | subsystem A has none today — §1.0 |
| `AF_Customer` | `FM_TSSN` | `party_account_id` | fk → `accounts` |
| `AF_CustomerN` | `FM_TName` | *(dropped)* | denormalised |
| — | `FM_TCode` | *(dropped)* | denormalised account code |
| `AF_Desc` | `FM_Desc` | `description` | |
| `AF_Sanad` | `FM_SanadNo` | `voucher_id` | fk after C2 |
| — | `FM_SanadDate` | *(dropped)* | derivable from the voucher |
| — | `FM_Lock` | `status` | `draft` / `posted` / `reversed` — §15 C3 |
| `AF_Mab` | `FM_Mab` | `gross_amount` | ⚠ `Mab` is **gross** at header level |
| `AF_Kasr` | `FM_Kasr` | `discount_amount` | |
| `AF_Maliat` | `FM_Maliat` | `tax_amount` | |
| `AF_Total` | `FM_Total` | `total_amount` | net |
| — | `FM_Count` | *(dropped)* | derivable |
| — | `FM_Link` | `counterpart_document_id` | self-fk; transfer 16↔26, production 15↔25 — §3.2.3 |
| — | `FM_UserID` | `created_by` | |
| — | `FM_LUserID` | `updated_by` | never written today |
| — | `FM_LDate` | `updated_at` | never written today |
| `AF_Sel2`–`AF_Sel5`, `AF_Mab1`–`AF_Mab5`, `AF_Desc1`–`AF_Desc5`, `AF_Date1`–`AF_Date5` | — | *(dropped)* | twenty dead columns — §3.1.1, Q9 |
| *(new)* | | `settled_amount`, `outstanding_amount`, `settlement_status` | §15 B6 |

### 16.6 `Anbar_FactorD` / `FactorDetail` → `inventory_document_lines`

| Legacy A | Legacy B | Proposed | Note |
|---|---|---|---|
| `AFD_SSN` | `FD_SSN` | `id` | plus an immutable `sequence_no` — §15 C4 |
| `AFD_Factor` (+`AFD_Coid`) | `FD_FMSSN` | `document_id` | subsystem A links by number today |
| `AFD_Coid` | — | *(dropped)* | on the header |
| `AFD_Type` | `FD_InOut` | *(dropped)* | denormalised from the header |
| `AFD_Date` | — | *(dropped)* | denormalised from the header |
| `AFD_Customer` | — | *(dropped)* | denormalised and drift-prone — §13.10, §15 A4 |
| — | `FD_Anbar` | *(dropped)* | on the header |
| `AFD_Code` | `FD_Code` | `item_id` | fk |
| `AFD_Name` | `FD_CodeN` | *(dropped)* | denormalised |
| `AFD_Prop` | `FD_CodeP` | *(dropped)* | denormalised |
| `AFD_Vahed` | `FD_CodeV` | *(dropped)* | denormalised |
| `AFD_Num` | `FD_Num` | `quantity` | numeric(14,3), always positive |
| — | `FD_VaznP` | `weight` | numeric(14,3); subsystem A has no equivalent |
| `AFD_Phi` | `FD_Phi` | `unit_price` | |
| `AFD_Kol` | `FD_Mab` | `gross_amount` | ⚠ `Kol` here, `Mab` there, same meaning |
| `AFD_Kasr` | `FD_Kasr` | `discount_amount` | |
| `AFD_Maliat` | `FD_Maliat` | `tax_amount` | |
| `AFD_Total` | `FD_Total` | `total_amount` | net |
| `AFD_UserID` | — | `created_by` | |

> **The `Mab` / `Kol` trap, stated once.** At **header** level `AF_Mab` is gross and `AF_Total`
> is net. At **line** level `AFD_Kol` is gross and `AFD_Total` is net. Subsystem B uses `FM_Mab`
> and `FD_Mab` for gross at both levels. So `Mab` and `Kol` mean the same thing at different
> levels of the same subsystem. Both become `gross_amount`.

### 16.7 `NewRamz` → `pistachio_deliveries`

| Legacy | Proposed | Note |
|---|---|---|
| `NR_Serial` | `id` | |
| `NR_Ghabz` | `weighbridge_ticket_number` | |
| `NR_GhabzDate` | `weighbridge_ticket_date` | |
| `NR_Ramz` | `blind_code` | the lab anonymisation code |
| `NR_Date` | `lab_date` | |
| `NR_State` | `status` | 1–5, §8.3.2 |
| `NR_Jari` | `supplier_account_segment` | third segment of `301-1-<n>` |
| `NR_Name` | `supplier_name` | denormalised |
| `NR_Kind` | `grade_id` | fk → `pistachio_grades` — §8.1.1 |
| `NR_KindName` | *(dropped)* | denormalised |
| `NR_Adl` | `bale_count` | |
| `NR_P1` | `ounce_count` | `انس` |
| `NR_P2` | `closed_shell_pct` | `دهن بست` |
| `NR_P3`–`NR_P12`, `NR_P1G`, `NR_P2V`, `NR_P2VV` | *(unknown)* | never read — Q22 |
| `NR_Vazn1` | `gross_weight` | |
| `NR_Vazn2` | `tare_weight` | |
| `NR_Vazn3`, `NR_Vazn4`, `NR_Vazn5` | *(inferred: `moisture_deduction`, `blank_deduction`, `other_deduction`)* | **positional inference only** — Q22 |
| `NR_Vazn` | `net_weight` | the only weight `arzi` uses |
| `NR_Phi` | `unit_price` | |
| `NR_Kol` | `total_amount` | |
| `NR_Factor` | `purchase_invoice_number` | |
| `NR_FDate` | `purchase_invoice_date` | the **voucher** date |
| `NR_Resid` | `warehouse_receipt_number` | written back by `arzi` |
| `NR_Sanad` | `voucher_id` | written back by `arzi` |
| `User1`, `User2`, `User3` | `stage1_user_id`, `stage2_user_id`, `stage3_user_id` | never read |

### 16.8 The pistachio deduction calculator (`KharidRec` / `PestehD_U`)

| Legacy | Persian | Proposed | §8.2 role |
|---|---|---|---|
| `KindId` / `KindName` | `نوع پسته` | `grade_id` | |
| `Ons` | `انس` | `ounce_count` | descriptive only |
| `Dahan` | `دهن بست` | `closed_shell_pct` | descriptive only |
| `Garam` | `گرم مغز` | `kernel_grams` | descriptive only |
| `Adl` | `تعداد` | `bale_count` | |
| `Adlv` | `100 گرم` / `200 گرم` / `یک کیلو` | `tare_allowance_per_bale_kg` | 0.1 / 0.2 / 1.0 |
| `Adlk` | `کسر ظرف` | `tare_deduction_kg` | derived |
| `Rot` | `درصد رطوبت` | `moisture_pct` | |
| `RotV` | `کسر رطوبت` | `moisture_deduction_kg` | derived |
| `Pook` | `درصد پوک` | `blank_pct` | |
| `PookV` | `کسر پوک` | `blank_deduction_kg` | derived |
| `Sayer` | `سایر کسورات` | `other_deduction_kg` | **kg, not %** |
| `Kasr` | `جمع کسورات` | `total_deduction_kg` | derived |
| `BascV` | `وزن باسکول` | `gross_weight_kg` | |
| `NabV` | `خالص وزن` | `net_weight_kg` | derived, floored at 0 |
| `Phi` | `بهای واحد` | `unit_price` | |
| `Kol` | `مبلغ کل` | `total_amount` | derived |


---

[← 15. PROPOSED IMPROVEMENTS (needs user approval)](05-15-proposed-improvements.md) | [index](00-index.md) | [16. Naming map (part b) →](05-16-b-naming-map.md)
