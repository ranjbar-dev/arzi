"use client";

// Step 3.4 (docs/phase-3-parties.md §3.4): `SahamdarEditU`/`CompanyEditU`
// equivalent (specs/07-parties-and-shareholders/07-10.md §10.2/§10.3) — one
// shared form for both kinds (the legacy's two near-identical forms), the
// control-account tick grid computed per-request (no `SC_Tik`, the B18 fix
// already done server-side in 3.1 — this component just renders whatever
// the API says is ticked).

import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import type { AccountConfigRow, PartyDetail, PartyType, TaxStatus } from "@/lib/parties";

interface FormValues {
  cardNumber: number;
  firstName: string;
  lastName: string;
  fatherName: string;
  idCardNumber: string;
  mobile: string;
  birthDate: string;
  birthPlace: string;
  idIssueDate: string;
  idIssuePlace: string;
  nationalId: string;
  postalCode: string;
  registrationNumber: string;
  address: string;
  taxStatus: TaxStatus;
}

const TAX_STATUS_OPTIONS: TaxStatus[] = [
  "not_specified",
  "taxpayer_required_to_register",
  "natural_person_article_81",
  "not_required_to_register",
  "final_consumer",
];
const TAX_STATUS_KEY: Record<TaxStatus, string> = {
  not_specified: "parties.taxStatusNotSpecified",
  taxpayer_required_to_register: "parties.taxStatusTaxpayerRequiredToRegister",
  natural_person_article_81: "parties.taxStatusNaturalPersonArticle81",
  not_required_to_register: "parties.taxStatusNotRequiredToRegister",
  final_consumer: "parties.taxStatusFinalConsumer",
};

const ERROR_KEYS: Record<string, string> = {
  incomplete_data: "parties.incompleteData",
  duplicate_card_number: "parties.duplicateCardNumber",
  duplicate_national_id: "parties.duplicateNationalId",
  control_account_not_provisioned: "parties.controlAccountNotProvisioned",
};

const emptyValues: FormValues = {
  cardNumber: 0,
  firstName: "",
  lastName: "",
  fatherName: "",
  idCardNumber: "",
  mobile: "",
  birthDate: "",
  birthPlace: "",
  idIssueDate: "",
  idIssuePlace: "",
  nationalId: "",
  postalCode: "",
  registrationNumber: "",
  address: "",
  taxStatus: "not_specified",
};

interface ChecklistRow {
  configId: number;
  name: string;
  ticked: boolean;
}

function toFormValues(p: PartyDetail): FormValues {
  return {
    cardNumber: p.cardNumber,
    firstName: p.firstName,
    lastName: p.lastName,
    fatherName: p.fatherName ?? "",
    idCardNumber: p.idCardNumber ?? "",
    mobile: p.mobile ?? "",
    birthDate: p.birthDate ?? "",
    birthPlace: p.birthPlace ?? "",
    idIssueDate: p.idIssueDate ?? "",
    idIssuePlace: p.idIssuePlace ?? "",
    nationalId: p.nationalId ?? "",
    postalCode: p.postalCode ?? "",
    registrationNumber: p.registrationNumber ?? "",
    address: p.address ?? "",
    taxStatus: p.taxStatus,
  };
}

