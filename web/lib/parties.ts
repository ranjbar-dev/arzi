// Shared types for the party register screens (step 3.1/3.2's API, step
// 3.4's UI, docs/phase-3-parties.md).

export type PartyType = "natural_person" | "legal_entity";

export type TaxStatus =
  | "not_specified"
  | "taxpayer_required_to_register"
  | "natural_person_article_81"
  | "not_required_to_register"
  | "final_consumer";

export interface PartySummary {
  id: number;
  cardNumber: number;
  partyType: PartyType;
  firstName: string;
  lastName: string;
  fatherName: string | null;
  idCardNumber: string | null;
  birthDate: string | null;
  birthPlace: string | null;
  idIssueDate: string | null;
  idIssuePlace: string | null;
  nationalId: string | null;
  postalCode: string | null;
  registrationNumber: string | null;
  address: string | null;
  mobile: string | null;
  taxStatus: TaxStatus;
  isLocked: boolean;
}

export interface ControlAccountView {
  configId: number;
  name: string;
  controlKolCode: number;
  controlMoeinCode: number;
  fixedTafsil1Code: number;
  countsTowardBalance: boolean;
  ticked: boolean;
  accountId: number | null;
}

export interface PartyDetail extends PartySummary {
  controlAccounts: ControlAccountView[];
}

export interface AccountConfigRow {
  id: number;
  controlKolCode: number;
  controlMoeinCode: number;
  fixedTafsil1Code: number;
  name: string;
  forPerson: boolean;
  forLegalEntity: boolean;
  offeredByDefault: boolean;
  countsTowardBalance: boolean;
}

export interface ControlAccountBalance {
  configId: number;
  name: string;
  accountId: number;
  debit: number;
  credit: number;
  remainder: number;
}

export interface PartyBalance {
  total: number;
  breakdown: ControlAccountBalance[];
}
