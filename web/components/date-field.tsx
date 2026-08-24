"use client";

// Persian (Jalali) calendar date picker. Displays and lets the user pick
// dates in the Jalali calendar; the value/onChangeAction contract stays a
// plain Gregorian "YYYY-MM-DD" ISO string so forms, zod schemas, and the API
// are untouched — only the presentation layer is Jalali.

import { useMemo } from "react";
import DatePicker from "react-multi-date-picker";
import DateObject from "react-date-object";
import persian from "react-date-object/calendars/persian";
import persian_fa from "react-date-object/locales/persian_fa";
import gregorian from "react-date-object/calendars/gregorian";
import gregorian_en from "react-date-object/locales/gregorian_en";
import { fieldInputClass } from "./form-field";

const ISO_FORMAT = "YYYY-MM-DD";

export function DateField({
  label,
  value,
  onChangeAction,
  className,
}: {
  label?: string;
  value: string;
  onChangeAction: (iso: string) => void;
  className?: string;
}) {
  const selected = useMemo(
    () =>
      value
        ? new DateObject({ date: value, format: ISO_FORMAT, calendar: gregorian }).convert(
            persian,
            persian_fa,
          )
        : null,
    [value],
  );

  return (
    <div className="flex flex-col gap-1">
      {label && <label className="text-sm text-muted-foreground">{label}</label>}
      <DatePicker
        value={selected}
        onChange={(date) => {
          const g = date ? (date as DateObject).convert(gregorian, gregorian_en) : null;
          onChangeAction(g ? g.format(ISO_FORMAT) : "");
        }}
        calendar={persian}
        locale={persian_fa}
        calendarPosition="bottom-right"
        inputClass={className ?? fieldInputClass}
      />
    </div>
  );
}
