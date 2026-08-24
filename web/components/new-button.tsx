"use client";

import { PlusIcon } from "./plus-icon";

export function NewButton({ onClickAction, children }: { onClickAction: () => void; children: React.ReactNode }) {
  return (
    <button
      type="button"
      onClick={onClickAction}
      className="flex h-9 w-fit cursor-pointer items-center gap-1.5 rounded-md bg-primary px-4 text-sm font-medium text-primary-foreground transition-colors duration-150 hover:opacity-90 focus-visible:ring-2 focus-visible:ring-accent"
    >
      <PlusIcon className="h-4 w-4" />
      {children}
    </button>
  );
}
