// Shared types for the voucher screens (step 2.3's API / step 2.4's UI,
// docs/phase-2-accounting-core.md).

export type VoucherStatus = "draft" | "confirmed" | "posted";

export interface VoucherSummary {
  id: number;
  fiscalYearId: number;
  voucherNumber: number;
  voucherDate: string;
  description: string | null;
  totalDebit: number;
  totalCredit: number;
  lineCount: number;
  status: VoucherStatus;
  isLocked: boolean;
}

export interface VoucherLine {
  id: number;
  voucherId: number;
  accountId: number;
  debitAmount: number;
  creditAmount: number;
  quantity: string;
  description: string | null;
  status: VoucherStatus;
  sourceModule: number; // 0 = manual, editable; anything else = generated, read-only
}

export interface VoucherDetail extends VoucherSummary {
  lines: VoucherLine[];
}
