import '@testing-library/jest-dom/vitest';
import { cleanup } from '@testing-library/react';
import { afterEach } from 'vitest';

afterEach(() => {
  cleanup();
  // `applyDocumentLocale` mutates the root element; reset it so a test never inherits
  // another test's direction.
  document.documentElement.removeAttribute('dir');
  document.documentElement.removeAttribute('lang');
});
