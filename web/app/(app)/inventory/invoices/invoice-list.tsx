"use client";

// Step 5.9: `AnbarListU`'s equivalent (specs/05-inventory §13.5) — filter by type/status (unlike
// the legacy, which only ever filters by type), a real create action, links to the editor and
// the 5.7 settlement view.

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
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
    register,
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
      router.push(`/inventory/invoices/${result.id}`);
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  if (fiscalYearId === null) {
    return <p className="text-sm text-muted-foreground">{t("shell.noFiscalYear")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-end gap-3">
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.documentType")}</label>
          <select
            value={typeFilter}
            onChange={(e) => setTypeFilter(e.target.value as InventoryDocumentType | "")}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <option value="">{t("inventory.allTypes")}</option>
            {ALL_TYPES.map((ty) => (
              <option key={ty} value={ty}>
                {t(DOCUMENT_TYPE_LABEL[ty])}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("treasury.status")}</label>
          <select
            value={statusFilter}
            onChange={(e) => setStatusFilter(e.target.value as InventoryDocumentStatus | "")}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <option value="">{t("treasury.allStatuses")}</option>
            {ALL_STATUSES.map((s) => (
              <option key={s} value={s}>
                {t(STATUS_LABEL[s])}
              </option>
            ))}
          </select>
        </div>
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
                <tr key={d.id} className="border-b border-border last:border-0 hover:bg-muted">
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

      <form
        onSubmit={handleSubmit((values) => createMutation.mutate(values))}
        className="flex flex-wrap items-end gap-3 rounded-md border border-border p-3"
      >
        <h2 className="w-full text-sm font-semibold text-foreground">{t("inventory.newInvoice")}</h2>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.documentType")}</label>
          <select
            {...register("documentType", { onChange: (e) => setSelectedType(e.target.value as InventoryDocumentType) })}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            {ALL_TYPES.map((ty) => (
              <option key={ty} value={ty}>
                {t(DOCUMENT_TYPE_LABEL[ty])}
              </option>
            ))}
          </select>
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.invoiceDate")}</label>
          <input
            type="date"
            {...register("documentDate")}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </div>
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("inventory.warehouse")}</label>
          <select
            value={warehouseId ?? ""}
            onChange={(e) => setWarehouseId(e.target.value ? Number(e.target.value) : null)}
            className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          >
            <option value="">—</option>
            {warehouses?.map((w) => (
              <option key={w.id} value={w.id}>
                {w.name}
              </option>
            ))}
          </select>
        </div>
        {isTransfer && (
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("inventory.destinationWarehouse")}</label>
            <select
              value={destinationWarehouseId ?? ""}
              onChange={(e) => setDestinationWarehouseId(e.target.value ? Number(e.target.value) : null)}
              className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            >
              <option value="">—</option>
              {warehouses?.filter((w) => w.id !== warehouseId).map((w) => (
                <option key={w.id} value={w.id}>
                  {w.name}
                </option>
              ))}
            </select>
          </div>
        )}
        {isCommercial && <AccountField label={t("inventory.counterparty")} value={counterpartyId} onChangeAction={setCounterpartyId} />}
        <button
          type="submit"
          disabled={isSubmitting}
          className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
        >
          {t("common.create")}
        </button>
        {errors.root && (
          <p role="alert" className="w-full text-sm text-danger">
            {errors.root.message}
          </p>
        )}
      </form>
    </div>
  );
}
