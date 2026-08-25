"use client";

import { TreasuryTabs } from "./treasury-tabs";

export default function TreasuryLayout({ children }: { children: React.ReactNode }) {
  return (
    <div className="flex flex-col gap-4">
      <TreasuryTabs />
      {children}
    </div>
  );
}
