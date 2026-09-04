import { StrictMode } from 'react';
import { createRoot } from 'react-dom/client';
import '@cyberloom/ui/fonts.css';
import '@cyberloom/ui/tokens.css';
import '@cyberloom/ui/globals.css';
import { App } from './app/App';

createRoot(document.getElementById('root')!).render(
  <StrictMode>
    <App />
  </StrictMode>,
);
