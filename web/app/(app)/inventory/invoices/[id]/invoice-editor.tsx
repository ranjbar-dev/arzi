"use client";

// Step 5.9 (docs/phase-5-inventory.md §5.9): `AnbarFactorU`'s equivalent — header + line grid,
// counterparty (required per 5.2's B7 fix, commercial types only), an item picker reused from
// 5.9's own `ItemPicker`, quantity/price/discount entry, a live running total (the document's own
// incrementally-maintained totals, same "live in the sense the architecture supports" framing as
// 2.4's voucher editor), a real average-cost-suggestion button (5.4 — replacing the legacy's
// undiscoverable click-a-read-only-label interaction), the pistachio deduction calculator as a
// first-class step for pistachio-grade items (5.6 — the actual B19 reachability fix), a post
// action (5.8) and an inline settlement panel (5.7).

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountLabel } from "@/components/account-label";
import { ItemPicker } from "@/components/item-picker";
import { ItemLabel } from "@/components/item-label";
import type {
  AverageCost,
  Item,
  InventoryDocumentDetail,
  PistachioDeductionResult,
  SettlementView,
  Warehouse,
} from "@/lib/inventory";
import { COMMERCIAL_TYPES, DOCUMENT_TYPE_LABEL, STATUS_LABEL } from "@/lib/inventory";

const ERROR_KEYS: Record<string, string> = {
  invalid_quantity: "inventory.quantity",
  ambiguous_discount_entry: "common.error",
  invalid_discount_percent: "common.error",
  item_not_found: "common.error",
  not_draft: "inventory.notDraft",
  document_frozen: "inventory.documentFrozen",
  counterparty_required: "inventory.counterpartyRequired",
  bale_count_required: "inventory.baleCountRequired",
  gross_weight_required: "inventory.grossWeightRequired",
  unit_price_required: "inventory.unitPriceRequired",
  invalid_tare_allowance: "common.error",
  item_not_pistachio_grade: "common.error",
  pistachio_purchase_only: "common.error",
  finished_goods_account_not_configured: "common.error",
  raw_materials_account_not_configured: "common.error",
  source_inventory_account_not_configured: "common.error",
  destination_inventory_account_not_configured: "common.error",
  posting_account_not_found: "common.error",
  posting_account_not_leaf: "vouchers.accountNotLeaf",
  insufficient_permission: "common.error",
};

const lineSchema = z.object({
  quantity: z.coerce.string().min(1),
  unitPrice: z.coerce.number().int(),
  discountAmount: z.coerce.number().int().default(0),
  taxAmount: z.coerce.number().int().default(0),
});
type LineFormValues = z.input<typeof lineSchema>;
type LineFormOutput = z.output<typeof lineSchema>;

const pistachioSchema = z.object({
  baleCount: z.coerce.number().int(),
  tareAllowanceKg: z.enum(["0.1", "0.2", "1.0"]),
  grossWeightKg: z.coerce.string().min(1),
  moisturePct: z.coerce.string().default("0"),
  blankPct: z.coerce.string().default("0"),
  otherDeductionsKg: z.coerce.string().default("0"),
  unitPrice: z.coerce.number().int(),
});
type PistachioFormValues = z.input<typeof pistachioSchema>;
type PistachioFormOutput = z.output<typeof pistachioSchema>;

