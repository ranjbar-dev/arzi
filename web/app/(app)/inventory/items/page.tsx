"use client";

import { useTranslation } from "react-i18next";
import { ItemRegister } from "./item-register";

export default function ItemsPage() {
  const { t } = useTranslation();
  return (
    <div className="flex flex-col gap-4">
      <h1 className="text-lg font-semibold text-foreground">{t("inventory.itemsTitle")}</h1>
      <ItemRegister />
    </div>
  );
}
