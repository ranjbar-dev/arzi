"use client";

// Step 5.9: `AnbarListU`'s equivalent (specs/05-inventory §13.5) — filter by type/status (unlike
// the legacy, which only ever filters by type), a real create action, links to the editor and
// the 5.7 settlement view.

import { useState } from "react";
import { Controller, useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import { DateField } from "@/components/date-field";
import { Select } from "@/components/select";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field } from "@/components/form-field";
import type { InventoryDocument, InventoryDocumentStatus, InventoryDocumentType, Warehouse } from "@/lib/inventory";
import { COMMERCIAL_TYPES, DOCUMENT_TYPE_LABEL, STATUS_LABEL } from "@/lib/inventory";

const ALL_TYPES: InventoryDocumentType[] = ["receipt", "issue", "purchase_return", "sales_return", "production", "transfer"];
const ALL_STATUSES: InventoryDocumentStatus[] = ["draft", "posted", "frozen"];

const schema = z.object({
  documentType: z.enum(["receipt", "issue", "purchase_return", "sales_return", "production", "transfer"]),
  documentDate: z.string().min(1),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  counterparty_required: "inventory.counterpartyRequired",
  counterparty_account_not_found: "inventory.counterpartyRequired",
  counterparty_account_not_leaf: "inventory.counterpartyRequired",
  date_outside_fiscal_year: "vouchers.dateOutsideFiscalYear",
  fiscal_year_closed: "fiscalYears.alreadyClosed",
  destination_warehouse_required: "inventory.destinationWarehouseRequired",
  destination_warehouse_must_differ: "inventory.destinationWarehouseMustDiffer",
  insufficient_permission: "common.error",
};

export function InvoiceList({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const [typeFilter, setTypeFilter] = useState<InventoryDocumentType | "">("");
  const [statusFilter, setStatusFilter] = useState<InventoryDocumentStatus | "">("");
  const [warehouseId, setWarehouseId] = useState<number | null>(null);
  const [destinationWarehouseId, setDestinationWarehouseId] = useState<number | null>(null);
  const [counterpartyId, setCounterpartyId] = useState<number | null>(null);
  const [createOpen, setCreateOpen] = useState(false);

  const { data: warehouses } = useQuery({
    queryKey: ["warehouses"],
    queryFn: () => apiRequest<Warehouse[]>("/api/v1/warehouses?activeOnly=true"),
  });

  const params = new URLSearchParams();
  if (fiscalYearId) params.set("fiscalYearId", String(fiscalYearId));
  if (typeFilter) params.set("documentType", typeFilter);
  if (statusFilter) params.set("status", statusFilter);

  const { data: documents, isLoading } = useQuery({
    queryKey: ["inventory-documents", fiscalYearId, typeFilter, statusFilter],
    queryFn: () => apiRequest<InventoryDocument[]>(`/api/v1/inventory-documents?${params.toString()}`),
    enabled: fiscalYearId !== null,
  });

  const [selectedType, setSelectedType] = useState<InventoryDocumentType>("receipt");
  const {
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({
    resolver: zodResolver(schema),
    defaultValues: { documentType: "receipt" },
  });
  const isCommercial = COMMERCIAL_TYPES.includes(selectedType);
  const isTransfer = selectedType === "transfer";

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) => {
      if (!warehouseId) throw new Error("warehouse_required");
      return apiRequest<{ id: number }>("/api/v1/inventory-documents", {
        method: "POST",
        body: JSON.stringify({
          ...values,
          fiscalYearId,
          warehouseId,
          counterpartyAccountId: isCommercial ? counterpartyId : undefined,
          destinationWarehouseId: isTransfer ? destinationWarehouseId : undefined,
        }),
      });
    },
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["inventory-documents"] });
      setCreateOpen(false);
      router.push(`/inventory/invoices/${result.id}`);
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  if (fiscalYearId === null) {
    return <p className="text-sm text-muted-foreground">{t("shell.noFiscalYear")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end justify-between gap-3">
        <div className="flex flex-wrap items-end gap-3">
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("inventory.documentType")}</label>
            <Select
              value={typeFilter}
              onChangeAction={(v) => setTypeFilter(v as InventoryDocumentType | "")}
              placeholder={t("inventory.allTypes")}
              className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
              options={ALL_TYPES.map((ty) => ({ value: ty, label: t(DOCUMENT_TYPE_LABEL[ty]) }))}
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("treasury.status")}</label>
            <Select
              value={statusFilter}
              onChangeAction={(v) => setStatusFilter(v as InventoryDocumentStatus | "")}
              placeholder={t("treasury.allStatuses")}
              className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
              options={ALL_STATUSES.map((s) => ({ value: s, label: t(STATUS_LABEL[s]) }))}
            />
          </div>
        </div>
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("inventory.newInvoice")}</NewButton>
      </div>

      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("inventory.invoiceNumber")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.invoiceDate")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.documentType")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("treasury.status")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("inventory.total")}</th>
              </tr>
            </thead>
            <tbody>
              {documents?.map((d) => (
                <tr
                  key={d.id}
                  onClick={() => router.push(`/inventory/invoices/${d.id}`)}
                  className="cursor-pointer border-b border-border last:border-0 hover:bg-muted"
                >
                  <td className="px-3 py-2">
                    <Link
                      href={`/inventory/invoices/${d.id}`}
                      className="text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                    >
                      {toPersianDigits(d.documentNumber)}
                    </Link>
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">{d.documentDate}</td>
                  <td className="px-3 py-2 text-muted-foreground">{t(DOCUMENT_TYPE_LABEL[d.documentType])}</td>
                  <td className="px-3 py-2 text-muted-foreground">{t(STATUS_LABEL[d.status])}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(d.totalAmount)}</td>
                </tr>
              ))}
              {documents?.length === 0 && (
                <tr>
                  <td colSpan={5} className="px-3 py-6 text-center text-sm text-muted-foreground">
                    —
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("inventory.newInvoice")}>
        <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <Field label={t("inventory.documentType")}>
              <Controller
                name="documentType"
                control={control}
                render={({ field }) => (
                  <Select
                    value={field.value ?? ""}
                    onChangeAction={(v) => {
                      field.onChange(v);
                      setSelectedType(v as InventoryDocumentType);
                    }}
                    className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                    options={ALL_TYPES.map((ty) => ({ value: ty, label: t(DOCUMENT_TYPE_LABEL[ty]) }))}
                  />
                )}
              />
            </Field>
            <Controller
              name="documentDate"
              control={control}
              render={({ field }) => (
                <DateField label={t("inventory.invoiceDate")} value={field.value ?? ""} onChangeAction={field.onChange} />
              )}
            />
            <Field label={t("inventory.warehouse")}>
              <Select
                value={warehouseId ? String(warehouseId) : ""}
                onChangeAction={(v) => setWarehouseId(v ? Number(v) : null)}
                placeholder="—"
                className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                options={(warehouses ?? []).map((w) => ({ value: String(w.id), label: w.name }))}
              />
            </Field>
            {isTransfer && (
              <Field label={t("inventory.destinationWarehouse")}>
                <Select
                  value={destinationWarehouseId ? String(destinationWarehouseId) : ""}
                  onChangeAction={(v) => setDestinationWarehouseId(v ? Number(v) : null)}
                  placeholder="—"
                  className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  options={(warehouses ?? []).filter((w) => w.id !== warehouseId).map((w) => ({ value: String(w.id), label: w.name }))}
                />
              </Field>
            )}
            {isCommercial && <AccountField label={t("inventory.counterparty")} value={counterpartyId} onChangeAction={setCounterpartyId} />}
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
              {t("common.create")}
            </button>
            <button
              type="button"
              onClick={() => setCreateOpen(false)}
              className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("common.cancel")}
            </button>
          </div>
        </form>
      </Modal>
    </div>
  );
}
