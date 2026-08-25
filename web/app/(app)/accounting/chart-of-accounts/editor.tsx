"use client";

// Step 2.2 (docs/phase-2-accounting-core.md §2.2): the chart-of-accounts
// editor, redesigned on top of react-complex-tree so the whole 4-level
// hierarchy (Kol/Moein/Tafsil1/Tafsil2) is visible and reparentable in one
// page instead of the original drill-down-one-level-at-a-time screen.
//
// There is no `parent_id` column (see accounts.rs's module doc comment) —
// the tree structure below is derived client-side from the same code-
// segment-prefix relationship the backend uses, and reparenting is done
// through the existing promote/demote endpoints (specs/03-accounting-core's
// "no legacy precedent" pair, accounts.rs's own doc comment on them): drag
// a leaf onto a same-level account to demote/nest it inside; drag it onto
// empty tree space (the root) to promote it out one level. Both endpoints
// are leaf-only (`child_count === 0`), so only leaf accounts are draggable.

import { useMemo, useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import {
  StaticTreeDataProvider,
  Tree,
  UncontrolledTreeEnvironment,
  type DraggingPosition,
  type TreeItem,
  type TreeItemIndex,
} from "react-complex-tree";
import "react-complex-tree/lib/style-modern.css";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { ownCode, type AccountDetail, type AccountSummary } from "@/lib/accounts";
import { AccountPicker } from "@/components/account-picker";
import { LockIcon } from "@/components/lock-icon";
import { EditIcon } from "@/components/edit-icon";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";

const LEVEL_KEYS = ["accounts.levelKol", "accounts.levelMoein", "accounts.levelTafsil1", "accounts.levelTafsil2"] as const;

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

type Codes = [number, number, number, number];

function codesOf(a: AccountSummary): Codes {
  return [a.generalLedgerCode, a.subsidiaryCode, a.analytic1Code, a.analytic2Code];
}

// Same segment-prefix relationship as the backend's `fetch_parent` — an
// account's parent is the account whose codes match its own down to (but
// not including) its own level.
function ancestorKey(codes: Codes, depth: number): string {
  return codes.slice(0, depth).join("-");
}

function buildTreeItems(accounts: AccountSummary[]): Record<string, TreeItem<AccountSummary | null>> {
  const byKey = new Map<string, AccountSummary>();
  for (const a of accounts) byKey.set(ancestorKey(codesOf(a), a.level), a);

  const childIds = new Map<string, string[]>();
  for (const a of [...accounts].sort((x, y) => ownCode(x) - ownCode(y))) {
    const parent = a.level === 1 ? null : byKey.get(ancestorKey(codesOf(a), a.level - 1));
    const parentIndex = parent ? String(parent.id) : "root";
    (childIds.get(parentIndex) ?? childIds.set(parentIndex, []).get(parentIndex)!).push(String(a.id));
  }

  const items: Record<string, TreeItem<AccountSummary | null>> = {
    root: { index: "root", isFolder: true, children: childIds.get("root") ?? [], data: null },
  };
  for (const a of accounts) {
    items[String(a.id)] = {
      index: String(a.id),
      isFolder: a.level < 4,
      children: childIds.get(String(a.id)) ?? [],
      data: a,
    };
  }
  return items;
}

export function ChartOfAccountsEditor({ canLock }: { canLock: boolean }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const [editingId, setEditingId] = useState<number | null>(null);
  const [creating, setCreating] = useState(false);
  const [createParent, setCreateParent] = useState<AccountSummary | null>(null);
  const [actionError, setActionError] = useState<string | null>(null);

  const { data: accounts, isLoading } = useQuery({
    queryKey: ["accounts", "all"],
    queryFn: () => apiRequest<AccountSummary[]>("/api/v1/accounts?all=true"),
  });
  const { data: editing } = useQuery({
    queryKey: ["accounts", editingId],
    queryFn: () => apiRequest<AccountDetail>(`/api/v1/accounts/${editingId}`),
    enabled: editingId !== null,
  });

  const treeItems = useMemo(() => buildTreeItems(accounts ?? []), [accounts]);
  const dataProvider = useMemo(() => new StaticTreeDataProvider(treeItems), [treeItems]);

  // Default-expanded set for the tree's first mount — react-complex-tree's
  // `viewState` prop only seeds its internal state once, at mount; changing
  // it on later renders has no effect, which is what lets the user's own
  // expand/collapse choices survive later refetches despite this being
  // recomputed every render.
  const initialExpanded: TreeItemIndex[] = useMemo(
    () => (accounts ?? []).filter((a) => a.childCount > 0).map((a) => String(a.id)),
    [accounts],
  );

  const invalidate = () => queryClient.invalidateQueries({ queryKey: ["accounts"] });

  function reportError(err: unknown) {
    const code = err instanceof ApiError ? err.message : "internal_error";
    setActionError(t(ERROR_KEYS[code] ?? "common.error"));
  }

  const createForm = useForm<CodeNameValues>({ resolver: zodResolver(codeNameSchema) });
  const createMutation = useMutation({
    mutationFn: (values: CodeNameValues) =>
      apiRequest("/api/v1/accounts", {
        method: "POST",
        body: JSON.stringify({ parentId: createParent?.id ?? null, code: values.code, name: values.name }),
      }),
    onSuccess: () => {
      createForm.reset();
      setCreating(false);
      setCreateParent(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const editForm = useForm<CodeNameValues>({ resolver: zodResolver(codeNameSchema) });
  const renameMutation = useMutation({
    mutationFn: (name: string) =>
      apiRequest(`/api/v1/accounts/${editingId}/name`, { method: "PUT", body: JSON.stringify({ name }) }),
  });
  const recodeMutation = useMutation({
    mutationFn: (code: number) =>
      apiRequest(`/api/v1/accounts/${editingId}/code`, { method: "PUT", body: JSON.stringify({ code }) }),
  });

  async function onEditSubmit(values: CodeNameValues) {
    if (!editing) return;
    try {
      if (values.name !== editing.name) await renameMutation.mutateAsync(values.name);
      if (values.code !== ownCode(editing)) await recodeMutation.mutateAsync(values.code);
      setEditingId(null);
      setActionError(null);
      invalidate();
    } catch (err) {
      reportError(err);
    }
  }

  const deleteMutation = useMutation({
    mutationFn: () => apiRequest(`/api/v1/accounts/${editingId}`, { method: "DELETE" }),
    onSuccess: () => {
      setEditingId(null);
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  const lockMutation = useMutation({
    mutationFn: (lock: boolean) =>
      apiRequest(`/api/v1/accounts/${editingId}/${lock ? "lock" : "unlock"}`, { method: "POST" }),
    onSuccess: () => {
      setActionError(null);
      queryClient.invalidateQueries({ queryKey: ["accounts", editingId] });
    },
    onError: reportError,
  });

  const promoteMutation = useMutation({
    mutationFn: (id: number) => apiRequest(`/api/v1/accounts/${id}/promote`, { method: "POST" }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });
  const demoteMutation = useMutation({
    mutationFn: ({ id, parentId }: { id: number; parentId: number }) =>
      apiRequest(`/api/v1/accounts/${id}/demote`, { method: "POST", body: JSON.stringify({ parentId }) }),
    onSuccess: () => {
      setActionError(null);
      invalidate();
    },
    onError: reportError,
  });

  function canDropAt(items: TreeItem<AccountSummary | null>[], target: DraggingPosition): boolean {
    if (items.length !== 1 || !items[0].data) return false;
    const dragged = items[0].data;
    if (target.targetType === "root") return dragged.level > 1;
    if (target.targetType === "item") {
      if (target.targetItem === items[0].index) return false;
      const targetData = treeItems[String(target.targetItem)]?.data;
      return !!targetData && targetData.level === dragged.level;
    }
    return false;
  }

  function onDrop(items: TreeItem<AccountSummary | null>[], target: DraggingPosition) {
    const dragged = items[0]?.data;
    if (!dragged) return;
    setActionError(null);
    if (target.targetType === "root") {
      promoteMutation.mutate(dragged.id);
    } else if (target.targetType === "item") {
      demoteMutation.mutate({ id: dragged.id, parentId: Number(target.targetItem) });
    }
  }

  return (
    <div className="flex flex-col gap-4">
      <div className="flex flex-wrap items-start justify-between gap-2">
        <p className="max-w-2xl text-xs text-muted-foreground">{t("accounts.dragHint")}</p>
        <NewButton
          onClickAction={() => {
            setCreateParent(null);
            setCreating(true);
          }}
        >
          {t("accounts.newCode")}
        </NewButton>
      </div>

      {actionError && (
        <p role="alert" className="text-sm text-danger">
          {actionError}
        </p>
      )}

      {isLoading || !accounts ? (
        <p className="text-sm text-muted-foreground">{t("common.loading")}</p>
      ) : accounts.length === 0 ? (
        <p className="text-sm text-muted-foreground">{t("accounts.emptyBranch")}</p>
      ) : (
        <div className="rct-rtl overflow-x-auto rounded-md border border-border bg-surface p-2">
          <UncontrolledTreeEnvironment
            dataProvider={dataProvider}
            getItemTitle={(item) => (item.data ? `${ownCode(item.data)} ${item.data.name}` : "")}
            viewState={{ accounts: { expandedItems: initialExpanded } }}
            canDragAndDrop
            canDropOnFolder
            canDropOnNonFolder={false}
            canReorderItems={false}
            canDrag={(items) => items.length === 1 && items[0].data?.childCount === 0}
            canDropAt={canDropAt}
            onDrop={onDrop}
            renderItemTitle={({ item }) => {
              const a = item.data;
              if (!a) return null;
              return (
                <span className="flex w-full min-w-0 items-center justify-between gap-2">
                  <span className="flex min-w-0 items-center gap-2">
                    <span className="tabular-nums text-xs text-muted-foreground">{toPersianDigits(ownCode(a))}</span>
                    <span className="truncate">{a.name}</span>
                    {a.isLocked && <LockIcon locked className="h-3.5 w-3.5 shrink-0 text-warning" />}
                  </span>
                  <button
                    type="button"
                    onClick={(e) => {
                      e.stopPropagation();
                      setEditingId(a.id);
                    }}
                    aria-label={t("accounts.editAccount")}
                    className="flex h-6 w-6 shrink-0 cursor-pointer items-center justify-center rounded text-muted-foreground hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-accent"
                  >
                    <EditIcon className="h-3.5 w-3.5" />
                  </button>
                </span>
              );
            }}
          >
            <Tree treeId="accounts" rootItem="root" treeLabel={t("accounts.title")} />
          </UncontrolledTreeEnvironment>
        </div>
      )}

      <Modal
        open={creating}
        onCloseAction={() => {
          setCreating(false);
          setCreateParent(null);
        }}
        title={t("accounts.newAccountTitle")}
      >
        <form
          onSubmit={createForm.handleSubmit((values) => createMutation.mutate(values))}
          className="flex flex-col gap-4"
        >
          <div className="flex items-center gap-2 rounded-md bg-muted px-3 py-2 text-sm text-muted-foreground">
            <span>{t("accounts.parentLabel")}:</span>
            <span className="font-medium text-foreground">
              {createParent ? `${toPersianDigits(ownCode(createParent))} ${createParent.name}` : t("accounts.root")}
            </span>
            <AccountPicker triggerLabel={t("accounts.selectTargetParent")} onSelect={setCreateParent} />
          </div>
          <div className="grid grid-cols-2 gap-3">
            <Field label={t("accounts.codeLabel")}>
              <input
                type="number"
                placeholder="11"
                {...createForm.register("code", { valueAsNumber: true })}
                className={fieldInputClass}
                autoFocus
              />
            </Field>
            <Field label={t("accounts.nameLabel")}>
              <input type="text" placeholder="بانک ملت" {...createForm.register("name")} className={fieldInputClass} />
            </Field>
          </div>
          <div className="flex gap-3">
            <button
              type="submit"
              className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("common.create")}
            </button>
            <button
              type="button"
              onClick={() => {
                setCreating(false);
                setCreateParent(null);
              }}
              className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
            >
              {t("common.cancel")}
            </button>
          </div>
        </form>
      </Modal>

      {editing && (
        <Modal open={editingId !== null} onCloseAction={() => setEditingId(null)} title={t("accounts.editAccountTitle")}>
          <div className="flex flex-col gap-4">
            <div className="flex flex-wrap items-center gap-x-4 gap-y-1 rounded-md border border-border bg-muted/40 p-3 text-xs text-muted-foreground">
              <span className="rounded bg-muted px-1.5 py-0.5 font-medium text-foreground">
                {t(LEVEL_KEYS[editing.level - 1])}
              </span>
              <span>{editing.fullNamePath}</span>
              <span className={editing.childCount === 0 ? "text-success" : "text-muted-foreground"}>
                {editing.childCount === 0 ? t("accounts.postable") : t("accounts.notPostable")}
              </span>
              {editing.isLocked && <span className="text-warning">{t("accounts.lockedAccount")}</span>}
            </div>

            <form onSubmit={editForm.handleSubmit(onEditSubmit)} className="flex flex-col gap-4">
              <div className="grid grid-cols-2 gap-3">
                <Field label={t("accounts.codeLabel")}>
                  <input
                    type="number"
                    defaultValue={ownCode(editing)}
                    {...editForm.register("code", { valueAsNumber: true })}
                    className={fieldInputClass}
                    autoFocus
                  />
                </Field>
                <Field label={t("accounts.nameLabel")}>
                  <input
                    type="text"
                    defaultValue={editing.name}
                    {...editForm.register("name")}
                    className={fieldInputClass}
                  />
                </Field>
              </div>
              <div className="flex flex-wrap gap-3">
                <button
                  type="submit"
                  className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("common.save")}
                </button>
                {canLock && (
                  <button
                    type="button"
                    onClick={() => lockMutation.mutate(!editing.isLocked)}
                    className="flex h-9 cursor-pointer items-center gap-1.5 rounded-md border border-border bg-surface px-3 text-sm text-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
                  >
                    <LockIcon locked={editing.isLocked} />
                    {t("accounts.lock")}
                  </button>
                )}
                <button
                  type="button"
                  onClick={() => {
                    if (confirm(t("accounts.deleteCode") + "?")) deleteMutation.mutate();
                  }}
                  className="h-9 cursor-pointer rounded-md border border-danger px-3 text-sm text-danger hover:bg-danger/10 focus-visible:ring-2 focus-visible:ring-accent disabled:cursor-not-allowed disabled:opacity-40"
                  disabled={editing.childCount > 0}
                >
                  {t("accounts.deleteCode")}
                </button>
                <button
                  type="button"
                  onClick={() => setEditingId(null)}
                  className="ms-auto h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("common.cancel")}
                </button>
              </div>
            </form>
          </div>
        </Modal>
      )}
    </div>
  );
}
