"use client";

import { useForm, useController } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import type { VoucherSummary } from "@/lib/vouchers";
import { DateField } from "@/components/date-field";
import { JournalGenerationForm } from "./journal-generation-form";

const schema = z.object({
  voucherDate: z.string().min(1),
  description: z.string().min(1),
});
type FormValues = z.infer<typeof schema>;

const ERROR_KEYS: Record<string, string> = {
  date_outside_fiscal_year: "vouchers.dateOutsideFiscalYear",
  duplicate_voucher_number: "vouchers.duplicateVoucherNumber",
  description_required: "vouchers.voucherEmpty",
  fiscal_year_closed: "fiscalYears.alreadyClosed",
};

const STATUS_LABEL: Record<string, string> = {
  draft: "vouchers.draft",
  confirmed: "vouchers.confirmed",
  posted: "vouchers.posted",
};

export function VoucherList({ fiscalYearId }: { fiscalYearId: number | null }) {
  const { t } = useTranslation();
  const router = useRouter();
  const queryClient = useQueryClient();

  const { data: vouchers, isLoading } = useQuery({
    queryKey: ["vouchers", fiscalYearId],
    queryFn: () =>
      apiRequest<VoucherSummary[]>(
        `/api/v1/vouchers${fiscalYearId ? `?fiscalYearId=${fiscalYearId}` : ""}`,
      ),
    enabled: fiscalYearId !== null,
  });

  const {
    register,
    control,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<FormValues>({ resolver: zodResolver(schema) });
  const { field: voucherDateField } = useController({ control, name: "voucherDate" });

  const createMutation = useMutation({
    mutationFn: (values: FormValues) =>
      apiRequest<{ id: number }>("/api/v1/vouchers", {
        method: "POST",
        body: JSON.stringify({ ...values, fiscalYearId }),
      }),
    onSuccess: (result) => {
      queryClient.invalidateQueries({ queryKey: ["vouchers"] });
      router.push(`/accounting/vouchers/${result.id}`);
    },
    onError: (err: ApiError) => {
      setError("root", { message: t(ERROR_KEYS[err.message] ?? "common.error") });
    },
  });

  if (fiscalYearId === null) {
    return <p className="text-sm text-muted-foreground">{t("shell.noFiscalYear")}</p>;
  }

  return (
    <div className="flex flex-col gap-4">
      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="px-3 py-2 text-start font-medium">{t("vouchers.voucherNumber")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("vouchers.voucherDate")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("vouchers.description")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("vouchers.totalDebit")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("vouchers.totalCredit")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("vouchers.status")}</th>
              </tr>
            </thead>
            <tbody>
              {vouchers?.map((v) => (
                <tr
                  key={v.id}
                  onClick={() => router.push(`/accounting/vouchers/${v.id}`)}
                  className="cursor-pointer border-b border-border last:border-0 hover:bg-muted"
                >
                  <td className="px-3 py-2">
                    <Link
                      href={`/accounting/vouchers/${v.id}`}
                      className="tabular-nums text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                    >
                      {toPersianDigits(v.voucherNumber)}
                    </Link>
                  </td>
                  <td className="px-3 py-2 text-muted-foreground">{v.voucherDate}</td>
                  <td className="px-3 py-2 text-foreground">{v.description}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(v.totalDebit)}</td>
                  <td className="tabular-nums px-3 py-2 text-foreground">{toPersianDigits(v.totalCredit)}</td>
                  <td className="px-3 py-2 text-muted-foreground">{t(STATUS_LABEL[v.status])}</td>
                </tr>
              ))}
              {vouchers?.length === 0 && (
                <tr>
                  <td colSpan={6} className="px-3 py-6 text-center text-sm text-muted-foreground">
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
        className="flex flex-wrap items-end gap-3"
      >
        <DateField
          label={t("vouchers.voucherDate")}
          value={voucherDateField.value}
          onChangeAction={voucherDateField.onChange}
        />
        <div className="flex flex-col gap-1">
          <label className="text-sm text-muted-foreground">{t("vouchers.description")}</label>
          <input
            type="text"
            placeholder="خرید پسته از انبار مرکزی"
            {...register("description")}
            className="h-9 w-64 rounded-md border border-border bg-surface px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
          />
        </div>
        <button
          type="submit"
          disabled={isSubmitting}
          className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
        >
          {t("vouchers.newVoucher")}
        </button>
        {(errors.voucherDate || errors.description || errors.root) && (
          <p role="alert" className="w-full text-sm text-danger">
            {errors.root?.message ?? t("common.error")}
          </p>
        )}
      </form>

      <JournalGenerationForm fiscalYearId={fiscalYearId} />
    </div>
  );
}
