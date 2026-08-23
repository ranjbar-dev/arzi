-- Step 5.8 (docs/phase-5-inventory.md §5.8 / specs/05-inventory/05-10-a/b-accounting-integration.md):
-- every inventory document type posts a correctly balanced voucher through the Phase 2.5 engine —
-- fixing B1 (purchase/opening-stock imbalance, §10.2.5's `2·discount − VAT` finding) and B2
-- (production/transfer post nothing at all, "Not implemented yet.", §3.2.4/§10.5) structurally.

-- B2: production and transfer get real, explicit posting rules — a genuinely new design (no
-- legacy precedent recoverable from source, per the roadmap's own "define them explicitly" ask).
ALTER TYPE inventory_document_type ADD VALUE 'production';
ALTER TYPE inventory_document_type ADD VALUE 'transfer';

-- New warehouse posting-account roles for the two new types — neither has a legacy equivalent
-- (subsystem A's Anbar_Config only ever had the six commercial roles, §1.1); nullable, since not
-- every warehouse does production or is a transfer endpoint.
ALTER TABLE warehouses ADD COLUMN finished_goods_account_id bigint REFERENCES accounts(id);
ALTER TABLE warehouses ADD COLUMN raw_materials_account_id  bigint REFERENCES accounts(id);
ALTER TABLE warehouses ADD COLUMN inventory_account_id      bigint REFERENCES accounts(id);

-- production posts Dr finished_goods_account_id / Cr raw_materials_account_id on the document's
-- own warehouse (no counterparty, no second warehouse). transfer is a wash entry between two
-- warehouses' inventory_account_id -- needs a second warehouse reference this table never had.
ALTER TABLE inventory_documents ADD COLUMN destination_warehouse_id bigint REFERENCES warehouses(id);

-- B7 (5.2) required a counterparty for every document -- correct for the four commercial types,
-- but production/transfer are internal movements with no external party at all. Relaxed to
-- nullable, with the CHECK re-imposing the original NOT NULL requirement everywhere it still
-- applies -- the B7 fix is preserved exactly for receipt/issue/purchase_return/sales_return, not
-- weakened.
ALTER TABLE inventory_documents ALTER COLUMN counterparty_account_id DROP NOT NULL;
ALTER TABLE inventory_documents ADD CONSTRAINT inventory_documents_counterparty_required_unless_internal
    CHECK (document_type::text IN ('production', 'transfer') OR counterparty_account_id IS NOT NULL);
