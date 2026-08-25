"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";
import { useSession } from "@/lib/use-session";
import { AdminUsersPanel } from "./admin-users-panel";

/** Same guard as the nav link (see ../page.tsx) applied to direct
 * navigation — a non-superuser hitting this URL bounces to /platform rather
 * than seeing an admin shell whose every data call 403s. Still UX only: the
 * routes themselves are what actually enforce it. */
export default function AdminUsersPage() {
  const router = useRouter();
  const { data: session, isLoading } = useSession();

  useEffect(() => {
    if (!isLoading && !session?.isSuperuser) {
      router.replace("/platform");
    }
  }, [isLoading, session, router]);

  if (isLoading || !session?.isSuperuser) {
    return null;
  }

  return <AdminUsersPanel />;
}
