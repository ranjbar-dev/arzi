import { InvoiceEditor } from "./invoice-editor";

export default async function InvoiceEditorPage({ params }: { params: Promise<{ id: string }> }) {
  const { id } = await params;
  return <InvoiceEditor documentId={Number(id)} />;
}
