"use client";

// Step 5.9: `AnbarTanzimU`'s equivalent (specs/05-inventory §13.1) — a list plus a create form
// with the six mandatory posting-account links (5.1) and the three optional production/transfer
// roles (5.8), and a real deactivate action (5.1's own fix for the legacy's dead "N2" handler).

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";
import { DataTable, FilterInput, useDebounced, useSort } from "@/components/data-table";
import type { Warehouse } from "@/lib/inventory";

const schema = z.object({
  name: z.string().min(1),
  vatRatePct: z.coerce.string().default("0"),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const REQUIRED_ROLES = [
  ["purchaseAccountId", "inventory.purchaseAccount"],
  ["purchaseReturnAccountId", "inventory.purchaseReturnAccount"],
  ["salesAccountId", "inventory.salesAccount"],
  ["salesReturnAccountId", "inventory.salesReturnAccount"],
  ["discountAccountId", "inventory.discountAccount"],
  ["vatAccountId", "inventory.vatAccount"],
] as const;
const OPTIONAL_ROLES = [
  ["finishedGoodsAccountId", "inventory.finishedGoodsAccount"],
  ["rawMaterialsAccountId", "inventory.rawMaterialsAccount"],
  ["inventoryAccountId", "inventory.inventoryAccount"],
] as const;

const ERROR_KEYS: Record<string, string> = {
  invalid_name: "inventory.nameRequired",
  warehouse_not_empty: "inventory.warehouseNotEmpty",
};

export function WarehouseRegister() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [accounts, setAccounts] = useState<Record<string, number | null>>({});
  const [createOpen, setCreateOpen] = useState(false);
  const [nameFilter, setNameFilter] = useState("");
  const debouncedName = useDebounced(nameFilter);
  const { sort, toggleSort } = useSort();

  const params = new URLSearchParams();
  if (debouncedName) params.set("search", debouncedName);
  if (sort) {
    params.set("sort", sort.field);
    params.set("order", sort.dir);
  }

  const { data: warehouses, isLoading } = useQuery({
    queryKey: ["warehouses", debouncedName, sort],
    queryFn: () => apiRequest<Warehouse[]>(`/api/v1/warehouses?${params.toString()}`),
  });

  const {
    register,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({ resolver: zodResolver(schema) });

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) => {
      for (const [key] of REQUIRED_ROLES) {
        if (!accounts[key]) throw new Error("accounts_required");
      }
      return apiRequest<{ id: number }>("/api/v1/warehouses", {
        method: "POST",
        body: JSON.stringify({ ...values, ...accounts }),
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["warehouses"] });
      reset();
      setAccounts({});
      setCreateOpen(false);
    },
    onError: (err: ApiError) => {
      setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") });
    },
  });

  const toggleActiveMutation = useMutation({
    mutationFn: ({ id, activate }: { id: number; activate: boolean }) =>
      apiRequest<void>(`/api/v1/warehouses/${id}/${activate ? "activate" : "deactivate"}`, { method: "POST" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["warehouses"] }),
  });

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-end">
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("inventory.newWarehouse")}</NewButton>
      </div>

      <DataTable<Warehouse>
        columns={[
          {
            key: "name",
            header: t("inventory.warehouseName"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={nameFilter} onChangeAction={setNameFilter} />,
            render: (w) => w.name,
          },
          {
            key: "vatRatePct",
            header: t("inventory.vatRate"),
            sortable: true,
            tdClassName: "tabular-nums text-muted-foreground",
            render: (w) => `${toPersianDigits(w.vatRatePct)}%`,
          },
          {
            key: "isActive",
            header: t("common.active"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            render: (w) => (w.isActive ? t("common.active") : t("common.inactive")),
          },
          {
            key: "actions",
            header: t("common.actions"),
            render: (w) => (
              <button
                type="button"
                onClick={() => toggleActiveMutation.mutate({ id: w.id, activate: !w.isActive })}
                className="cursor-pointer text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
              >
                {w.isActive ? t("inventory.deactivate") : t("inventory.activate")}
              </button>
            ),
          },
        ]}
        rows={warehouses}
        isLoading={isLoading}
        rowKeyAction={(w) => w.id}
        sort={sort}
        onSortAction={toggleSort}
      />

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("inventory.newWarehouse")}>
        <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <Field label={t("inventory.warehouseName")} wide>
              <input type="text" placeholder="انبار مرکزی" {...register("name")} className={fieldInputClass} autoFocus />
            </Field>
            <Field label={t("inventory.vatRate")}>
              <input type="number" step="0.01" placeholder="9" {...register("vatRatePct")} className={fieldInputClass} />
            </Field>
          </div>
          <div className="flex flex-col gap-2">
            <span className="text-sm text-muted-foreground">{t("accounts.title")}</span>
            <div className="flex flex-wrap gap-3">
              {REQUIRED_ROLES.map(([key, labelKey]) => (
                <AccountField
                  key={key}
                  label={t(labelKey)}
                  value={accounts[key] ?? null}
                  onChangeAction={(id) => setAccounts((a) => ({ ...a, [key]: id }))}
                />
              ))}
            </div>
            <div className="flex flex-wrap gap-3 border-t border-border pt-3">
              {OPTIONAL_ROLES.map(([key, labelKey]) => (
                <AccountField
                  key={key}
                  label={t(labelKey)}
                  value={accounts[key] ?? null}
                  onChangeAction={(id) => setAccounts((a) => ({ ...a, [key]: id }))}
                />
              ))}
            </div>
          </div>
          {(errors.name || errors.root) && (
            <p role="alert" className="text-sm text-danger">
              {errors.root?.message ?? t("common.error")}
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
