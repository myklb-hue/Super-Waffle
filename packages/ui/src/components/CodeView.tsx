import s from './ui.module.css';

export type TokenKind = 'keyword' | 'string' | 'number' | 'comment' | 'type' | 'func' | 'plain';

export interface Token {
  text: string;
  kind?: TokenKind;
}

const TOKEN_CLASS: Record<TokenKind, string> = {
  keyword: s.tokKeyword!,
  string: s.tokString!,
  number: s.tokNumber!,
  comment: s.tokComment!,
  type: s.tokType!,
  func: s.tokFunc!,
  plain: s.tokPlain!,
};

export interface CodeViewProps {
  lines: Token[][];
  height?: number;
  /** One-based line numbers to highlight, for example the signature the block's
   *  interface was derived from. */
  marks?: number[];
  /** One-based line number of an error; the row is tinted and its gutter red. */
  errorLine?: number;
  loading?: boolean;
}

/** Read-only source, tokenised by the engine rather than by the shell. Used in
 *  a custom block's Code view and in the code drawer. */
export function CodeView({
  lines,
  height,
  marks = [],
  errorLine,
  loading = false,
}: CodeViewProps) {
  if (loading) {
    return (
      <div className={s.codeView} style={{ height }} aria-busy="true">
        <div className={s.codeSkeleton}>
          {[70, 45, 88, 60].map((w, i) => (
            <span key={i} className={s.skeleton} style={{ width: `${w}%`, flex: 'none' }} />
          ))}
        </div>
      </div>
    );
  }

  if (lines.length === 0) {
    return (
      <div className={s.codeView} style={{ height }}>
        <div className={s.codeEmpty}>No source. Write code here, or point the block at a file.</div>
      </div>
    );
  }

  return (
    <div className={s.codeView} style={{ height }}>
      {lines.map((tokens, i) => {
        const n = i + 1;
        const cls = [s.codeLine, marks.includes(n) && s.codeMarked, errorLine === n && s.codeErrored]
          .filter(Boolean)
          .join(' ');
        return (
          <div key={n} className={cls}>
            <span className={s.codeGutter}>{n}</span>
            <span>
              {tokens.map((t, j) => (
                <span key={j} className={TOKEN_CLASS[t.kind ?? 'plain']}>
                  {t.text}
                </span>
              ))}
            </span>
          </div>
        );
      })}
    </div>
  );
}
