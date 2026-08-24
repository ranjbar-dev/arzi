"use client";

import * as RadixSelect from "@radix-ui/react-select";
import { ChevronIcon } from "./chevron-icon";

const NONE_VALUE = "__none__";

export type SelectOption = { value: string; label: string; disabled?: boolean };

export function Select({
  value,
  onChangeAction,
  options,
  placeholder,
  className = "h-9 w-full rounded-md border border-border bg-background px-2 text-sm outline-none focus-visible:ring-2 focus-visible:ring-accent",
  disabled,
  name,
}: {
  value: string;
  onChangeAction: (value: string) => void;
  options: SelectOption[];
  placeholder?: string;
  className?: string;
  disabled?: boolean;
  name?: string;
}) {
  return (
    <RadixSelect.Root
      value={value === "" ? NONE_VALUE : value}
      onValueChange={(v) => onChangeAction(v === NONE_VALUE ? "" : v)}
      disabled={disabled}
      name={name}
    >
      <RadixSelect.Trigger
        className={`flex items-center justify-between gap-2 disabled:opacity-60 ${className}`}
      >
        <RadixSelect.Value placeholder={placeholder ?? "—"} />
        <RadixSelect.Icon>
          <ChevronIcon className="h-3.5 w-3.5 text-muted-foreground" />
        </RadixSelect.Icon>
      </RadixSelect.Trigger>
      <RadixSelect.Portal>
        <RadixSelect.Content
          position="popper"
          sideOffset={4}
          className="z-50 overflow-hidden rounded-md border border-border bg-surface text-sm text-foreground shadow-lg"
        >
          <RadixSelect.Viewport className="p-1">
            {placeholder !== undefined && (
              <RadixSelect.Item
                value={NONE_VALUE}
                className="cursor-pointer select-none rounded-sm px-2 py-1.5 text-muted-foreground outline-none data-[highlighted]:bg-muted data-[highlighted]:text-foreground"
              >
                <RadixSelect.ItemText>{placeholder}</RadixSelect.ItemText>
              </RadixSelect.Item>
            )}
            {options.map((o) => (
              <RadixSelect.Item
                key={o.value}
                value={o.value}
                disabled={o.disabled}
                className="cursor-pointer select-none rounded-sm px-2 py-1.5 outline-none data-[highlighted]:bg-muted data-[highlighted]:text-foreground data-[disabled]:pointer-events-none data-[disabled]:opacity-50"
              >
                <RadixSelect.ItemText>{o.label}</RadixSelect.ItemText>
              </RadixSelect.Item>
            ))}
          </RadixSelect.Viewport>
        </RadixSelect.Content>
      </RadixSelect.Portal>
    </RadixSelect.Root>
  );
}
