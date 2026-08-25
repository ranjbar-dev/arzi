"use client";

import { InventoryTabs } from "./inventory-tabs";

export default function InventoryLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-4">
      <InventoryTabs />
      {children}
    </div>
  );
}
