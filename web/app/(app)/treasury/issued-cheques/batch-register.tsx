"use client";

// Step 4.5 follow-up: the issued-cheque payment batch register (legacy
// CheckListU/CheckEditU equivalent, A9 unblocked as "batch" per explicit
// user decision -- see api/src/issued_cheques.rs's module doc comment).
// Structurally the mirror of petty-cash's claim-register.tsx.

import { useForm, useFieldArray, useController, type Control } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useState } from "react";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { AccountField } from "@/components/account-field";
import { DateField } from "@/components/date-field";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";
import { DataTable, FilterInput, useDebounced, useSort } from "@/components/data-table";
import type { ChequeBatchSummary } from "@/lib/treasury";

const schema = z.object({
  batchNumber: z.string().optional(),
  issueDate: z.string().min(1),
  description: z.string().min(1),
  lines: z
    .array(
      z.object({
        payeeAccountId: z.number().positive(),
        amount: z.coerce.number().positive(),
        description: z.string().optional(),
      }),
    )
    .min(1),
});
type FormValues = z.input<typeof schema>;
type FormOutput = z.output<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  description_required: "vouchers.voucherEmpty",
  at_least_one_line_required: "treasury.atLeastOneLineRequired",
  line_amount_must_be_positive: "treasury.amountMustBePositive",
  date_outside_fiscal_year: "treasury.dateOutsideFiscalYear",
  account_not_leaf: "treasury.accountNotLeaf",
  voucher_not_draft: "treasury.voucherNotDraft",
};

function PayeeAccountField({ control, index }: { control: Control<FormValues>; index: number }) {
  const { field } = useController({ control, name: `lines.${index}.payeeAccountId` });
  return <AccountField value={field.value || null} onChangeAction={(id) => field.onChange(id)} />;
}

