"use client";

import { useEffect } from "react";
import { useRouter } from "next/navigation";

export default function TreasuryPage() {
  const router = useRouter();
  useEffect(() => {
    router.replace("/treasury/received-cheques");
  }, [router]);
  return null;
}
