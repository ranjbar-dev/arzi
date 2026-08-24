"use client";

import { useEffect, useRef } from "react";
import { useTranslation } from "react-i18next";
import { XIcon } from "./x-icon";

export function Modal({
  open,
  onCloseAction,
  title,
  children,
  widthClassName = "w-[min(92vw,32rem)]",
}: {
  open: boolean;
  onCloseAction: () => void;
  title: string;
  children: React.ReactNode;
  widthClassName?: string;
}) {
  const { t } = useTranslation();
  const ref = useRef<HTMLDialogElement>(null);

  useEffect(() => {
    const dialog = ref.current;
    if (!dialog) return;
    if (open && !dialog.open) dialog.showModal();
    if (!open && dialog.open) dialog.close();
  }, [open]);

  return (
    <dialog
      ref={ref}
      onClose={onCloseAction}
      onCancel={onCloseAction}
      onClick={(e) => {
        if (e.target === ref.current) onCloseAction();
      }}
      className={`modal fixed inset-0 m-auto max-h-[85vh] ${widthClassName} overflow-y-auto rounded-lg border border-border bg-surface p-0 text-foreground shadow-2xl outline-none backdrop:cursor-default`}
    >
      <div className="flex items-center justify-between border-b border-border px-4 py-3">
        <h2 className="text-sm font-semibold text-foreground">{title}</h2>
        <button
          type="button"
          onClick={onCloseAction}
          aria-label={t("common.close")}
          className="flex h-7 w-7 cursor-pointer items-center justify-center rounded-md text-muted-foreground transition-colors duration-150 hover:bg-muted hover:text-foreground focus-visible:ring-2 focus-visible:ring-accent"
        >
          <XIcon className="h-4 w-4" />
        </button>
      </div>
      <div className="p-4">{children}</div>
    </dialog>
  );
}
