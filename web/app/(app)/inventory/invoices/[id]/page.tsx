"use client";

import { useParams } from "next/navigation";
import { InvoiceEditor } from "./invoice-editor";

export default function InvoiceEditorPage() {
  const { id } = useParams<{ id: string }>();
  return <InvoiceEditor documentId={Number(id)} />;
}
