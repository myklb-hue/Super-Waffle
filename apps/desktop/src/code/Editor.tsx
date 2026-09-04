import { useEffect, useLayoutEffect, useRef } from 'react';
import { tokenize } from './highlight';
import s from './Editor.module.css';

export interface EditorProps {
  value: string;
  language: string;
  /** Read-only when absent, which is what File mode shows. */
  onChange?: (value: string) => void;
  /** Called when the editor loses focus: one of the two moments a block
   *  re-parses (SPEC §10.3). */
  onSettle?: () => void;
  /** A line to mark, from a parse error (SPEC §10.4). */
  errorLine?: number | null;
  className?: string;
}

/**
 * The code editor: a textarea with a coloured copy behind it.
 *
 * The textarea is real and invisible; the colours are a `<pre>` underneath it
 * holding the same text. That is the standard trick, and it is worth naming
 * why it is used here rather than a code-editor library: the caret, selection,
 * undo, IME and every keyboard habit a person has are the browser's, already
 * correct, and free. What is left to build is the colouring, which is a
 * screenful of code rather than a megabyte of dependency.
 *
 * The two layers must agree exactly on metrics or the caret drifts from the
 * glyphs, so every font and spacing value is set once, on the shared parent.
 */
export function Editor({
  value,
  language,
  onChange,
  onSettle,
  errorLine,
  className,
}: EditorProps) {
  const input = useRef<HTMLTextAreaElement>(null);
  const painted = useRef<HTMLPreElement>(null);
  const gutter = useRef<HTMLDivElement>(null);
  const lines = value.split('\n');

  // Scrolling the textarea has to scroll the paint and the gutter with it.
  const follow = () => {
    const from = input.current;
    if (!from) return;
    if (painted.current) {
      painted.current.scrollTop = from.scrollTop;
      painted.current.scrollLeft = from.scrollLeft;
    }
    if (gutter.current) gutter.current.scrollTop = from.scrollTop;
  };
  useLayoutEffect(follow, [value]);

  // Tab indents rather than leaving the editor. In a code box that is what
  // the key is for; the block's own controls are still reachable by mouse and
  // the escape hatch is Escape, which blurs.
  useEffect(() => {
    const area = input.current;
    if (!area || !onChange) return;
    const onKey = (e: KeyboardEvent) => {
      if (e.key === 'Escape') {
        area.blur();
        return;
      }
      if (e.key !== 'Tab') return;
      e.preventDefault();
      const { selectionStart: from, selectionEnd: to } = area;
      const next = `${value.slice(0, from)}    ${value.slice(to)}`;
      onChange(next);
      requestAnimationFrame(() => area.setSelectionRange(from + 4, from + 4));
    };
    area.addEventListener('keydown', onKey);
    return () => area.removeEventListener('keydown', onKey);
  }, [value, onChange]);

  return (
    <div className={`${s.editor} ${className ?? ''}`}>
      <div className={s.gutter} ref={gutter} aria-hidden="true">
        {lines.map((_, i) => (
          <div key={i} className={errorLine === i + 1 ? s.errorNumber : undefined}>
            {i + 1}
          </div>
        ))}
      </div>
      <div className={s.pane}>
        <pre className={s.paint} ref={painted} aria-hidden="true">
          {lines.map((line, i) => (
            <div key={i} className={errorLine === i + 1 ? s.errorLine : undefined}>
              {tokenize(line, language).map((token, j) => (
                <span key={j} className={s[token.kind]}>
                  {token.text}
                </span>
              ))}
              {/* A trailing newline keeps an empty line the height of a full
                  one, so the paint and the textarea stay in step. */}
              {'\n'}
            </div>
          ))}
        </pre>
        <textarea
          ref={input}
          className={s.input}
          value={value}
          readOnly={!onChange}
          spellCheck={false}
          autoCorrect="off"
          autoCapitalize="off"
          wrap="off"
          onChange={(e) => onChange?.(e.target.value)}
          onScroll={follow}
          onBlur={onSettle}
          aria-label="Block code"
        />
      </div>
    </div>
  );
}
