"use client";

import { useTranslation } from "react-i18next";
import { WarehouseRegister } from "./warehouse-register";

export default function WarehousesPage() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.warehousesTitle")}</h1>
      <WarehouseRegister />
    </div>
  );
}
