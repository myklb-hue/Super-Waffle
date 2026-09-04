import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@cyberloom/ui/fonts.css';
import '@cyberloom/ui/tokens.css';
import '@cyberloom/ui/globals.css';
import { App } from './app/App';
import { useDocument } from './stores/document';
import { useRun } from './stores/run';

// In a dev build the store is reachable from the console, which is what makes
// a canvas bug something you can inspect rather than infer. Not in a release
// build: the shell should not have a back door in the shipped product.
if (import.meta.env.DEV) {
  (window as unknown as { cyberloom: unknown }).cyberloom = { useDocument, useRun };
}

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
