export const fieldInputClass =
  "h-9 w-full rounded-md border border-border bg-background px-2 text-sm outline-none transition-colors duration-150 focus-visible:ring-2 focus-visible:ring-accent disabled:opacity-60";

export function Field({
  label,
  wide,
  children,
}: {
  label: string;
  wide?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div className={`flex flex-col gap-1 ${wide ? "col-span-2 sm:col-span-3" : ""}`}>
      <label className="text-sm text-muted-foreground">{label}</label>
      {children}
    </div>
  );
}