export function PartyForm({
  kind,
  partyId,
  onCloseAction,
}: {
  kind: PartyType;
  partyId: number | null;
  onCloseAction: () => void;
}) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const isEdit = partyId !== null;
  const onClose = onCloseAction;

  const { data: existing } = useQuery({
    queryKey: ["parties", partyId],
    queryFn: () => apiRequest<PartyDetail>(`/api/v1/parties/${partyId}`),
    enabled: isEdit,
  });
  const { data: allConfig } = useQuery({
    queryKey: ["parties", "account-config"],
    queryFn: () => apiRequest<AccountConfigRow[]>("/api/v1/parties/account-config"),
    enabled: !isEdit,
  });

  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({ values: existing ? toFormValues(existing) : isEdit ? undefined : emptyValues });

  const [selected, setSelected] = useState<Set<number> | null>(null);

  const checklist: ChecklistRow[] = useMemo(() => {
    if (isEdit) {
      return (existing?.controlAccounts ?? []).map((c) => ({ configId: c.configId, name: c.name, ticked: c.ticked }));
    }
    return (allConfig ?? [])
      .filter((c) => (kind === "legal_entity" ? c.forLegalEntity : c.forPerson) && c.offeredByDefault)
      .map((c) => ({ configId: c.id, name: c.name, ticked: false }));
  }, [isEdit, existing, allConfig, kind]);

  const currentTicks = selected ?? new Set(checklist.filter((c) => c.ticked).map((c) => c.configId));

  const mutation = useMutation({
    mutationFn: (values: FormValues) => {
      const body = {
        cardNumber: values.cardNumber,
        partyType: kind,
        firstName: values.firstName,
        lastName: values.lastName,
        fatherName: values.fatherName || null,
        idCardNumber: values.idCardNumber || null,
        birthDate: values.birthDate || null,
        birthPlace: values.birthPlace || null,
        idIssueDate: values.idIssueDate || null,
        idIssuePlace: values.idIssuePlace || null,
        nationalId: values.nationalId || null,
        postalCode: values.postalCode || null,
        registrationNumber: values.registrationNumber || null,
        address: values.address || null,
        mobile: values.mobile || null,
        taxStatus: values.taxStatus,
        controlAccountConfigIds: [...currentTicks],
      };
      return isEdit
        ? apiRequest(`/api/v1/parties/${partyId}`, { method: "PUT", body: JSON.stringify(body) })
        : apiRequest("/api/v1/parties", { method: "POST", body: JSON.stringify(body) });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["parties"] });
      onClose();
    },
    onError: (err: ApiError) => {
      setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") });
    },
  });

  const isPerson = kind !== "legal_entity";

  return (
    <div className="rounded-md border border-border bg-surface p-4">
      <h2 className="mb-3 text-sm font-medium text-foreground">
        {isEdit ? t("parties.editParty") : t("parties.newParty")}
      </h2>
      <form onSubmit={handleSubmit((v) => mutation.mutate(v))} className="flex flex-col gap-4">
        <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
          <Field label={t("parties.cardNumber")}>
            <input
              type="number"
              disabled={isEdit}
              {...register("cardNumber", { valueAsNumber: true })}
              className="h-9 w-full rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
            />
          </Field>
          <Field label={isPerson ? t("parties.firstName") : t("parties.entityName")}>
            <input {...register("firstName")} autoFocus className={inputCls} />
          </Field>
          <Field label={isPerson ? t("parties.lastName") : t("parties.representative")}>
            <input {...register("lastName")} className={inputCls} />
          </Field>
          {isPerson && (
            <Field label={t("parties.fatherName")}>
              <input {...register("fatherName")} className={inputCls} />
            </Field>
          )}
          <Field label={t("parties.idCardNumber")}>
            <input {...register("idCardNumber")} className={inputCls} />
          </Field>
          <Field label={t("parties.mobile")}>
            <input {...register("mobile")} className={inputCls} />
          </Field>
          <Field label={isPerson ? t("parties.birthDate") : t("parties.incorporationDate")}>
            <input placeholder="YYYY/MM/DD" {...register("birthDate")} className={inputCls} />
          </Field>
          <Field label={t("parties.birthPlace")}>
            <input {...register("birthPlace")} className={inputCls} />
          </Field>
          {isPerson && (
            <>
              <Field label={t("parties.idIssueDate")}>
                <input placeholder="YYYY/MM/DD" {...register("idIssueDate")} className={inputCls} />
              </Field>
              <Field label={t("parties.idIssuePlace")}>
                <input {...register("idIssuePlace")} className={inputCls} />
              </Field>
            </>
          )}
          <Field label={isPerson ? t("parties.nationalId") : t("parties.entityNationalId")}>
            <input {...register("nationalId")} className={inputCls} />
          </Field>
          <Field label={t("parties.postalCode")}>
            <input {...register("postalCode")} className={inputCls} />
          </Field>
          <Field label={t("parties.registrationNumber")}>
            <input {...register("registrationNumber")} className={inputCls} />
          </Field>
          <Field label={t("parties.taxStatus")}>
            <select {...register("taxStatus")} className={inputCls}>
              {TAX_STATUS_OPTIONS.map((s) => (
                <option key={s} value={s}>
                  {t(TAX_STATUS_KEY[s])}
                </option>
              ))}
            </select>
          </Field>
          <Field label={t("parties.address")} wide>
            <input {...register("address")} className={inputCls} />
          </Field>
        </div>

        <div>
          <h3 className="mb-1 text-sm font-medium text-foreground">{t("parties.controlAccounts")}</h3>
          <div className="max-h-56 overflow-y-auto rounded-md border border-border">
            <table className="w-full text-sm">
              <thead>
                <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                  <th className="w-16 px-3 py-2 text-center font-medium">{t("parties.member")}</th>
                  <th className="px-3 py-2 text-start font-medium">{t("parties.groupName")}</th>
                </tr>
              </thead>
              <tbody>
                {checklist.map((row) => (
                  <tr key={row.configId} className="border-b border-border last:border-0">
                    <td className="px-3 py-1.5 text-center">
                      <input
                        type="checkbox"
                        checked={currentTicks.has(row.configId)}
                        onChange={(e) => {
                          const next = new Set(currentTicks);
                          if (e.target.checked) next.add(row.configId);
                          else next.delete(row.configId);
                          setSelected(next);
                        }}
                        className="h-4 w-4 accent-accent"
                      />
                    </td>
                    <td className="px-3 py-1.5 text-foreground">{row.name}</td>
                  </tr>
                ))}
                {checklist.length === 0 && (
                  <tr>
                    <td colSpan={2} className="px-3 py-4 text-center text-muted-foreground">
                      {t("parties.noHoldings")}
                    </td>
                  </tr>
                )}
              </tbody>
            </table>
          </div>
        </div>

        {errors.root && (
          <p role="alert" className="text-sm text-danger">
            {errors.root.message}
          </p>
        )}

        <div className="flex gap-3">
          <button
            type="submit"
            disabled={isSubmitting}
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
          >
            {t("common.save")}
          </button>
          <button
            type="button"
            onClick={onClose}
            className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.cancel")}
          </button>
        </div>
      </form>
    </div>
  );
}

const inputCls =
  "h-9 w-full rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent";

function Field({ label, wide, children }: { label: string; wide?: boolean; children: React.ReactNode }) {
  return (
    <div className={`flex flex-col gap-1 ${wide ? "col-span-2 sm:col-span-3" : ""}`}>
      <label className="text-sm text-muted-foreground">{label}</label>
      {children}
    </div>
  );
}
