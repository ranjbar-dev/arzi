"use client";

// Step 2.2 (docs/phase-2-accounting-core.md §2.2): a drill-down tree editor
// covering everything `SNewu` did (specs/03-accounting-core/03-12-a.md
// §12.2) — one level at a time with a breadcrumb, not a fully expanded
// nested tree widget (matches the legacy's actual UX, which this replaces
// rather than reinvents).

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { ownCode, type AccountDetail, type AccountSummary } from "@/lib/accounts";
import { AccountPicker } from "@/components/account-picker";
import { LockIcon } from "@/components/lock-icon";

const codeNameSchema = z.object({
  code: z.number().int().min(1),
  name: z.string().min(1),
});
type CodeNameValues = z.infer<typeof codeNameSchema>;

const ERROR_KEYS: Record<string, string> = {
  duplicate_code: "accounts.duplicateCode",
  has_children: "accounts.hasChildren",
  invalid_code: "accounts.invalidCode",
  invalid_name: "accounts.invalidName",
  already_top_level: "accounts.alreadyTopLevel",
  already_max_level: "accounts.alreadyMaxLevel",
  max_depth_reached: "accounts.maxDepthReached",
  invalid_target_level: "accounts.invalidTargetLevel",
};

export function ChartOfAccountsEditor({ canLock }: { canLock: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [stack, setStack] = useState<AccountSummary[]>([]);
  const [selectedId, setSelectedId] = useState<number | null>(null);
  const [dialog, setDialog] = useState<"create" | "rename" | "recode" | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const parentId = stack.length ? stack[stack.length - 1].id : null;

  const { data: children, isLoading } = useQuery({
    queryKey: ["accounts", "children", parentId],
    queryFn: () =>
      apiRequest<AccountSummary[]>(`/api/v1/accounts${parentId ? `?parent=${parentId}` : ""}`),
  });
  const { data: selected } = useQuery({
    queryKey: ["accounts", selectedId],
    queryFn: () => apiRequest<AccountDetail>(`/api/v1/accounts/${selectedId}`),
    enabled: selectedId !== null,
  });

  const invalidate = () => {
    queryClient.invalidateQueries({ queryKey: ["accounts"] });
  };

  function reportError(err: unknown) {
    const code = err instanceof ApiError ? err.message : "internal_error";
    setActionError(t(ERROR_KEYS[code] ?? "common.error"));
  }

  const createForm = useForm<CodeNameValues>({ resolver: zodResolver(codeNameSchema) });
  const createMutation = useMutation({
    mutationFn: (values: CodeNameValues) =>
      apiRequest("/api/v1/accounts", {
        method: "POST",
        body: JSON.stringify({ parentId, code: values.code, name: values.name }),
      }),
    onSuccess: () => {
      createForm.reset();
      setDialog(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const renameForm = useForm<{ name: string }>();
  const renameMutation = useMutation({
    mutationFn: (name: string) =>
      apiRequest(`/api/v1/accounts/${selectedId}/name`, {
        method: "PUT",
        body: JSON.stringify({ name }),
      }),
    onSuccess: () => {
      setDialog(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const recodeForm = useForm<{ code: number }>();
  const recodeMutation = useMutation({
    mutationFn: (code: number) =>
      apiRequest(`/api/v1/accounts/${selectedId}/code`, {
        method: "PUT",
        body: JSON.stringify({ code }),
      }),
    onSuccess: () => {
      setDialog(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const deleteMutation = useMutation({
    mutationFn: () => apiRequest(`/api/v1/accounts/${selectedId}`, { method: "DELETE" }),
    onSuccess: () => {
      setSelectedId(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const promoteMutation = useMutation({
    mutationFn: () => apiRequest(`/api/v1/accounts/${selectedId}/promote`, { method: "POST" }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const demoteMutation = useMutation({
    mutationFn: (targetParentId: number) =>
      apiRequest(`/api/v1/accounts/${selectedId}/demote`, {
        method: "POST",
        body: JSON.stringify({ parentId: targetParentId }),
      }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const lockMutation = useMutation({
    mutationFn: (lock: boolean) =>
      apiRequest(`/api/v1/accounts/${selectedId}/${lock ? "lock" : "unlock"}`, { method: "POST" }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
      queryClient.invalidateQueries({ queryKey: ["accounts", selectedId] });
    },
    onError: reportError,
  });

  function drillInto(account: AccountSummary) {
    if (account.level < 4) {
      setStack([...stack, account]);
      setSelectedId(null);
    }
  }

  function goUp() {
    setStack(stack.slice(0, -1));
    setSelectedId(null);
  }

  const toolbarDisabled = selectedId === null;

  return (
    <div className="flex flex-col gap-4">
      {/* Breadcrumb — ST1/ST2/St3 (03-12-a.md §12.2). */}
      <div className="flex flex-wrap items-center gap-1 text-sm text-muted-foreground">
        <button
          type="button"
          onClick={() => {
            setStack([]);
            setSelectedId(null);
          }}
          className="cursor-pointer hover:text-accent focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("accounts.root")}
        </button>
        {stack.map((node, i) => (
          <span key={node.id} className="flex items-center gap-1">
            <span>/</span>
            <button
              type="button"
              onClick={() => {
                setStack(stack.slice(0, i + 1));
                setSelectedId(null);
              }}
              className="cursor-pointer hover:text-accent focus-visible:ring-2 focus-visible:ring-accent"
            >
              {toPersianDigits(ownCode(node))} {node.name}
            </button>
          </span>
        ))}
        {stack.length > 0 && (
          <button
            type="button"
            onClick={goUp}
            className="ms-2 cursor-pointer text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
          >
            ↑ {t("accounts.backToParent")}
          </button>
        )}
      </div>

      {/* Toolbar */}
      <div className="flex flex-wrap gap-2">
        <button
          type="button"
          onClick={() => setDialog("create")}
          className="h-9 cursor-pointer rounded-md bg-primary px-3 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("accounts.newCode")}
        </button>
        <button
          type="button"
          disabled={toolbarDisabled}
          onClick={() => setDialog("rename")}
          className="h-9 cursor-pointer rounded-md border border-border bg-surface px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40"
        >
          {t("accounts.editName")}
        </button>
        <button
          type="button"
          disabled={toolbarDisabled}
          onClick={() => setDialog("recode")}
          className="h-9 cursor-pointer rounded-md border border-border bg-surface px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40"
        >
          {t("accounts.editCode")}
        </button>
        <button
          type="button"
          disabled={toolbarDisabled}
          onClick={() => promoteMutation.mutate()}
          className="h-9 cursor-pointer rounded-md border border-border bg-surface px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40"
        >
          {t("accounts.promote")}
        </button>
        {selected && (
          <AccountPicker
            triggerLabel={t("accounts.demote")}
            isSelectable={(a) => a.level === selected.level && a.id !== selected.id}
            onSelect={(target) => demoteMutation.mutate(target.id)}
          />
        )}
        {canLock && (
          <button
            type="button"
            disabled={toolbarDisabled}
            onClick={() => lockMutation.mutate(!selected?.isLocked)}
            className="flex h-9 cursor-pointer items-center gap-1.5 rounded-md border border-border bg-surface px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40"
          >
            <LockIcon locked={!!selected?.isLocked} />
            {t("accounts.lock")}
          </button>
        )}
        <button
          type="button"
          disabled={toolbarDisabled}
          onClick={() => {
            if (confirm(t("accounts.deleteCode") + "?")) deleteMutation.mutate();
          }}
          className="h-9 cursor-pointer rounded-md border border-danger px-3 text-sm text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40"
        >
          {t("accounts.deleteCode")}
        </button>
      </div>

      {actionError && (
        <p role="alert" className="text-sm text-danger">
          {actionError}
        </p>
      )}

      {selected && (
        <div className="rounded-md border border-border bg-surface p-3 text-sm">
          <div className="flex flex-wrap gap-x-6 gap-y-1 text-muted-foreground">
            <span>
              LTR: <span className="tabular-nums text-foreground">{selected.codeLtr}</span>
            </span>
            <span>
              RTL: <span className="tabular-nums text-foreground">{selected.codeRtl}</span>
            </span>
            <span>{selected.fullNamePath}</span>
            <span className={selected.childCount === 0 ? "text-success" : "text-muted-foreground"}>
              {selected.childCount === 0 ? t("accounts.postable") : t("accounts.notPostable")}
            </span>
            {selected.isLocked && <span className="text-warning">{t("accounts.lockedAccount")}</span>}
          </div>
        </div>
      )}

      {/* Grid — Code / S_Name / SNO / S_Lock (03-12-a.md §12.2). */}
      {isLoading ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : (
        <div className="overflow-x-auto rounded-md border border-border">
          <table className="w-full text-sm">
            <thead>
              <tr className="border-b border-border bg-muted/50 text-muted-foreground">
                <th className="w-24 px-3 py-2 text-center font-medium">{t("accounts.code")}</th>
                <th className="px-3 py-2 text-start font-medium">{t("accounts.fullName")}</th>
                <th className="w-20 px-3 py-2 text-center font-medium">{t("accounts.childCount")}</th>
                <th className="w-16 px-3 py-2 text-center font-medium">{t("accounts.lock")}</th>
              </tr>
            </thead>
            <tbody>
              {children?.map((account) => (
                <tr
                  key={account.id}
                  onClick={() => setSelectedId(account.id)}
                  onDoubleClick={() => drillInto(account)}
                  className={`cursor-pointer border-b border-border last:border-0 hover:bg-muted ${
                    selectedId === account.id ? "bg-accent/10" : ""
                  }`}
                >
                  <td className="tabular-nums px-3 py-2 text-center text-foreground">
                    {toPersianDigits(ownCode(account))}
                  </td>
                  <td className="px-3 py-2 text-foreground">
                    {account.name}
                    {account.childCount > 0 && (
                      <span className="ms-1 text-xs text-muted-foreground">›</span>
                    )}
                  </td>
                  <td className="tabular-nums px-3 py-2 text-center text-muted-foreground">
                    {toPersianDigits(account.childCount)}
                  </td>
                  <td className="px-3 py-2 text-center">
                    {account.isLocked && <LockIcon locked className="mx-auto h-4 w-4 text-warning" />}
                  </td>
                </tr>
              ))}
              {children?.length === 0 && (
                <tr>
                  <td colSpan={4} className="px-3 py-6 text-center text-sm text-muted-foreground">
                    —
                  </td>
                </tr>
              )}
            </tbody>
          </table>
        </div>
      )}

      {dialog === "create" && (
        <form
          onSubmit={createForm.handleSubmit((values) => createMutation.mutate(values))}
          className="flex flex-wrap items-end gap-3 rounded-md border border-border bg-surface p-3"
        >
          <h3 className="w-full text-sm font-medium text-foreground">{t("accounts.newAccountTitle")}</h3>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("accounts.codeLabel")}</label>
            <input
              type="number"
              {...createForm.register("code", { valueAsNumber: true })}
              className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
              autoFocus
            />
          </div>
          <div className="flex flex-col gap-1">
            <label className="text-sm text-muted-foreground">{t("accounts.nameLabel")}</label>
            <input
              type="text"
              {...createForm.register("name")}
              className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            />
          </div>
          <button
            type="submit"
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.create")}
          </button>
          <button
            type="button"
            onClick={() => setDialog(null)}
            className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.cancel")}
          </button>
        </form>
      )}

      {dialog === "rename" && selected && (
        <form
          onSubmit={renameForm.handleSubmit((v) => renameMutation.mutate(v.name))}
          className="flex flex-wrap items-end gap-3 rounded-md border border-border bg-surface p-3"
        >
          <input
            type="text"
            defaultValue={selected.name}
            {...renameForm.register("name")}
            className="h-9 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            autoFocus
          />
          <button
            type="submit"
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.save")}
          </button>
          <button
            type="button"
            onClick={() => setDialog(null)}
            className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.cancel")}
          </button>
        </form>
      )}

      {dialog === "recode" && selected && (
        <form
          onSubmit={recodeForm.handleSubmit((v) => recodeMutation.mutate(Number(v.code)))}
          className="flex flex-wrap items-end gap-3 rounded-md border border-border bg-surface p-3"
        >
          <input
            type="number"
            defaultValue={ownCode(selected)}
            {...recodeForm.register("code", { valueAsNumber: true })}
            className="h-9 w-28 rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent"
            autoFocus
          />
          <button
            type="submit"
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.save")}
          </button>
          <button
            type="button"
            onClick={() => setDialog(null)}
            className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
          >
            {t("common.cancel")}
          </button>
        </form>
      )}
    </div>
  );
}
