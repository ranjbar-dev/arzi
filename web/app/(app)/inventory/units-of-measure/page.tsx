import { t } from "@/lib/i18n/fa";
import { UnitRegister } from "./unit-register";

export default function UnitsOfMeasurePage() {
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.unitsTitle")}</h1>
      <UnitRegister />
    </div>
  );
}