export function InvoiceEditor({ documentId }: { documentId: number }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const [selectedItem, setSelectedItem] = useState<Item | null>(null);
  const [pistachioPreview, setPistachioPreview] = useState<PistachioDeductionResult | null>(null);
  const [lastPistachioInput, setLastPistachioInput] = useState<PistachioFormOutput | null>(null);

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["inventory-documents", documentId] });

  const { data: document, isLoading } = useQuery({
    queryKey: ["inventory-documents", documentId],
    queryFn: () => apiRequest<InventoryDocumentDetail>(`/api/v1/inventory-documents/${documentId}`),
  });
  const { data: warehouses } = useQuery({
    queryKey: ["warehouses"],
    queryFn: () => apiRequest<Warehouse[]>("/api/v1/warehouses"),
  });
  const isCommercial = document ? COMMERCIAL_TYPES.includes(document.documentType) : false;
  const editable = document ? document.status !== "frozen" : false;
  const isPistachioItem = selectedItem?.pistachioGradeId != null;

  const { data: averageCost } = useQuery({
    queryKey: ["items", selectedItem?.id, "average-cost", document?.fiscalYearId, document?.documentDate],
    queryFn: () =>
      apiRequest<AverageCost>(
        `/api/v1/items/${selectedItem!.id}/average-cost?fiscalYearId=${document!.fiscalYearId}&asOfDate=${document!.documentDate}&excludeDocumentId=${documentId}`,
      ),
    enabled: !!selectedItem && !!document,
  });

  const { data: settlement } = useQuery({
    queryKey: ["inventory-documents", documentId, "settlement"],
    queryFn: () => apiRequest<SettlementView>(`/api/v1/inventory-documents/${documentId}/settlement`),
    enabled: isCommercial,
  });

  const lineForm = useForm<LineFormValues, unknown, LineFormOutput>({
    resolver: zodResolver(lineSchema),
    defaultValues: { discountAmount: 0, taxAmount: 0 },
  });

  const addLineMutation = useMutation({
    mutationFn: (values: LineFormOutput) => {
      if (!selectedItem) throw new Error("item_not_found");
      return apiRequest<{ id: number }>(`/api/v1/inventory-documents/${documentId}/lines`, {
        method: "POST",
        body: JSON.stringify({ itemId: selectedItem.id, ...values }),
      });
    },
    onSuccess: () => {
      invalidate();
      lineForm.reset({ discountAmount: 0, taxAmount: 0 });
      setSelectedItem(null);
    },
    onError: (err: ApiError) => lineForm.setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const pistachioForm = useForm<PistachioFormValues, unknown, PistachioFormOutput>({
    resolver: zodResolver(pistachioSchema),
    defaultValues: { tareAllowanceKg: "0.2", moisturePct: "0", blankPct: "0", otherDeductionsKg: "0" },
  });

  const calculateMutation = useMutation({
    mutationFn: (values: PistachioFormOutput) => apiRequest<PistachioDeductionResult>("/api/v1/pistachio-deduction/calculate", {
      method: "POST",
      body: JSON.stringify(values),
    }),
    onSuccess: (result, variables) => {
      setPistachioPreview(result);
      setLastPistachioInput(variables);
    },
    onError: (err: ApiError) => pistachioForm.setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const addPistachioLineMutation = useMutation({
    mutationFn: (values: PistachioFormOutput) => {
      if (!selectedItem) throw new Error("item_not_found");
      return apiRequest<{ id: number }>(`/api/v1/inventory-documents/${documentId}/lines/pistachio`, {
        method: "POST",
        body: JSON.stringify({ itemId: selectedItem.id, ...values }),
      });
    },
    onSuccess: () => {
      invalidate();
      pistachioForm.reset({ tareAllowanceKg: "0.2", moisturePct: "0", blankPct: "0", otherDeductionsKg: "0" });
      setSelectedItem(null);
      setPistachioPreview(null);
      setLastPistachioInput(null);
    },
    onError: (err: ApiError) => pistachioForm.setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const deleteLineMutation = useMutation({
    mutationFn: (lineId: number) =>
      apiRequest<void>(`/api/v1/inventory-documents/${documentId}/lines/${lineId}`, { method: "DELETE" }),
    onSuccess: invalidate,
  });

  const postMutation = useMutation({
    mutationFn: () => apiRequest<void>(`/api/v1/inventory-documents/${documentId}/post`, { method: "POST" }),
    onSuccess: invalidate,
  });

  const deleteDocumentMutation = useMutation({
    mutationFn: () => apiRequest<void>(`/api/v1/inventory-documents/${documentId}`, { method: "DELETE" }),
    onSuccess: () => router.push("/inventory/invoices"),
  });

  if (isLoading || !document) {
    return <p className="text-sm text-muted-foreground">{t("common.loading")}</p>;
  }
  const warehouseName = (id: number) => warehouses?.find((w) => w.id === id)?.name ?? "—";

  return (
    <div className="flex flex-col gap-6">
      <div className="flex flex-wrap items-center justify-between gap-3">
        <div className="flex items-center gap-3">
          <h1 className="text-lg font-semibold text-foreground">
            {t(DOCUMENT_TYPE_LABEL[document.documentType])} — {toPersianDigits(document.documentNumber)}
          </h1>
          <span className="rounded-full bg-muted px-2 py-0.5 text-xs text-muted-foreground">
            {t(STATUS_LABEL[document.status])}
          </span>
        </div>
        <div className="flex gap-2">
          {editable && (
            <button
              type="button"
              onClick={() => postMutation.mutate()}
              disabled={postMutation.isPending}
              className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
            >
              {t("inventory.post")}
            </button>
          )}
          {editable && (
            <button
              type="button"
              onClick={() => deleteDocumentMutation.mutate()}
              disabled={deleteDocumentMutation.isPending}
              className="h-9 cursor-pointer rounded-md border border-danger px-4 text-sm font-medium text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
            >
              {t("common.delete")}
            </button>
          )}
        </div>
      </div>

      <div className="grid grid-cols-2 gap-3 rounded-md border border-border p-3 text-sm sm:grid-cols-4">
        <div>
          <div className="text-muted-foreground">{t("inventory.invoiceDate")}</div>
          <div className="text-foreground">{document.documentDate}</div>
        </div>
        <div>
          <div className="text-muted-foreground">{t("inventory.warehouse")}</div>
          <div className="text-foreground">{warehouseName(document.warehouseId)}</div>
        </div>
        {document.destinationWarehouseId && (
          <div>
            <div className="text-muted-foreground">{t("inventory.destinationWarehouse")}</div>
            <div className="text-foreground">{warehouseName(document.destinationWarehouseId)}</div>
          </div>
        )}
        {document.counterpartyAccountId && (
          <div>
            <div className="text-muted-foreground">{t("inventory.counterparty")}</div>
            <AccountLabel accountId={document.counterpartyAccountId} />
          </div>
        )}
        {document.postedVoucherId && (
          <div>
            <div className="text-muted-foreground">{t("inventory.postedVoucher")}</div>
            <div className="tabular-nums text-foreground">{toPersianDigits(document.postedVoucherId)}</div>
          </div>
        )}
      </div>

      <div className="overflow-x-auto rounded-md border border-border">
        <table className="w-full text-sm">
          <thead>
            <tr className="border-b border-border bg-muted/50 text-muted-foreground">
              <th className="px-3 py-2 text-start font-medium">{t("inventory.itemName")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("inventory.quantity")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("inventory.unitPrice")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("inventory.discount")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("inventory.tax")}</th>
              <th className="px-3 py-2 text-start font-medium">{t("inventory.total")}</th>
              {editable && <th className="px-3 py-2" />}
            </tr>
          </thead>
          <tbody>
            {document.lines.map((line) => (
              <tr key={line.id} className="border-b border-border last:border-0 hover:bg-muted">
                <td className="px-3 py-2 text-foreground">
                  <ItemLabel itemId={line.itemId} />
                </td>
                <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(line.quantity)}</td>
                <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(line.unitPrice)}</td>
                <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(line.discountAmount)}</td>
                <td className="tabular-nums px-3 py-2 text-muted-foreground">{toPersianDigits(line.taxAmount)}</td>
                <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(line.totalAmount)}</td>
                {editable && (
                  <td className="px-3 py-2">
                    <button
                      type="button"
                      onClick={() => deleteLineMutation.mutate(line.id)}
                      className="cursor-pointer text-danger hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                    >
                      {t("common.delete")}
                    </button>
                  </td>
                )}
              </tr>
            ))}
            {document.lines.length === 0 && (
              <tr>
                <td colSpan={7} className="px-3 py-6 text-center text-sm text-muted-foreground">
                  —
                </td>
              </tr>
            )}
          </tbody>
          <tfoot>
            <tr className="border-t border-border bg-muted/30 font-medium text-foreground">
              <td className="px-3 py-2" colSpan={5}>
                {t("inventory.total")}
              </td>
              <td className="tabular-nums px-3 py-2">{toPersianDigits(document.totalAmount)}</td>
              {editable && <td />}
            </tr>
          </tfoot>
        </table>
      </div>

      {editable && (
        <div className="flex flex-col gap-3 rounded-md border border-border p-3">
          <h2 className="text-sm font-semibold text-foreground">{t("inventory.addLine")}</h2>
          <div className="flex flex-wrap items-center gap-3">
            <ItemPicker
              triggerLabel={selectedItem ? selectedItem.name : t("inventory.itemName")}
              onSelectAction={setSelectedItem}
            />
            {selectedItem && isPistachioItem && (
              <span className="rounded-full bg-accent/10 px-2 py-0.5 text-xs text-accent">
                {t("inventory.pistachioCalculator")}
              </span>
            )}
          </div>

          {selectedItem && !isPistachioItem && (
            <form
              onSubmit={lineForm.handleSubmit((values) => addLineMutation.mutate(values))}
              className="flex flex-wrap items-end gap-3"
            >
              <div className="flex flex-col gap-1">
                <label className="text-sm text-muted-foreground">{t("inventory.quantity")}</label>
                <input
                  type="number"
                  step="0.001"
                  {...lineForm.register("quantity")}
                  className="h-9 w-28 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-sm text-muted-foreground">{t("inventory.unitPrice")}</label>
                <input
                  type="number"
                  {...lineForm.register("unitPrice")}
                  className="h-9 w-32 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              </div>
              {averageCost && averageCost.averageCost > 0 && (
                <button
                  type="button"
                  onClick={() => lineForm.setValue("unitPrice", averageCost.averageCost)}
                  className="h-9 cursor-pointer rounded-md border border-border bg-surface px-3 text-sm text-accent hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("inventory.useAverageCost")}: {toPersianDigits(averageCost.averageCost)}
                </button>
              )}
              <div className="flex flex-col gap-1">
                <label className="text-sm text-muted-foreground">{t("inventory.discount")}</label>
                <input
                  type="number"
                  {...lineForm.register("discountAmount")}
                  className="h-9 w-28 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              </div>
              <div className="flex flex-col gap-1">
                <label className="text-sm text-muted-foreground">{t("inventory.tax")}</label>
                <input
                  type="number"
                  {...lineForm.register("taxAmount")}
                  className="h-9 w-28 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                />
              </div>
              <button
                type="submit"
                disabled={addLineMutation.isPending}
                className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
              >
                {t("inventory.addLine")}
              </button>
              {lineForm.formState.errors.root && (
                <p role="alert" className="w-full text-sm text-danger">
                  {lineForm.formState.errors.root.message}
                </p>
              )}
            </form>
          )}

          {selectedItem && isPistachioItem && (
            <form
              onSubmit={pistachioForm.handleSubmit((values) => calculateMutation.mutate(values))}
              className="flex flex-col gap-3"
            >
              <div className="flex flex-wrap items-end gap-3">
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.baleCount")}</label>
                  <input
                    type="number"
                    {...pistachioForm.register("baleCount")}
                    className="h-9 w-24 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.tareAllowance")}</label>
                  <select
                    {...pistachioForm.register("tareAllowanceKg")}
                    className="h-9 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  >
                    <option value="0.1">{t("inventory.tareAllowance100g")}</option>
                    <option value="0.2">{t("inventory.tareAllowance200g")}</option>
                    <option value="1.0">{t("inventory.tareAllowance1kg")}</option>
                  </select>
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.grossWeight")}</label>
                  <input
                    type="number"
                    step="0.1"
                    {...pistachioForm.register("grossWeightKg")}
                    className="h-9 w-28 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.moisturePct")}</label>
                  <input
                    type="number"
                    step="0.01"
                    {...pistachioForm.register("moisturePct")}
                    className="h-9 w-24 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.blankPct")}</label>
                  <input
                    type="number"
                    step="0.01"
                    {...pistachioForm.register("blankPct")}
                    className="h-9 w-24 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.otherDeductions")}</label>
                  <input
                    type="number"
                    step="0.1"
                    {...pistachioForm.register("otherDeductionsKg")}
                    className="h-9 w-28 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("inventory.unitPrice")}</label>
                  <input
                    type="number"
                    {...pistachioForm.register("unitPrice")}
                    className="h-9 w-32 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <button
                  type="submit"
                  disabled={calculateMutation.isPending}
                  className="h-9 cursor-pointer rounded-md border border-border bg-surface px-4 text-sm text-accent hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("inventory.calculate")}
                </button>
              </div>

              {pistachioPreview && (
                <div className="grid grid-cols-2 gap-2 rounded-md bg-muted/40 p-3 text-sm sm:grid-cols-3">
                  <div>
                    <span className="text-muted-foreground">{t("inventory.tareDeduction")}: </span>
                    {toPersianDigits(pistachioPreview.tareDeductionKg)}
                  </div>
                  <div>
                    <span className="text-muted-foreground">{t("inventory.moistureDeduction")}: </span>
                    {toPersianDigits(pistachioPreview.moistureDeductionKg)}
                  </div>
                  <div>
                    <span className="text-muted-foreground">{t("inventory.blankDeduction")}: </span>
                    {toPersianDigits(pistachioPreview.blankDeductionKg)}
                  </div>
                  <div>
                    <span className="text-muted-foreground">{t("inventory.totalDeduction")}: </span>
                    {toPersianDigits(pistachioPreview.totalDeductionKg)}
                  </div>
                  <div>
                    <span className="text-muted-foreground">{t("inventory.netWeight")}: </span>
                    {toPersianDigits(pistachioPreview.netWeightKg)}
                  </div>
                  <div>
                    <span className="text-muted-foreground">{t("inventory.lineAmount")}: </span>
                    {toPersianDigits(pistachioPreview.lineAmount)}
                  </div>
                  <button
                    type="button"
                    onClick={() => lastPistachioInput && addPistachioLineMutation.mutate(lastPistachioInput)}
                    disabled={addPistachioLineMutation.isPending || !lastPistachioInput}
                    className="col-span-full h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
                  >
                    {t("inventory.addPistachioLine")}
                  </button>
                </div>
              )}
              {pistachioForm.formState.errors.root && (
                <p role="alert" className="text-sm text-danger">
                  {pistachioForm.formState.errors.root.message}
                </p>
              )}
            </form>
          )}
        </div>
      )}

      {isCommercial && settlement && (
        <div className="flex flex-col gap-3 rounded-md border border-border p-3">
          <h2 className="text-sm font-semibold text-foreground">{t("inventory.settlement")}</h2>
          <div className="flex flex-wrap gap-6 text-sm">
            <div>
              <span className="text-muted-foreground">{t("inventory.total")}: </span>
              {toPersianDigits(settlement.invoiceTotal)}
            </div>
            <div>
              <span className="text-muted-foreground">{t("inventory.settledTotal")}: </span>
              {toPersianDigits(settlement.settledTotal)}
            </div>
            <div className={settlement.outstandingAmount < 0 ? "text-danger" : "text-foreground"}>
              <span className="text-muted-foreground">{t("inventory.outstanding")}: </span>
              {toPersianDigits(settlement.outstandingAmount)}
            </div>
          </div>
          {settlement.instruments.length === 0 ? (
            <p className="text-sm text-muted-foreground">{t("inventory.noInstruments")}</p>
          ) : (
            <div className="overflow-x-auto rounded-md border border-border">
              <table className="w-full text-sm">
                <thead>
                  <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                    <th className="px-3 py-2 text-start font-medium">{t("inventory.documentType")}</th>
                    <th className="px-3 py-2 text-start font-medium">{t("treasury.eventDate")}</th>
                    <th className="px-3 py-2 text-start font-medium">{t("treasury.amount")}</th>
                    <th className="px-3 py-2 text-start font-medium">{t("treasury.description")}</th>
                  </tr>
                </thead>
                <tbody>
                  {settlement.instruments.map((i) => (
                    <tr key={`${i.kind}-${i.id}`} className="border-b border-border last:border-0">
                      <td className="px-3 py-2 text-foreground">
                        {i.kind === "deposit_slip" ? t("treasury.depositSlipsTitle") : t("treasury.receivedChequesTitle")}
                      </td>
                      <td className="px-3 py-2 text-muted-foreground">{i.date}</td>
                      <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(i.amount)}</td>
                      <td className="px-3 py-2 text-muted-foreground">{i.description ?? "—"}</td>
                    </tr>
                  ))}
                </tbody>
              </table>
            </div>
          )}
        </div>
      )}
    </div>
  );
}
