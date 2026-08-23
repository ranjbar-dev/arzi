import { t } from "@/lib/i18n/fa";
import { WarehouseRegister } from "./warehouse-register";

export default function WarehousesPage() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.warehousesTitle")}</h1>
      <WarehouseRegister />
    </div>
  );
}
