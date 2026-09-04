import type { CSSProperties } from 'react';
import type { PortType } from '../types';
import s from './ui.module.css';

export interface TypeDotProps {
  kind: PortType;
  /** A port the drag in flight cannot reach. */
  dim?: boolean;
}

/** The endpoint of a wire. Its colour is its type, which is the whole grammar
 *  of the application (SPEC 4.1). */
export function TypeDot({ kind, dim = false }: TypeDotProps) {
  return (
    <span
      className={`${s.typeDot}${dim ? ` ${s.typeDotDim}` : ''}`}
      style={{ '--c': `var(--type-${kind})` } as CSSProperties}
      role="img"
      aria-label={`${kind} port`}
    />
  );
}

export interface TypeDotsProps {
  kinds: PortType[];
}

/** The type summary on a library row: which types a block speaks, in order. */
export function TypeDots({ kinds }: TypeDotsProps) {
  return (
    <span className={s.typeDots} role="img" aria-label={`Ports: ${kinds.join(', ')}`}>
      {kinds.map((k, i) => (
        <span
          key={`${k}-${i}`}
          className={s.typeDot}
          style={{ '--c': `var(--type-${k})` } as CSSProperties}
        />
      ))}
    </span>
  );
}
