import { redirect } from "next/navigation";
import { getSession } from "@/lib/session";
import { AdminUsersPanel } from "./admin-users-panel";

/** Same guard as the nav link (see ../page.tsx) applied to direct
 * navigation — a non-superuser hitting this URL bounces to /platform rather
 * than seeing an admin shell whose every data call 403s. Still UX only: the
 * routes themselves are what actually enforce it. */
export default async function AdminUsersPage() {
  const session = await getSession();
  if (!session?.isSuperuser) {
    redirect("/platform");
  }

  return <AdminUsersPanel />;
}
