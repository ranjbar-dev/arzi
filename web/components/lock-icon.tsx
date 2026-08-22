// SVG, not emoji, per the design system's icon rule.
export function LockIcon({ locked, className = "h-4 w-4" }: { locked: boolean; className?: string }) {
  return (
    <svg viewBox="0 0 24 24" fill="none" stroke="currentColor" strokeWidth={2} className={className} aria-hidden="true">
      <rect x="4" y="11" width="16" height="10" rx="2" />
      {locked ? (
        <path d="M8 11V7a4 4 0 0 1 8 0v4" />
      ) : (
        <path d="M8 11V7a4 4 0 0 1 7.6-1.8" />
      )}
    </svg>
  );
}
