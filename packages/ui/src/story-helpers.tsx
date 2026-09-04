import type { ReactNode } from 'react';

/** Layout helpers shared by the story files. Not exported from the package:
 *  these exist to review primitives, not to build the application with. */

export function Row({ children, gap = 12 }: { children: ReactNode; gap?: number }) {
  return (
    <div style={{ display: 'flex', alignItems: 'center', gap, flexWrap: 'wrap' }}>{children}</div>
  );
}

export function Stack({ children, gap = 16 }: { children: ReactNode; gap?: number }) {
  return <div style={{ display: 'flex', flexDirection: 'column', gap }}>{children}</div>;
}

export function Case({ name, children }: { name: string; children: ReactNode }) {
  return (
    <div style={{ display: 'flex', flexDirection: 'column', gap: 8 }}>
      <span
        style={{
          fontFamily: 'var(--font-mono)',
          fontSize: 'var(--fs-xs)',
          letterSpacing: 'var(--tr-caps)',
          textTransform: 'uppercase',
          color: 'var(--text-faint)',
        }}
      >
        {name}
      </span>
      {children}
    </div>
  );
}
