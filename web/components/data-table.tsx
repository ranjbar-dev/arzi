"use client";

// Shared server-side sort/filter table shell: a filter row under the header
// (caller supplies the actual input/select/date controls) and sortable
// column headers that toggle asc/desc and call back into the caller's query
// params, so every register refetches from the backend rather than
// re-sorting/filtering an already-fetched page client-side.

import { useEffect, useState, type ReactNode } from "react";

export type SortState = { field: string; dir: "asc" | "desc" };

export function useDebounced<T>(value: T, delayMs = 300): T {
  const [debounced, setDebounced] = useState(value);
  useEffect(() => {
    const id = setTimeout(() => setDebounced(value), delayMs);
    return () => clearTimeout(id);
  }, [value, delayMs]);
  return debounced;
}

export function useSort(initial?: SortState) {
  const [sort, setSort] = useState<SortState | undefined>(initial);
  const toggleSort = (field: string) =>
    setSort((s) => (s?.field === field ? { field, dir: s.dir === "asc" ? "desc" : "asc" } : { field, dir: "asc" }));
  return { sort, toggleSort };
}

export type Column<T> = {
  key: string;
  header: string;
  sortable?: boolean;
  thClassName?: string;
  tdClassName?: string;
  filter?: ReactNode;
  render: (row: T) => ReactNode;
};

export function DataTable<T>({
  columns,
  rows,
  isLoading,
  rowKeyAction,
  sort,
  onSortAction,
  onRowClickAction,
  emptyLabel,
}: {
  columns: Column<T>[];
  rows: T[] | undefined;
  isLoading?: boolean;
  rowKeyAction: (row: T) => string | number;
  sort?: SortState;
  onSortAction?: (field: string) => void;
  onRowClickAction?: (row: T) => void;
  emptyLabel?: string;
}) {
  const hasFilters = columns.some((c) => c.filter);
  return (
    <div className="overflow-x-auto rounded-md border border-border">
      <table className="w-full text-sm">
        <thead>
          <tr className="border-b border-border bg-muted/50 text-muted-foreground">
            {columns.map((c) => (
              <th key={c.key} className={`px-3 py-2 text-start font-medium ${c.thClassName ?? ""}`}>
                {c.sortable && onSortAction ? (
                  <button
                    type="button"
                    onClick={() => onSortAction(c.key)}
                    className="inline-flex cursor-pointer items-center gap-1 hover:text-foreground"
                  >
                    {c.header}
                    <span className="w-2.5 text-[10px]">{sort?.field === c.key ? (sort.dir === "asc" ? "▲" : "▼") : ""}</span>
                  </button>
                ) : (
                  c.header
                )}
              </th>
            ))}
          </tr>
          {hasFilters && (
            <tr className="border-b border-border bg-muted/20">
              {columns.map((c) => (
                <th key={c.key} className="px-2 py-1.5 font-normal">
                  {c.filter}
                </th>
              ))}
            </tr>
          )}
        </thead>
        <tbody>
          {isLoading ? (
            <tr>
              <td colSpan={columns.length} className="px-3 py-6 text-center text-sm text-muted-foreground">
                …
              </td>
            </tr>
          ) : rows && rows.length > 0 ? (
            rows.map((row) => (
              <tr
                key={rowKeyAction(row)}
                onClick={onRowClickAction ? () => onRowClickAction(row) : undefined}
                className={`border-b border-border last:border-0 hover:bg-muted ${onRowClickAction ? "cursor-pointer" : ""}`}
              >
                {columns.map((c) => (
                  <td key={c.key} className={`px-3 py-2 ${c.tdClassName ?? ""}`}>
                    {c.render(row)}
                  </td>
                ))}
              </tr>
            ))
          ) : (
            <tr>
              <td colSpan={columns.length} className="px-3 py-6 text-center text-sm text-muted-foreground">
                {emptyLabel ?? "—"}
              </td>
            </tr>
          )}
        </tbody>
      </table>
    </div>
  );
}

export function FilterInput({
  value,
  onChangeAction,
  placeholder,
}: {
  value: string;
  onChangeAction: (v: string) => void;
  placeholder?: string;
}) {
  return (
    <input
      type="text"
      value={value}
      onChange={(e) => onChangeAction(e.target.value)}
      placeholder={placeholder}
      className="h-8 w-full min-w-24 rounded border border-border bg-surface px-2 text-xs text-foreground outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-accent"
    />
  );
}

export const filterSelectClass =
  "h-8 w-full min-w-24 rounded border border-border bg-surface px-1.5 text-xs outline-none focus-visible:ring-2 focus-visible:ring-accent";

export const filterDateClass =
  "h-8 w-full min-w-28 rounded border border-border bg-surface px-1.5 text-xs outline-none focus-visible:ring-2 focus-visible:ring-accent";
