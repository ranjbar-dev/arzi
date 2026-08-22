"use client";

// Client-side react-i18next instance (docs/00-overview.md's locked-in stack:
// "react-i18next, adapted to Next.js client components"). Single locale (fa)
// for now — resources are the same `fa` dictionary Server Components read
// via the plain `t()` helper in `./fa`, so there is exactly one source of
// truth for every caption either way.
import i18next from "i18next";
import { initReactI18next } from "react-i18next";
import { fa } from "./fa";

if (!i18next.isInitialized) {
  i18next.use(initReactI18next).init({
    resources: { fa: { translation: fa } },
    lng: "fa",
    fallbackLng: "fa",
    interpolation: { escapeValue: false },
  });
}

export default i18next;
