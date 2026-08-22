import { redirect } from "next/navigation";
import { getSession } from "@/lib/session";
import { t } from "@/lib/i18n/fa";
import { LoginForm } from "./login-form";

export default async function LoginPage() {
  const session = await getSession();
  if (session) {
    redirect("/");
  }

  return (
    <main className="flex min-h-full flex-1 items-center justify-center bg-background px-4">
      <div className="w-full max-w-sm rounded-lg border border-border bg-surface p-8 shadow-sm">
        <h1 className="mb-6 text-center text-xl font-semibold text-foreground">
          {t("common.appName")}
        </h1>
        <LoginForm />
      </div>
    </main>
  );
}
