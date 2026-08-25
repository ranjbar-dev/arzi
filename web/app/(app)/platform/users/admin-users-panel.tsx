"use client";

import { useState } from "react";
import { useForm } from "react-hook-form";
import { zodResolver } from "@hookform/resolvers/zod";
import { z } from "zod";
import { useMutation, useQuery, useQueryClient } from "@tanstack/react-query";
import { useTranslation } from "react-i18next";
import { apiRequest, ApiError } from "@/lib/api-client";
import { toPersianDigits } from "@/lib/format";
import { Modal } from "@/components/modal";
import { NewButton } from "@/components/new-button";
import { Field, fieldInputClass } from "@/components/form-field";
import { DataTable, FilterInput, useDebounced, useSort } from "@/components/data-table";

interface UserRow {
  id: number;
  username: string;
  isActive: boolean;
  isSuperuser: boolean;
  createdAt: string;
}

interface Permission {
  id: number;
  code: string;
  labelFa: string;
}

const createUserSchema = z.object({ username: z.string().min(1).max(50) });
type CreateUserValues = z.infer<typeof createUserSchema>;

const setPasswordSchema = z.object({ newPassword: z.string().min(8) });
type SetPasswordValues = z.infer<typeof setPasswordSchema>;

export function AdminUsersPanel() {
  const { t } = useTranslation();
  const queryClient = useQueryClient();
  const [editingPermissionsFor, setEditingPermissionsFor] = useState<number | null>(null);
  const [settingPasswordFor, setSettingPasswordFor] = useState<number | null>(null);
  const [createOpen, setCreateOpen] = useState(false);
  const [usernameFilter, setUsernameFilter] = useState("");
  const debouncedUsername = useDebounced(usernameFilter);
  const { sort, toggleSort } = useSort();

  const params = new URLSearchParams();
  if (debouncedUsername) params.set("username", debouncedUsername);
  if (sort) {
    params.set("sort", sort.field);
    params.set("order", sort.dir);
  }

  const { data: users, isLoading } = useQuery({
    queryKey: ["admin", "users", debouncedUsername, sort],
    queryFn: () => apiRequest<UserRow[]>(`/api/v1/admin/users?${params.toString()}`),
  });

  const invalidateUsers = () => queryClient.invalidateQueries({ queryKey: ["admin", "users"] });

  const {
    register: registerCreate,
    handleSubmit: handleCreateSubmit,
    reset: resetCreate,
    setError: setCreateError,
    formState: { errors: createErrors, isSubmitting: creating },
  } = useForm<CreateUserValues>({ resolver: zodResolver(createUserSchema) });

  const createMutation = useMutation({
    mutationFn: (values: CreateUserValues) =>
      apiRequest("/api/v1/admin/users", { method: "POST", body: JSON.stringify(values) }),
    onSuccess: () => {
      resetCreate();
      invalidateUsers();
      setCreateOpen(false);
    },
    onError: (err: ApiError) => {
      setCreateError("root", {
        message: err.message === "username_taken" ? t("admin.usernameTaken") : t("common.error"),
      });
    },
  });

  const toggleActiveMutation = useMutation({
    mutationFn: ({ id, enable }: { id: number; enable: boolean }) =>
      apiRequest(`/api/v1/admin/users/${id}/${enable ? "enable" : "disable"}`, { method: "POST" }),
    onSuccess: invalidateUsers,
  });

  return (
    <div className="flex flex-col gap-8">
      <div className="flex items-center justify-between">
        <h1 className="text-lg font-semibold text-foreground">{t("admin.users")}</h1>
        <NewButton onClickAction={() => setCreateOpen(true)}>{t("admin.createUser")}</NewButton>
      </div>

      <DataTable<UserRow>
        columns={[
          {
            key: "username",
            header: t("auth.username"),
            sortable: true,
            tdClassName: "text-foreground",
            filter: <FilterInput value={usernameFilter} onChangeAction={setUsernameFilter} />,
            render: (u) => u.username,
          },
          {
            key: "isActive",
            header: t("common.active"),
            sortable: true,
            render: (u) =>
              u.isActive ? (
                <span className="text-success">{t("common.active")}</span>
              ) : (
                <span className="text-muted-foreground">{t("common.inactive")}</span>
              ),
          },
          {
            key: "isSuperuser",
            header: t("shell.superuser"),
            sortable: true,
            tdClassName: "text-muted-foreground",
            render: (u) => (u.isSuperuser ? t("common.yes") : t("common.no")),
          },
          {
            key: "actions",
            header: t("common.actions"),
            render: (u) => (
              <div className="flex flex-wrap gap-3">
                <button
                  type="button"
                  onClick={() => toggleActiveMutation.mutate({ id: u.id, enable: !u.isActive })}
                  className="cursor-pointer text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {u.isActive ? t("admin.disableUser") : t("admin.enableUser")}
                </button>
                <button
                  type="button"
                  onClick={() => setSettingPasswordFor(u.id)}
                  className="cursor-pointer text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("admin.setPassword")}
                </button>
                <button
                  type="button"
                  onClick={() => setEditingPermissionsFor(u.id)}
                  className="cursor-pointer text-accent hover:underline focus-visible:ring-2 focus-visible:ring-accent"
                >
                  {t("admin.permissions")}
                </button>
              </div>
            ),
          },
        ]}
        rows={users}
        isLoading={isLoading}
        rowKeyAction={(u) => u.id}
        sort={sort}
        onSortAction={toggleSort}
      />

      <Modal open={createOpen} onCloseAction={() => setCreateOpen(false)} title={t("admin.createUser")}>
        <form onSubmit={handleCreateSubmit((values) => createMutation.mutate(values))} className="flex flex-col gap-4">
          <Field label={t("auth.username")}>
            <input id="username" placeholder="a.rezaei" {...registerCreate("username")} className={fieldInputClass} autoFocus />
          </Field>
          {(createErrors.username || createErrors.root) && (
            <p role="alert" className="text-sm text-danger">
              {createErrors.root?.message ?? t("common.error")}
            </p>
          )}
          <div className="flex gap-3">
            <button
              type="submit"
              disabled={creating}
              className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition-colors duration-150 hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60"
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

      <SetPasswordDialog
        open={settingPasswordFor !== null}
        userId={settingPasswordFor}
        onClose={() => setSettingPasswordFor(null)}
      />

      <PermissionsDialog
        open={editingPermissionsFor !== null}
        userId={editingPermissionsFor}
        onClose={() => setEditingPermissionsFor(null)}
      />
    </div>
  );
}

