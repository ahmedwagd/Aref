/**
 * The status-pill grammar of DESIGN.md §7.4: inherited geometry from the upstream badge,
 * and the four variants Aref states map onto. A pill always carries text — §2.3 `D-03`
 * forbids conveying a state by colour alone, so no variant may be the only signal.
 */
export type PillVariant = 'default' | 'secondary' | 'destructive' | 'outline';

const variantClasses: Record<PillVariant, string> = {
  default: 'bg-primary text-primary-foreground',
  secondary: 'bg-secondary text-secondary-foreground',
  destructive: 'border-destructive/40 bg-destructive/10 text-destructive',
  outline: 'border-border text-foreground',
};

const baseClasses =
  'inline-flex h-5 w-fit items-center gap-1 whitespace-nowrap rounded-4xl border border-transparent px-2 py-0.5 text-xs font-medium';

type StatusPillProps = {
  text: string;
  variant?: PillVariant;
};

export function StatusPill({ text, variant = 'outline' }: StatusPillProps) {
  return <span className={`${baseClasses} ${variantClasses[variant]}`}>{text}</span>;
}
