_Part of [05-inventory](../05-inventory.md) — [index](00-index.md)_

### 16.9 Document types

| Legacy | Persian | Proposed `document_type` | Direction |
|---|---|---|---|
| `AF_Type = 1` | `رسید انبار` | `receipt` | in |
| `AF_Type = 2` | `حواله انبار` / `فاکتور فروش کالاو خدمات` | `issue` | out |
| `AF_Type = 3` | `برگشت از خرید` | `purchase_return` | out |
| `AF_Type = 4` | `برگشت از فروش` | `sales_return` | in |
| `FM_ID = 11` | `اول دوره` | `opening_stock` | in |
| `FM_ID = 12` | `رسید انبار` | `purchase_receipt` | in |
| `FM_ID = 13` | `برگشت از فروش` | `sales_return` | in |
| `FM_ID = 14` | `خرید پسته` | `pistachio_purchase_receipt` | in |
| `FM_ID = 15` | `تولید` | `production_output` | in |
| `FM_ID = 16` | `جابجایی` | `transfer_in` | in |
| `FM_ID = 21` | *(unknown)* | — | out — Q30 |
| `FM_ID = 22` | `حواله انبار` / `فروش` | `sales_issue` | out |
| `FM_ID = 25` | `تولید` | `production_input` | out |
| `FM_ID = 26` | `جابجایی` | `transfer_out` | out |
| *(new)* | — | `adjustment` | either — §15 B4 |

### 16.10 Voucher `M_Id` values originating in inventory

| `M_Id` | Source | Proposed `voucher_lines.source_kind` |
|---|---|---|
| `1` (range `1..9` reserved) | subsystem A invoice, all four types | `inventory_document` |
| `31` | `FM_ID = 11` opening stock | `inventory_document` |
| `32` | `FM_ID = 12` purchase | `inventory_document` |
| `33` | `FM_ID = 22` sale | `inventory_document` |
| `34` | `FM_ID = 14` pistachio purchase | `inventory_document` |
| `35` | `FM_ID = 13` sales return | `inventory_document` |
| `36`–`39` | reserved, unused | — |

After §15 C2 the `M_Id` family disappears entirely: a voucher line points at its source document
by `source_document_id`, and the document knows its own type.

### 16.11 Screens → routes

| Legacy unit | Proposed route | Component |
|---|---|---|
| `AnbarTanzimU` | `/inventory/warehouses` | `WarehouseSettings` |
| `AnbarCalaU` | `/inventory/items` | `ItemList` |
| `AnbarCalaAddU` | `/inventory/items/:id` | `ItemEditor` |
| `AnbarCalaSelectU` | *(modal)* | `ItemPicker` |
| `AnbarListU` | `/inventory/documents` | `DocumentList` |
| `AnbarFactorU` | `/inventory/documents/:id` | `DocumentEditor` |
| `AnbarFactorAddU` | *(modal)* | `DocumentLineEditor` |
| `Anbar_MandehU` | `/inventory/reports/stock-balance` | `StockBalanceReport` |
| `AnbarCardJensiU` | `/inventory/reports/item-ledger-card` | `ItemLedgerCard` |
| `Anbar_Amalkard` | `/inventory/reports/movements` | `MovementReport` |
| `AnbarReportU` | *(folded into `MovementReport` after C1)* | |
| `TasfiehFactor` | `/inventory/documents/:id/settlement` | `DocumentSettlement` |
| `Factorprint2U` | `/inventory/documents/:id/print/official` | `OfficialInvoicePrint` |
| `FactorPrint3U` | `/inventory/documents/:id/print` | `InvoicePrint` |
| `SodoorSanadU` | `/inventory/documents/posting` | `PostingQueue` |
| `MakeSanadU` | *(modal)* | `VoucherPreview` |
| `Print_Anbar15` / `Print_Anbar16` | `/inventory/documents/:id/print/production` / `/transfer` | |
| `FactorPesteh_U` | `/pistachio/deliveries` | `PistachioDeliveryList` |
| `PestehD_U` | *(modal / service function)* | `PistachioDeductionCalculator` |
| `AnbarReportKharidU`, `FactorPrintU`, `Kharid_U`, `Kharid_BU`, `Lab`, `Ghabz`, `Get_Serial` | — | **not ported — unreachable in the legacy application (§13.0)** |

### 16.12 Rust module layout

```
inventory/
    items          -- §1.2, §2
    warehouses     -- §1.1
    units          -- §1.3
    documents      -- §3, §4
    lines          -- §3.1.2
    stock          -- §5, §11   (the one balance function, §15 C5)
    costing        -- §6        (the one advisory-cost function)
    pricing        -- §7
    posting        -- §10       (the one posting engine, §15 D)
    settlement     -- §9
    pistachio/
        grades     -- §8.1
        deductions -- §8.2      (the formula, as a pure function)
        deliveries -- §8.3
```

### 16.13 Persian terms used in this document

Beyond `docs/01-glossary.md` §2, these appeared here and are recorded for completeness.

| Persian | Transliteration | English |
|---|---|---|
| `اول دوره` | Aval Doreh | opening period / opening stock |
| `جابجایی` | Jabejaei | (inter-warehouse) transfer |
| `تولید` | Tolid | production |
| `حواله انبار` | Havaleh Anbar | goods issue note |
| `رسید انبار` | Resid Anbar | goods receipt note |
| `قبض` | Ghabz | ticket / receipt note (weighbridge) |
| `باسکول` | Bascul | weighbridge |
| `رمز` / `کشف رمز` | Ramz / Kashf-e Ramz | blind code / code revealed |
| `عدل` | Adl | bale (of pistachios) |
| `انس` | Ons | ounce (a pistachio sizing measure: nuts per ounce) |
| `دهن بست` | Dahan-Bast | closed-shell (non-split) nuts |
| `گرم مغز` | Garam-e Maghz | kernel weight in grams |
| `رطوبت` | Rotubat | moisture |
| `پوک` | Pook | blank / empty shell |
| `کسورات` | Kosoorat | deductions |
| `کسر ظرف` | Kasr-e Zarf | tare (container) deduction |
| `خالص` | Khales | net |
| `ناخالص` | Nakhales | gross |
| `فی` | Fi | unit price |
| `مانده منفی` | Mandeh-ye Manfi | negative balance |
| `تحریر` | Tahrir | draft / editable state (of a voucher) |
| `بایگانی` | Bayegani | archived |
| `مشمول مالیات` | Mashmool-e Maliat | subject to VAT |
| `شناسه مالیاتی` | Shenase-ye Maliati | tax identifier |
| `حد اقل موجودی` | Had-e Aghal-e Mojoodi | minimum stock level |
| `مشخصه فنی` | Moshakhase-ye Fanni | technical specification |
| `واحد شمارش` | Vahed-e Shomaresh | unit of measure |
| `متوسط قیمت خرید` | Motevasset-e Gheymat-e Kharid | average purchase price |
| `متوسط قیمت تمام شده` | Motevasset-e Gheymat-e Tamam-Shodeh | average cost price |
| `صورتحساب` | Soorat-Hesab | (official) invoice |
| `پیش فاکتور` | Pish-Factor | proforma invoice |
| `طرف حساب` | Taraf-e Hesab | counterparty |
| `فیش واریزی` | Fish-e Varizi | deposit slip |
| `چک موعدی` | Chek-e Moedi | post-dated cheque |


---

[← 16. Naming map (part a)](05-16-a-naming-map.md) | [index](00-index.md) | _end_
