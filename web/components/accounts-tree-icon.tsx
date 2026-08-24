// SVG, not emoji, per the design system's icon rule.
export function AccountsTreeIcon({ className = "h-6 w-6" }: { className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} strokeLinecap="round" strokeLinejoin="round" className={className} aria-hidden="true">
      <path d="M4 4v9a2 2 0 0 0 2 2h4M4 4h6M4 9h4" />
      <circle cx="4" cy="4" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="4" cy="9" r="1.5" fill="currentColor" stroke="none" />
      <circle cx="10" cy="15" r="1.5" fill="currentColor" stroke="none" />
      <path d="M14 6h6M14 12h6M14 18h6" />
    </svg>
  );
}
