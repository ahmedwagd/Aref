import type { Translate } from '../i18n';
import type { AsyncState } from '../ipc/hooks';
import type { AppStatus } from '../ipc/types';

type AppShellProps = {
  t: Translate;
  /**
   * The navigation is the shell's first child, so it lands on the inline-start side in
   * `ar` and in `en` with no direction branch (DESIGN.md §7.1/§7.2).
   */
  navigation: React.ReactNode;
  status: AsyncState<AppStatus>;
  children?: React.ReactNode;
};

export function AppShell({ t, navigation, status, children }: AppShellProps) {
  return (
    <div className="flex min-h-screen bg-background text-foreground" data-testid="app-shell">
      <a
        href="#workspace"
        className="sr-only focus:not-sr-only focus:absolute focus:z-50 focus:m-3 focus:rounded-lg focus:bg-primary focus:px-3 focus:py-2 focus:text-primary-foreground"
      >
        {t('shell.skip_to_content')}
      </a>
      {navigation}
      <div className="flex min-w-0 flex-1 flex-col">
        <header className="sticky top-0 z-40 border-b bg-background/80 px-6 py-4 backdrop-blur">
          <div className="flex flex-wrap items-baseline gap-x-3 gap-y-1">
            <h1 className="text-base font-medium">{t('app.name')}</h1>
            <p dir="auto" className="text-xs text-muted-foreground">
              {t('app.tagline')}
            </p>
            <p dir="ltr" className="ms-auto text-xs text-muted-foreground tabular-nums">
              {status.status === 'ready' ? status.data.app_version : null}
            </p>
          </div>
        </header>
        <main id="workspace" className="mx-auto w-full max-w-6xl flex-1 px-6 py-8">
          {/* A screen replaces the placeholder rather than stacking under it. */}
          {children ?? (
            <section className="rounded-3xl border bg-card p-5 text-card-foreground">
              <p dir="auto" className="text-sm">
                {t('shell.workspace_placeholder')}
              </p>
            </section>
          )}
        </main>
      </div>
    </div>
  );
}
