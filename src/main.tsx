import { createRoot } from 'react-dom/client';

import { App } from './App';
import './styles/index.css';

const container = document.getElementById('root');

if (container === null) {
  // The document is ours; a missing root is a broken build, not a user condition.
  throw new Error('root container missing from index.html');
}

createRoot(container).render(<App />);