export function BatchRegister({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [bankAccountId, setBankAccountId] = useState<number | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [batchNumberFilter, setBatchNumberFilter] = useState("");
  const [descriptionFilter, setDescriptionFilter] = useState("");
  const debouncedBatchNumber = useDebounced(batchNumberFilter);
  const debouncedDescription = useDebounced(descriptionFilter);
  const { sort, toggleSort } = useSort();

  const params = new URLSearchParams();
  if (fiscalYearId) params.set("fiscalYearId", String(fiscalYearId));
  if (debouncedBatchNumber) params.set("batchNumber", debouncedBatchNumber);
  if (debouncedDescription) params.set("description", debouncedDescription);
  if (sort) {
    params.set("sort", sort.field);
    params.set("order", sort.dir);
  }

  const { data: batches, isLoading } = useQuery({
    queryKey: ["cheque-payment-batches", fiscalYearId, debouncedBatchNumber, debouncedDescription, sort],
    queryFn: () => apiRequest<ChequeBatchSummary[]>(`/api/v1/cheque-payment-batches?${params.toString()}`),
    enabled: fiscalYearId !== null,
  });

  const {
    register,
    control,
    handleSubmit,
    setError,
    reset,
    formState: { errors, isSubmitting },
  } = useForm<FormValues, unknown, FormOutput>({
    resolver: zodResolver(schema),
    defaultValues: { lines: [{ payeeAccountId: 0, amount: undefined, description: "" }] },
  });
  const { fields, append, remove } = useFieldArray({ control, name: "lines" });
  const { field: issueDateField } = useController({ control, name: "issueDate" });

  const createMutation = useMutation({
    mutationFn: (values: FormOutput) => {
      if (!bankAccountId) throw new Error("accounts_required");
      return apiRequest<{ id: number }>("/api/v1/cheque-payment-batches", {
        method: "POST",
        body: JSON.stringify({ ...values, fiscalYearId, bankAccountId }),
      });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["cheque-payment-batches"] });
      reset({ lines: [{ payeeAccountId: 0, amount: undefined, description: "" }] });
      setCreateOpen(false);
    },
    onError: (err: ApiError) => setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") }),
  });

  const deleteMutation = useMutation({
    mutationFn: (id: number) => apiRequest(`/api/v1/cheque-payment-batches/${id}`, { method: "DELETE" }),
    onSuccess: () => queryClient.invalidateQueries({ queryKey: ["cheque-payment-batches"] }),
  });

  if (fiscalYearId === null) {
    return <p className="text-sm text-muted-foreground">{t("shell.noFiscalYear")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex justify-end">
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("treasury.newBatch")}</NewButton>
      </div>

      <DataTable<ChequeBatchSummary>
        columns={[
          {
            key: "batchNumber",
            header: t("treasury.batchNumber"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={batchNumberFilter} onChangeAction={setBatchNumberFilter} />,
            render: (b) => (b.batchNumber ? toPersianDigits(b.batchNumber) : "—"),
          },
          {
            key: "issueDate",
            header: t("treasury.issueDate"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            render: (b) => b.issueDate,
          },
          {
            key: "totalAmount",
            header: t("treasury.listTotal"),
            sortable: true,
            tdClassName: "tabular-nums text-foreground",
            render: (b) => toPersianDigits(b.totalAmount),
          },
          {
            key: "lineCount",
            header: t("treasury.lineCount"),
            sortable: true,
            tdClassName: "tabular-nums text-muted-foreground",
            render: (b) => toPersianDigits(b.lineCount),
          },
          {
            key: "description",
            header: t("treasury.description"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={descriptionFilter} onChangeAction={setDescriptionFilter} />,
            render: (b) => b.description,
          },
          {
            key: "actions",
            header: t("common.actions"),
            render: (b) => (
              <button
                type="button"
                onClick={() => deleteMutation.mutate(b.id)}
                className="cursor-pointer text-sm text-danger hover:underline focus-visible:ring-2 focus-visible:ring-danger"
              >
                {t("treasury.deleteBatch")}
              </button>
            ),
          },
        ]}
        rows={batches}
        isLoading={isLoading}
        rowKeyAction={(b) => b.id}
        sort={sort}
        onSortAction={toggleSort}
      />

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("treasury.newBatch")} widthClassName="w-[min(92vw,40rem)]">
        <form onSubmit={handleSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <div className="grid grid-cols-2 gap-3 sm:grid-cols-3">
            <Field label={t("treasury.batchNumber")}>
              <input type="text" placeholder="1" {...register("batchNumber")} className={fieldInputClass} autoFocus />
            </Field>
            <DateField label={t("treasury.issueDate")} value={issueDateField.value} onChangeAction={issueDateField.onChange} />
            <Field label={t("treasury.description")} wide>
              <input type="text" placeholder="پرداخت دسته چک به تامین‌کنندگان" {...register("description")} className={fieldInputClass} />
            </Field>
            <AccountField label={t("treasury.bankAccountCode")} value={bankAccountId} onChangeAction={setBankAccountId} />
          </div>

          <div className="flex flex-col gap-2">
            <h3 className="text-sm font-medium text-foreground">{t("treasury.payeeLines")}</h3>
            {fields.map((field, index) => (
              <div key={field.id} className="flex flex-wrap items-end gap-2 rounded-md border border-border p-2">
                <PayeeAccountField control={control} index={index} />
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("treasury.amount")}</label>
                  <input
                    type="number"
                    placeholder="5000000"
                    {...register(`lines.${index}.amount`)}
                    className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                <div className="flex flex-col gap-1">
                  <label className="text-sm text-muted-foreground">{t("treasury.description")}</label>
                  <input
                    type="text"
                    placeholder="بابت خرید کالا"
                    {...register(`lines.${index}.description`)}
                    className="h-9 w-48 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
                  />
                </div>
                {fields.length > 1 && (
                  <button
                    type="button"
                    onClick={() => remove(index)}
                    className="h-9 cursor-pointer rounded-md border border-danger px-3 text-sm text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-danger"
                  >
                    {t("treasury.removeLine")}
                  </button>
                )}
              </div>
            ))}
            <button
              type="button"
              onClick={() => append({ payeeAccountId: 0, amount: undefined, description: "" })}
              className="h-9 w-fit cursor-pointer rounded-md border border-border px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("treasury.addLine")}
            </button>
          </div>

          {(errors.issueDate || errors.description || errors.lines || errors.root) && (
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
              {t("treasury.save")}
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
