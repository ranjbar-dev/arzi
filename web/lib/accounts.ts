// Shared types for the chart-of-accounts screens and the account picker
// (step 2.1's API / step 2.2's UI, docs/phase-2-accounting-core.md).

export interface AccountSummary {
  id: number;
  generalLedgerCode: number;
  subsidiaryCode: number;
  analytic1Code: number;
  analytic2Code: number;
  name: string;
  childCount: number;
  isLocked: boolean;
  level: 1 | 2 | 3 | 4;
  isActive: boolean;
}

export interface AccountDetail extends AccountSummary {
  codeLtr: string;
  codeRtl: string;
  fullNamePath: string;
  lineName: string;
}

export function ownCode(account: AccountSummary): number {
  switch (account.level) {
    case 1:
      return account.generalLedgerCode;
    case 2:
      return account.subsidiaryCode;
    case 3:
      return account.analytic1Code;
    case 4:
      return account.analytic2Code;
  }
}

export const LEVEL_LABELS = ["kol", "moein", "tafsil1", "tafsil2"] as const;
