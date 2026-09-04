import type { GlobalProvider } from '@ladle/react';
import '../src/styles/fonts.css';
import '../src/styles/tokens.css';
import '../src/styles/globals.css';

/**
 * Every primitive is drawn against the shell's own ground, not a white page.
 * The panel width matches the real inspector so a component that will live
 * there is reviewed at the size it will actually be.
 */
export const Provider: GlobalProvider = ({ children }) => (
  <div
    style={{
      background: 'var(--panel)',
      color: 'var(--text-hi)',
      fontFamily: 'var(--font-ui)',
      fontSize: 'var(--fs-md)',
      minHeight: '100vh',
      padding: 'var(--sp-7)',
    }}
  >
    <div style={{ maxWidth: 'var(--inspector-w)' }}>{children}</div>
  </div>
);
