// Persian-Indic digit formatting at the presentation layer only
// (docs/phase-1-platform-and-auth.md §1.6) — every value stored/computed
// stays ASCII; this never touches anything before it's rendered.
const PERSIAN_DIGITS = ["۰", "۱", "۲", "۳", "۴", "۵", "۶", "۷", "۸", "۹"];

export function toPersianDigits(value: string | number): string {
  return String(value).replace(/[0-9]/g, (d) => PERSIAN_DIGITS[Number(d)]);
}
