import type { Digits } from "./bindings/Digits";

const PERSIAN = "۰۱۲۳۴۵۶۷۸۹";
const ARABIC = "٠١٢٣٤٥٦٧٨٩";

/** Writes a number in the chosen digits (dates themselves come formatted from Rust). */
export function localDigits(value: number | string, digits: Digits): string {
  const text = String(value);
  return digits === "persian" ? text.replace(/\d/g, (d) => PERSIAN[Number(d)]) : text;
}

/** Reads a number typed in Persian, Arabic or Latin digits; NaN if it isn't one. */
export function parseNumber(text: string): number {
  const latin = text
    .trim()
    .replace(/[۰-۹]/g, (d) => String(PERSIAN.indexOf(d)))
    .replace(/[٠-٩]/g, (d) => String(ARABIC.indexOf(d)));
  return /^-?\d+$/.test(latin) ? Number(latin) : Number.NaN;
}
