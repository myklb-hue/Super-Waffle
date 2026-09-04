/**
 * Just enough syntax colouring to read code by.
 *
 * Not a parser and not trying to be one. It colours the four things the eye
 * uses to find its place in a function — comments, strings, keywords and
 * numbers — and leaves everything else alone. A block's code is a screenful,
 * read far more often than it is written, and the alternative was a megabyte
 * of editor for a panel that shows ten lines.
 *
 * Because it is not a parser it will occasionally be wrong: a keyword inside
 * an identifier, a `#` in a URL inside a string. Both are handled; deeper
 * confusions are not worth the machinery, and being wrong about a colour is
 * not the same as being wrong about the code.
 */

export type TokenKind = 'plain' | 'comment' | 'string' | 'keyword' | 'number' | 'decorator' | 'type';

export interface Token {
  kind: TokenKind;
  text: string;
}

const KEYWORDS: Record<string, string[]> = {
  python: [
    'def', 'return', 'if', 'elif', 'else', 'for', 'while', 'in', 'not', 'and',
    'or', 'None', 'True', 'False', 'import', 'from', 'as', 'class', 'try',
    'except', 'finally', 'raise', 'with', 'lambda', 'yield', 'async', 'await',
    'pass', 'break', 'continue', 'is', 'global', 'nonlocal', 'assert', 'del',
  ],
  javascript: [
    'function', 'return', 'if', 'else', 'for', 'while', 'of', 'in', 'const',
    'let', 'var', 'null', 'undefined', 'true', 'false', 'import', 'from',
    'export', 'default', 'class', 'try', 'catch', 'finally', 'throw', 'new',
    'async', 'await', 'typeof', 'instanceof', 'break', 'continue', 'this',
  ],
  shell: [
    'if', 'then', 'else', 'fi', 'for', 'do', 'done', 'while', 'case', 'esac',
    'function', 'return', 'export', 'local', 'echo', 'exit',
  ],
};

/** The type system's own names, which a custom block's annotations use. */
const TYPES = ['Text', 'Data', 'Image', 'Audio', 'File', 'Stream', 'Tools', 'Memory', 'Exec'];

const COMMENT: Record<string, string> = {
  python: '#',
  shell: '#',
  javascript: '//',
  typescript: '//',
};

function keywordsFor(language: string): string[] {
  if (language === 'typescript') return KEYWORDS.javascript!;
  return KEYWORDS[language] ?? KEYWORDS.python!;
}

/** One line, split into coloured runs. */
export function tokenize(line: string, language: string): Token[] {
  const out: Token[] = [];
  const comment = COMMENT[language] ?? '#';
  const keywords = keywordsFor(language);
  let i = 0;
  let plain = '';

  const flush = () => {
    if (plain) {
      out.push({ kind: 'plain', text: plain });
      plain = '';
    }
  };

  while (i < line.length) {
    const rest = line.slice(i);

    if (rest.startsWith(comment)) {
      flush();
      out.push({ kind: 'comment', text: rest });
      return out;
    }

    const quote = rest[0];
    if (quote === '"' || quote === "'" || quote === '`') {
      flush();
      // A triple quote is a docstring opening; treat the rest of the line as
      // string, which is right for the common single-line case and harmless
      // for the multi-line one.
      const triple = rest.slice(0, 3);
      if (triple === '"""' || triple === "'''") {
        out.push({ kind: 'string', text: rest });
        return out;
      }
      let j = 1;
      while (j < rest.length) {
        if (rest[j] === '\\') {
          j += 2;
          continue;
        }
        if (rest[j] === quote) {
          j += 1;
          break;
        }
        j += 1;
      }
      out.push({ kind: 'string', text: rest.slice(0, j) });
      i += j;
      continue;
    }

    if (rest[0] === '@' && /^@\w/.test(rest)) {
      flush();
      const word = /^@\w+/.exec(rest)![0];
      out.push({ kind: 'decorator', text: word });
      i += word.length;
      continue;
    }

    const word = /^[A-Za-z_$][\w$]*/.exec(rest)?.[0];
    if (word) {
      flush();
      const kind: TokenKind = keywords.includes(word)
        ? 'keyword'
        : TYPES.includes(word)
          ? 'type'
          : 'plain';
      out.push({ kind, text: word });
      i += word.length;
      continue;
    }

    const number = /^\d[\d._]*/.exec(rest)?.[0];
    if (number) {
      flush();
      out.push({ kind: 'number', text: number });
      i += number.length;
      continue;
    }

    plain += line[i];
    i += 1;
  }
  flush();
  return out;
}
