import type { MessageKey, Translate } from '../i18n';

export const navItems = ['detector', 'humanizer', 'providers', 'settings'] as const;

export type NavKey = (typeof navItems)[number];

const labelKeys: Record<NavKey, MessageKey> = {
  detector: 'nav.detector',
  humanizer: 'nav.humanizer',
  providers: 'nav.providers',
  settings: 'nav.settings',
};

/**
 * Decorative only. DESIGN.md §7.2 describes nav items as icon + label; the glyph
 * carries no meaning that the label does not, so it is hidden from assistive tech.
 */
const iconPaths: Record<NavKey, string> = {
  detector: 'M4 17l5-5 3 3 6-6',
  humanizer: 'M4 8h11M4 8l3-3M4 8l3 3M20 16H9M20 16l-3-3M20 16l-3 3',
  providers: 'M4 7h16M4 12h16M4 17h10',
  settings: 'M12 15a3 3 0 100-6 3 3 0 000 6zM4 12h2M18 12h2M12 4v2M12 18v2',
};

type NavigationProps = {
  t: Translate;
  active: NavKey;
  onSelect: (key: NavKey) => void;
};

/**
 * Sits on the inline-start side in both locales because it is the first child of a
 * row flex container and carries `border-e` (DESIGN.md §7.2, `D-02`).
 */
export function Navigation({ t, active, onSelect }: NavigationProps) {
  return (
    <nav
      aria-label={t('nav.label')}
      data-testid="app-navigation"
      className="flex w-60 shrink-0 flex-col gap-1 border-e border-sidebar-border bg-sidebar p-3 text-sidebar-foreground"
    >
      <ul className="flex flex-col gap-1">
        {navItems.map((item) => {
          const isActive = item === active;
          return (
            <li key={item}>
              <button
                type="button"
                aria-current={isActive ? 'page' : undefined}
                onClick={() => onSelect(item)}
                className={`flex h-10 w-full items-center gap-3 rounded-lg px-3 text-sm transition-colors focus-visible:ring-[3px] focus-visible:ring-sidebar-ring/50 focus-visible:outline-none ${
                  isActive
                    ? 'bg-sidebar-accent text-sidebar-accent-foreground'
                    : 'hover:bg-sidebar-accent/60'
                }`}
              >
                <svg
                  aria-hidden="true"
                  viewBox="0 0 24 24"
                  fill="none"
                  stroke="currentColor"
                  strokeWidth="1.5"
                  className="size-4"
                >
                  <path d={iconPaths[item]} strokeLinecap="round" strokeLinejoin="round" />
                </svg>
                <span>{t(labelKeys[item])}</span>
              </button>
            </li>
          );
        })}
      </ul>
    </nav>
  );
}
