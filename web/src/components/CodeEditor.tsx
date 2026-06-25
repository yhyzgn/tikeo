import { linter, lintGutter } from '@codemirror/lint';
import { javascript } from '@codemirror/lang-javascript';
import { python } from '@codemirror/lang-python';
import { StreamLanguage, syntaxTree } from '@codemirror/language';
import { shell } from '@codemirror/legacy-modes/mode/shell';
import { EditorState } from '@codemirror/state';
import { oneDark } from '@codemirror/theme-one-dark';
import { EditorView, basicSetup } from 'codemirror';
import { useEffect, useRef } from 'react';

interface CodeEditorProps {
  value: string;
  onChange: (value: string) => void;
  language: string;
  readOnly?: boolean;
}

function shellLinter() {
  return linter((view) => {
    const doc = view.state.doc;
    const text = doc.toString();
    const diagnostics: Array<{ from: number; to: number; severity: 'error' | 'warning'; message: string }> = [];

    const push = (line: number, col: number, len: number, severity: 'error' | 'warning', message: string) => {
      const pos = doc.line(line).from + col;
      diagnostics.push({ from: pos, to: pos + len, severity, message });
    };

    const lines = text.split('\n');
    const keywordStack: Array<{ keyword: string; line: number }> = [];

    for (let i = 0; i < lines.length; i++) {
      const trimmed = lines[i].trim();
      if (!trimmed || trimmed.startsWith('#')) continue;

      if (/^(if|case|while|for|select)\b/.test(trimmed)) {
        const kw = trimmed.match(/^(if|case|while|for|select)\b/)![1];
        keywordStack.push({ keyword: kw, line: i });
      }
      if (/\bfi\b/.test(trimmed) && keywordStack.at(-1)?.keyword === 'if') keywordStack.pop();
      if (/\besac\b/.test(trimmed) && keywordStack.at(-1)?.keyword === 'case') keywordStack.pop();
      if (/\bdone\b/.test(trimmed) && (keywordStack.at(-1)?.keyword === 'while' || keywordStack.at(-1)?.keyword === 'for' || keywordStack.at(-1)?.keyword === 'select')) keywordStack.pop();

      let inSingle = false;
      let inDouble = false;
      let escaped = false;
      for (let c = 0; c < lines[i].length; c++) {
        const ch = lines[i][c];
        if (escaped) { escaped = false; continue; }
        if (ch === '\\') { escaped = true; continue; }
        if (ch === "'" && !inDouble) inSingle = !inSingle;
        if (ch === '"' && !inSingle) inDouble = !inDouble;
      }
      if (inSingle) push(i, lines[i].lastIndexOf("'"), 1, 'error', 'unclosed single quote');
      if (inDouble) push(i, lines[i].lastIndexOf('"'), 1, 'error', 'unclosed double quote');
    }

    for (const item of keywordStack) {
      push(item.line, 0, item.keyword.length, 'error', `${item.keyword} without matching close (fi/esac/done)`);
    }

    return diagnostics;
  });
}

function parseErrorLinter() {
  return linter((view) => {
    const diagnostics: Array<{ from: number; to: number; severity: 'error'; message: string }> = [];
    const tree = syntaxTree(view.state);
    if (!tree) return diagnostics;
    tree.iterate({
      enter(node: { type: { isError: boolean }; from: number; to: number }) {
        if (node.type.isError) {
          diagnostics.push({
            from: node.from,
            to: Math.min(node.to, node.from + 1),
            severity: 'error',
            message: 'syntax error',
          });
        }
      },
    });
    return diagnostics;
  });
}

function getExtension(lang: string) {
  switch (lang) {
    case 'python':
      return [python(), parseErrorLinter()];
    case 'node':
    case 'javascript':
    case 'js':
      return [javascript(), parseErrorLinter()];
    case 'typescript':
    case 'ts':
      return [javascript({ typescript: true }), parseErrorLinter()];
    case 'shell':
      return [StreamLanguage.define(shell), shellLinter()];
    default:
      return [];
  }
}

export function CodeEditor({ value, onChange, language, readOnly }: CodeEditorProps) {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!containerRef.current) return;

    const extensions = [
      basicSetup,
      ...getExtension(language),
      lintGutter(),
      oneDark,
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChangeRef.current(update.state.doc.toString());
        }
      }),
      EditorView.theme({
        '&': { minHeight: '240px', border: '1px solid #d9d9d9', borderRadius: '6px' },
        '.cm-content': { fontFamily: "'JetBrains Mono', 'Fira Code', 'Cascadia Code', monospace", fontSize: '14px' },
        '.cm-gutters': { borderRight: '1px solid #3c3c3c' },
      }),
    ];
    if (readOnly) {
      extensions.push(EditorState.readOnly.of(true));
    }

    const state = EditorState.create({ doc: value, extensions });
    const view = new EditorView({ state, parent: containerRef.current });
    viewRef.current = view;

    return () => {
      view.destroy();
      viewRef.current = null;
    };
  }, [language]);

  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== value) {
      view.dispatch({ changes: { from: 0, to: current.length, insert: value } });
    }
  }, [value]);

  return <div ref={containerRef} />;
}
