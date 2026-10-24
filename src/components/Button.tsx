import type { ReactNode } from 'react';

/**
 * The button grammar of DESIGN.md §5 (radius `lg` = 8 px for buttons and inputs) and
 * §7.4's variants. One component so a screen cannot drift into a second button style.
 */
export type ButtonVariant = 'primary' | 'secondary' | 'danger' | 'ghost';

const variantClasses: Record<ButtonVariant, string> = {
  primary: 'bg-primary text-primary-foreground hover:bg-primary/90',
  secondary:
    'border border-border bg-background text-foreground hover:bg-accent hover:text-accent-foreground',
  danger: 'border border-destructive/40 bg-background text-destructive hover:bg-destructive/10',
  ghost: 'text-muted-foreground hover:bg-accent hover:text-foreground',
};

const baseClasses =
  'inline-flex h-9 items-center justify-center gap-2 rounded-lg px-3 text-sm font-medium transition-colors focus-visible:ring-[3px] focus-visible:ring-ring/50 focus-visible:outline-none disabled:pointer-events-none disabled:opacity-50';

type ButtonProps = {
  children: ReactNode;
  /** Absent for a `submit` button: the enclosing form owns the action. */
  onClick?: (() => void) | undefined;
  type?: 'button' | 'submit';
  variant?: ButtonVariant;
  disabled?: boolean;
};

export function Button({
  children,
  onClick,
  type = 'button',
  variant = 'secondary',
  disabled = false,
}: ButtonProps) {
  return (
    <button
      type={type}
      onClick={onClick}
      disabled={disabled}
      className={`${baseClasses} ${variantClasses[variant]}`}
    >
      {children}
    </button>
  );
}
