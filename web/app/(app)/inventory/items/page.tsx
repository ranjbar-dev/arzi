import { t } from "@/lib/i18n/fa";
import { ItemRegister } from "./item-register";

export default function ItemsPage() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.itemsTitle")}</h1>
      <ItemRegister />
    </div>
  );
}
