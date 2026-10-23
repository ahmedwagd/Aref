/**
 * User- or provider-supplied content has unknown direction, so it renders inside
 * `dir="auto"` (DESIGN.md §7.6, AGENTS.md §10). Nothing is passed as children: the
 * value must stay one text node so the browser can resolve its first strong character.
 */
type SuppliedTextProps = {
  value: string;
  className?: string;
};

export function SuppliedText({ value, className }: SuppliedTextProps) {
  return (
    <span dir="auto" className={className}>
      {value}
    </span>
  );
}
