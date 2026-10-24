import { useEffect, useMemo, useState } from 'react';

import { AppShell } from './components/AppShell';
import { Navigation, type NavKey } from './components/Navigation';
import { ProvidersScreen } from './features/providers/ProvidersScreen';
import { applyDocumentLocale, createTranslator, defaultLocale, type Locale } from './i18n';
import { useAppStatus } from './ipc';

type AppProps = {
  /**
   * Tests mount the shell once per locale. There is no user-facing switch yet: a
   * language setting belongs to the settings change, not to the scaffold.
   */
  locale?: Locale;
};

export function App({ locale = defaultLocale }: AppProps) {
  const [active, setActive] = useState<NavKey>('detector');
  const status = useAppStatus();
  const t = useMemo(() => createTranslator(locale), [locale]);

  useEffect(() => {
    applyDocumentLocale(locale);
  }, [locale]);

  return (
    <AppShell t={t} status={status} navigation={<Navigation t={t} active={active} onSelect={setActive} />}>
      {active === 'providers' ? (
        <ProvidersScreen t={t} status={status} onOpenSettings={() => setActive('settings')} />
      ) : null}
    </AppShell>
  );
}