function SetPasswordDialog({ open, userId, onClose }: { open: boolean; userId: number | null; onClose: () => void }) {
  const { t } = useTranslation();
  const {
    register,
    handleSubmit,
    setError,
    formState: { errors, isSubmitting },
  } = useForm<SetPasswordValues>({ resolver: zodResolver(setPasswordSchema) });

  const mutation = useMutation({
    mutationFn: (values: SetPasswordValues) =>
      apiRequest(`/api/v1/admin/users/${userId}/set-password`, {
        method: "POST",
        body: JSON.stringify(values),
      }),
    onSuccess: onClose,
    onError: () => setError("root", { message: t("admin.passwordTooShort") }),
  });

  return (
    <Modal open={open} onCloseAction={onClose} title={t("admin.setPassword")}>
      <form onSubmit={handleSubmit((values) => mutation.mutate(values))} className="flex flex-col gap-4">
        <Field label={t("admin.setPassword")}>
          <input type="password" autoFocus placeholder="••••••••" {...register("newPassword")} className={fieldInputClass} />
        </Field>
        {(errors.newPassword || errors.root) && (
          <p role="alert" className="text-sm text-danger">
            {t("admin.passwordTooShort")}
          </p>
        )}
        <div className="flex gap-3">
          <button
            type="submit"
            disabled={isSubmitting}
            className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
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
    </Modal>
  );
}

function PermissionsDialog({ open, userId, onClose }: { open: boolean; userId: number | null; onClose: () => void }) {
  const { t } = useTranslation();
  const queryClient = useQueryClient();

  const { data: catalogue } = useQuery({
    queryKey: ["admin", "permissions"],
    queryFn: () => apiRequest<Permission[]>("/api/v1/admin/permissions"),
  });
  const { data: granted } = useQuery({
    queryKey: ["admin", "users", userId, "permissions"],
    queryFn: () => apiRequest<number[]>(`/api/v1/admin/users/${userId}/permissions`),
    enabled: userId !== null,
  });

  const [selected, setSelected] = useState<Set<number> | null>(null);
  const current = selected ?? new Set(granted ?? []);

  const mutation = useMutation({
    mutationFn: (permissionIds: number[]) =>
      apiRequest(`/api/v1/admin/users/${userId}/permissions`, {
        method: "PUT",
        body: JSON.stringify({ permissionIds }),
      }),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["admin", "users", userId, "permissions"] });
      onClose();
    },
  });

  return (
    <Modal open={open} onCloseAction={onClose} title={t("admin.permissions")}>
      <div className="grid max-h-64 grid-cols-2 gap-x-4 gap-y-1 overflow-y-auto">
        {catalogue?.map((p) => (
          <label key={p.id} className="flex items-center gap-2 text-sm text-foreground">
            <input
              type="checkbox"
              checked={current.has(p.id)}
              onChange={(e) => {
                const next = new Set(current);
                if (e.target.checked) next.add(p.id);
                else next.delete(p.id);
                setSelected(next);
              }}
              className="h-4 w-4 accent-accent"
            />
            <span>
              {p.labelFa} <span className="text-muted-foreground">({toPersianDigits(p.id)})</span>
            </span>
          </label>
        ))}
      </div>
      <div className="mt-4 flex gap-3">
        <button
          type="button"
          onClick={() => mutation.mutate([...current])}
          disabled={mutation.isPending}
          className="h-9 cursor-pointer rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("admin.grantPermissions")}
        </button>
        <button
          type="button"
          onClick={onClose}
          className="h-9 cursor-pointer rounded-md px-4 text-sm text-muted-foreground hover:bg-muted focus-visible:ring-2 focus-visible:ring-accent"
        >
          {t("common.cancel")}
        </button>
      </div>
    </Modal>
  );
}
