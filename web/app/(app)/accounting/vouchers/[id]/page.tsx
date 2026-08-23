import { VoucherEditor } from "./editor";

export default async function VoucherEditorPage({ params }: PageProps<"/accounting/vouchers/[id]">) {
  const { id } = await params;
  return <VoucherEditor voucherId={Number(id)} />;
}
