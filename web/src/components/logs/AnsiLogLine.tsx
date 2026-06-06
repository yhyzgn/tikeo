import type { ReactNode } from 'react';

type AnsiStyle = {
  foreground?: string;
  background?: string;
  fontWeight?: 700;
};

const ANSI_COLOR_CLASS: Record<number, string> = {
  30: 'ansi-fg-black',
  31: 'ansi-fg-red',
  32: 'ansi-fg-green',
  33: 'ansi-fg-yellow',
  34: 'ansi-fg-blue',
  35: 'ansi-fg-magenta',
  36: 'ansi-fg-cyan',
  37: 'ansi-fg-white',
  90: 'ansi-fg-bright-black',
  91: 'ansi-fg-bright-red',
  92: 'ansi-fg-bright-green',
  93: 'ansi-fg-bright-yellow',
  94: 'ansi-fg-bright-blue',
  95: 'ansi-fg-bright-magenta',
  96: 'ansi-fg-bright-cyan',
  97: 'ansi-fg-bright-white',
};

const ANSI_BACKGROUND_CLASS: Record<number, string> = {
  40: 'ansi-bg-black',
  41: 'ansi-bg-red',
  42: 'ansi-bg-green',
  43: 'ansi-bg-yellow',
  44: 'ansi-bg-blue',
  45: 'ansi-bg-magenta',
  46: 'ansi-bg-cyan',
  47: 'ansi-bg-white',
  100: 'ansi-bg-bright-black',
  101: 'ansi-bg-bright-red',
  102: 'ansi-bg-bright-green',
  103: 'ansi-bg-bright-yellow',
  104: 'ansi-bg-bright-blue',
  105: 'ansi-bg-bright-magenta',
  106: 'ansi-bg-bright-cyan',
  107: 'ansi-bg-bright-white',
};

const ansiClassName = (style: AnsiStyle) => [style.fontWeight ? 'ansi-bold' : undefined, style.foreground, style.background]
  .filter(Boolean)
  .join(' ');

const applyAnsiCode = (style: AnsiStyle, code: number): AnsiStyle => {
  if (code === 0) {
    return {};
  }
  if (code === 1) {
    return { ...style, fontWeight: 700 };
  }
  if (code === 22) {
    return { foreground: style.foreground, background: style.background };
  }
  if (code === 39) {
    return { fontWeight: style.fontWeight, background: style.background };
  }
  if (code === 49) {
    return { fontWeight: style.fontWeight, foreground: style.foreground };
  }
  if (ANSI_COLOR_CLASS[code]) {
    return { ...style, foreground: ANSI_COLOR_CLASS[code] };
  }
  if (ANSI_BACKGROUND_CLASS[code]) {
    return { ...style, background: ANSI_BACKGROUND_CLASS[code] };
  }
  return style;
};

export const renderAnsiLogLine = (message: string): ReactNode[] => {
  const parts: ReactNode[] = [];
  let lastIndex = 0;
  let style: AnsiStyle = {};
  let key = 0;
  const pattern = /\u001B\[([0-9;?]*)([A-Za-z])/g;

  for (const match of message.matchAll(pattern)) {
    const index = match.index ?? 0;
    const text = message.slice(lastIndex, index);
    if (text) {
      const className = ansiClassName(style);
      parts.push(className ? <span key={key++} className={className}>{text}</span> : text);
    }

    if (match[2] === 'm') {
      const codes = match[1].split(';').filter(Boolean).map((code) => Number.parseInt(code, 10));
      for (const code of codes.length > 0 ? codes : [0]) {
        style = applyAnsiCode(style, Number.isNaN(code) ? 0 : code);
      }
    }
    lastIndex = index + match[0].length;
  }

  const tail = message.slice(lastIndex);
  if (tail) {
    const className = ansiClassName(style);
    parts.push(className ? <span key={key++} className={className}>{tail}</span> : tail);
  }
  return parts.length > 0 ? parts : [''];
};
