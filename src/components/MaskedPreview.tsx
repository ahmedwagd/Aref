/**
 * `masked_preview` has a known direction — mask characters plus at most four key
 * characters, never Arabic — so it must not mirror when the page is RTL. `dir="ltr"`
 * on the wrapper keeps the mask and its tail in order; `<bdi>` isolates it from
 * surrounding RTL text (design.md §9, DESIGN.md §7.6).
 */
type MaskedPreviewProps = {
  value: string;
  className?: string;
};

export function MaskedPreview({ value, className }: MaskedPreviewProps) {
  return (
    <span
      dir="ltr"
      data-testid="masked-preview"
      className={className === undefined ? 'font-mono tabular-nums' : `font-mono tabular-nums ${className}`}
    >
      <bdi>{value}</bdi>
    </span>
  );
}
