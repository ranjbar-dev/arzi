// Step 5.9 (docs/phase-5-inventory.md §5.9): shared TS types for the inventory screens, matching
// the JSON shapes api/src/items.rs, inventory_documents.rs, stock.rs, pistachio.rs and
// settlement.rs already serve (5.1-5.8).

export interface Warehouse {
  id: number;
  name: string;
  vatRatePct: string; // BigDecimal serialises as a string
  purchaseAccountId: number;
  purchaseReturnAccountId: number;
  salesAccountId: number;
  salesReturnAccountId: number;
  discountAccountId: number;
  vatAccountId: number;
  isActive: boolean;
  finishedGoodsAccountId: number | null;
  rawMaterialsAccountId: number | null;
  inventoryAccountId: number | null;
}

export interface UnitOfMeasure {
  id: number;
  name: string;
  baseUnitId: number | null;
  conversionFactor: string;
}

export interface PistachioGrade {
  id: number;
  name: string;
  sortOrder: number;
}

export interface Item {
  id: number;
  code: number;
  name: string;
  specification: string | null;
  unitOfMeasureId: number;
  salePrice: number;
  minStock: number;
  isTaxable: boolean;
  allowNegativeStock: boolean;
  taxItemCode: string | null;
  isActive: boolean;
  pistachioGradeId: number | null;
}

export interface ItemDetail extends Item {
  warehouseIds: number[];
}

export type InventoryDocumentType = "receipt" | "issue" | "purchase_return" | "sales_return" | "production" | "transfer";
export type InventoryDocumentStatus = "draft" | "posted" | "frozen";

export interface InventoryDocument {
  id: number;
  fiscalYearId: number;
  documentType: InventoryDocumentType;
  status: InventoryDocumentStatus;
  documentNumber: number;
  documentDate: string;
  warehouseId: number;
  counterpartyAccountId: number | null;
  description: string | null;
  grossAmount: number;
  discountAmount: number;
  taxAmount: number;
  totalAmount: number;
  postedVoucherId: number | null;
  destinationWarehouseId: number | null;
}

export interface InventoryDocumentLine {
  id: number;
  documentId: number;
  itemId: number;
  quantity: string;
  unitPrice: number;
  grossAmount: number;
  discountAmount: number;
  taxAmount: number;
  totalAmount: number;
  description: string | null;
}

export interface InventoryDocumentDetail extends InventoryDocument {
  lines: InventoryDocumentLine[];
}

export interface AverageCost {
  averageCost: number;
  purchaseQuantity: string;
}

export interface PistachioDeductionResult {
  tareDeductionKg: string;
  moistureDeductionKg: string;
  blankDeductionKg: string;
  totalDeductionKg: string;
  netWeightKg: string;
  lineAmount: number;
}

export interface SettledInstrument {
  kind: "deposit_slip" | "received_cheque";
  id: number;
  date: string;
  amount: number;
  description: string | null;
  referenceNumber: string | null;
}

export interface SettlementView {
  invoiceTotal: number;
  settledTotal: number;
  outstandingAmount: number;
  instruments: SettledInstrument[];
}

export const DOCUMENT_TYPE_LABEL: Record<InventoryDocumentType, string> = {
  receipt: "inventory.typeReceipt",
  issue: "inventory.typeIssue",
  purchase_return: "inventory.typePurchaseReturn",
  sales_return: "inventory.typeSalesReturn",
  production: "inventory.typeProduction",
  transfer: "inventory.typeTransfer",
};

export const COMMERCIAL_TYPES: InventoryDocumentType[] = ["receipt", "issue", "purchase_return", "sales_return"];

export const STATUS_LABEL: Record<InventoryDocumentStatus, string> = {
  draft: "inventory.statusDraft",
  posted: "inventory.statusPosted",
  frozen: "inventory.statusFrozen",
};
