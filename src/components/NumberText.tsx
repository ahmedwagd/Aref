/**
 * A number inside an RTL sentence gets its own `dir="ltr"` element rather than forcing
 * the sentence (DESIGN.md §7.6), so digit order survives in mixed Arabic/English text.
 */
type NumberTextProps = {
  value: string;
  className?: string;
};

export function NumberText({ value, className }: NumberTextProps) {
  return (
    <bdi
      dir="ltr"
      data-testid="number-text"
      className={className === undefined ? 'tabular-nums' : `tabular-nums ${className}`}
    >
      {value}
    </bdi>
  );
}
