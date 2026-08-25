"use client";

import { useParams } from "next/navigation";
import { VoucherEditor } from "./editor";

export default function VoucherEditorPage() {
  const { id } = useParams<{ id: string }>();
  return <VoucherEditor voucherId={Number(id)} />;
}
