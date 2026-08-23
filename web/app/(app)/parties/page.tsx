import { getSession } from "@/lib/session";
import { PartyRegister } from "./party-register";

export default async function PartiesPage() {
  const session = await getSession();
  return <PartyRegister canLock={!!session?.isSuperuser} />;
}
