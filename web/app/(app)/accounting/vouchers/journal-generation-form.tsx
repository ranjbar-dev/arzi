"use client";

// Step 2.6 (docs/phase-2-accounting-core.md §2.6): trigger for the journal
// (Rooznameh) roll-up. Range selection mirrors `MoeinToRU.pas` (03-08.md
// §8.1) — by voucher number OR by date, never both.

import { useState } from "react";
import { useForm, useWatch, useController } from "react-hook-form";
import { useMutation, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import { apiRequest, ApiError } from "@/lib/api-client";
import { DateField } from "@/components/date-field";

const ERROR_KEYS: Record<string, string> = {
  description_too_short: "vouchers.descriptionTooShort",
  invalid_range: "vouchers.invalidRange",
  no_vouchers_in_range: "vouchers.noVouchersInRange",
  vouchers_not_all_posted: "vouchers.vouchersNotAllPosted",
  no_unjournalised_vouchers_in_range: "vouchers.noUnjournalisedVouchersInRange",
  date_outside_fiscal_year: "vouchers.dateOutsideFiscalYear",
  duplicate_voucher_number: "vouchers.duplicateVoucherNumber",
  fiscal_year_closed: "fiscalYears.alreadyClosed",
};

interface FormValues {
  rangeBy: "number" | "date";
  fromVoucherNumber?: number;
  toVoucherNumber?: number;
  fromDate?: string;
  toDate?: string;
  voucherDate: string;
  description: string;
}

export function JournalGenerationForm({ fiscalYearId }: { fiscalYearId: number }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();
  const [open, setOpen] = useState(false);
  const { register, handleSubmit, control, setError, formState: { errors, isSubmitting } } =
    useForm<FormValues>({ defaultValues: { rangeBy: "date" } });
  const rangeBy = useWatch({ control, name: "rangeBy" });
  const { field: fromDateField } = useController({ control, name: "fromDate" });
  const { field: toDateField } = useController({ control, name: "toDate" });
  const { field: voucherDateField } = useController({ control, name: "voucherDate" });

  const mutation = useMutation({
    mutationFn: (values: FormValues) => {
      const body: Record<string, unknown> = {
        fiscalYearId,
        voucherDate: values.voucherDate,
        description: values.description,
      };
      if (values.rangeBy === "number") {
        body.fromVoucherNumber = Number(values.fromVoucherNumber);
        body.toVoucherNumber = Number(values.toVoucherNumber);
      } else {
        body.fromDate = values.fromDate;
        body.toDate = values.toDate;
      }
      return apiRequest<{ id: number }>("/api/v1/vouchers/generate-journal", {
        method: "POST",
        body: JSON.stringify(body),
      });
    },
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["vouchers"] });
      router.push(`/accounting/vouchers/${result.id}`);
    },
    onError: (err: ApiError) => {
      setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") });
    },
  });

  if (!open) {
    return (
      <button
        type="button"
        onClick={() => setOpen(true)}
        className="h-9 cursor-pointer rounded-md border border-border bg-surface px-4 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
      >
        {t("vouchers.generateJournal")}
      </button>
    );
  }

  return (
    <form
      onSubmit={handleSubmit((values) => mutation.mutate(values))}
      className="flex flex-col gap-3 rounded-md border border-border bg-surface p-3"
    >
      <div className="flex gap-4 text-sm text-foreground">
        <label className="flex items-center gap-1.5">
          <input type="radio" value="date" {...register("rangeBy")} defaultChecked />
          {t("vouchers.journalRangeByDate")}
        </label>
        <label className="flex items-center gap-1.5">
          <input type="radio" value="number" {...register("rangeBy")} />
          {t("vouchers.journalRangeByNumber")}
        </label>
      </div>

      <div className="flex flex-wrap items-end gap-3">
        {rangeBy === "date" ? (
          <>
            <DateField
              label={t("vouchers.fromDate")}
              value={fromDateField.value ?? ""}
              onChangeAction={fromDateField.onChange}
            />
            <DateField
              label={t("vouchers.toDate")}
              value={toDateField.value ?? ""}
              onChangeAction={toDateField.onChange}
            />
          </>
        ) : (
          <>
            <div className="flex flex-col gap-1">
              <label className="text-sm text-muted-foreground">{t("vouchers.fromVoucherNumber")}</label>
              <input type="number" placeholder="1" {...register("fromVoucherNumber", { valueAsNumber: true })} className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
            </div>
            <div className="flex flex-col gap-1">
              <label className="text-sm text-muted-foreground">{t("vouchers.toVoucherNumber")}</label>
              <input type="number" placeholder="50" {...register("toVoucherNumber", { valueAsNumber: true })} className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
            </div>
          </>
        )}
        <DateField
          label={t("vouchers.voucherDate")}
          value={voucherDateField.value}
          onChangeAction={voucherDateField.onChange}
        />
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("vouchers.description")}</label>
          <input type="text" placeholder="روزنامه فروردین ۱۴۰۳" {...register("description")} className="h-9 w-64 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent" />
        </div>
        <button
          type="submit"
          disabled={isSubmitting}
          className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
        >
          {t("vouchers.generateJournal")}
        </button>
        <button
          type="button"
          onClick={() => setOpen(false)}
          className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("common.cancel")}
        </button>
      </div>
      {errors.root && (
        <p role="alert" className="text-sm text-danger">
          {errors.root.message}
        </p>
      )}
    </form>
  );
}
