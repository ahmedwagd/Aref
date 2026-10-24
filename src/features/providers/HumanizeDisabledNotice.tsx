import { Button } from '../../components/Button';
import type { Translate } from '../../i18n';

type HumanizeDisabledNoticeProps = {
  t: Translate;
  /**
   * `AppStatus.humanize_available` — the core's own answer, computed from the same
   * selection function the request path uses (`FR-212`). This component never re-derives
   * it: a UI guess that disagreed with the run would enable a button that cannot work.
   */
  available: boolean;
  onOpenSettings: () => void;
};

/**
 * The disabled-Humanize state (`FR-107`, `FR-212`, design.md §4.4 delta 5). It renders the
 * static catalog sentence for "no provider is ready" plus the one action that can change
 * it. It renders nothing when a provider is usable, so the screen carries no dead banner.
 */
export function HumanizeDisabledNotice({ t, available, onOpenSettings }: HumanizeDisabledNoticeProps) {
  if (available) return null;

  return (
    <section
      data-testid="humanize-disabled-notice"
      className="rounded-2xl border border-border bg-secondary p-4 text-secondary-foreground"
    >
      <h3 className="text-sm font-medium">{t('providers.humanize_disabled.title')}</h3>
      <p className="mt-1 text-xs">{t('error.provider.none')}</p>
      <div className="mt-3">
        <Button variant="secondary" onClick={onOpenSettings}>
          {t('providers.humanize_disabled.action')}
        </Button>
      </div>
    </section>
  );
}
