import type { Metadata } from "next";
import { Vazirmatn } from "next/font/google";
import { Providers } from "./providers";
import "./globals.css";

// Persian webfont with native Persian-Indic digit glyphs and full RTL
// coverage (docs/phase-1-platform-and-auth.md §1.6's "Persian, RTL Next.js
// shell" — the Vite/create-next-app-generated Geist pair had no Persian
// glyphs at all).
const vazirmatn = Vazirmatn({
  variable: "--font-vazirmatn",
  subsets: ["arabic", "latin"],
});

export const metadata: Metadata = {
  title: "آرزی",
  description: "سامانه یکپارچه حسابداری، انبار و خزانه‌داری",
};

export default function RootLayout({ children }: LayoutProps<"/">) {
  return (
    <html
      lang="fa"
      dir="rtl"
      className={`${vazirmatn.variable} h-full antialiased`}
    >
      <body className="min-h-full flex flex-col">
        <Providers>{children}</Providers>
      </body>
    </html>
  );
}
