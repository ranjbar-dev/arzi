"use client";

import { useTranslation } from "react-i18next";
import { UnitRegister } from "./unit-register";

export default function UnitsOfMeasurePage() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.unitsTitle")}</h1>
      <UnitRegister />
    </div>
  );
}
