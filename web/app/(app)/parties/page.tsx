"use client";

import { useSession } from "@/lib/use-session";
import { PartyRegister } from "./party-register";

export default function PartiesPage() {
  const { data: session } = useSession();
  return <PartyRegister canLock={!!session?.isSuperuser} />;
}
