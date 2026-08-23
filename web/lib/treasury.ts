// Shared types for the treasury screens (step 4.1-4.4's API / step 4.5's UI,
// docs/phase-4-treasury.md).

export type ChequeStatus =
  | "in_hand"
  | "at_bank"
  | "bounced"
  | "returned_to_issuer"
  | "cleared"
  | "endorsed_to_third_party";

export interface ChequeSummary {
  id: number;
  fiscalYearId: number;
  status: ChequeStatus;
  chequeNumber: string | null;
  receivedOn: string;
  dueDate: string;
  amount: number;
  description: string;
  payerAccountId: number;
  notesReceivableAccountId: number;
  issuingBank: string | null;
  issuingBranch: string | null;
  issuingAccountNumber: string | null;
  drawerName: string | null;
  depositedAt: string | null;
  clearedAt: string | null;
  bouncedAt: string | null;
  returnedAt: string | null;
  endorsedAt: string | null;
  voucherId: number | null;
}

export interface ChequeEvent {
  id: number;
  resultingStatus: ChequeStatus;
  eventDate: string;
  amount: number;
  debitAccountId: number | null;
  creditAccountId: number | null;
  description: string | null;
  voucherId: number | null;
}

export interface ChequeDetail extends ChequeSummary {
  events: ChequeEvent[];
}

export type DepositChannel = "pos_terminal" | "cash_slip" | "card_to_card" | "wire_transfer";

export interface DepositSlip {
  id: number;
  fiscalYearId: number;
  slipNumber: string | null;
  slipDate: string;
  amount: number;
  description: string | null;
  payerAccountId: number;
  bankAccountId: number;
  channel: DepositChannel;
  voucherId: number | null;
  sourceModule: number;
}

export interface PettyCashLine {
  id: number;
  expenseAccountId: number;
  amount: number;
  description: string | null;
}

export interface PettyCashClaimSummary {
  id: number;
  fiscalYearId: number;
  claimNumber: string | null;
  claimDate: string;
  description: string | null;
  custodianAccountId: number;
  totalAmount: number;
  lineCount: number;
  voucherId: number | null;
}

export interface PettyCashClaimDetail extends PettyCashClaimSummary {
  lines: PettyCashLine[];
}

export interface ChequeBatchLine {
  id: number;
  payeeAccountId: number;
  amount: number;
  description: string | null;
  payeeBankAccountNumber: string | null;
  payeeAccountHolderName: string | null;
}

export interface ChequeBatchSummary {
  id: number;
  fiscalYearId: number;
  batchNumber: string | null;
  issueDate: string;
  description: string;
  letterBody: string | null;
  bankAccountId: number;
  totalAmount: number;
  lineCount: number;
  voucherId: number | null;
}

export interface ChequeBatchDetail extends ChequeBatchSummary {
  lines: ChequeBatchLine[];
}
