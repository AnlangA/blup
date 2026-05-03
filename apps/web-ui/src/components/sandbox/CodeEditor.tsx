import { useEffect, useRef, useCallback } from 'react';
import { EditorState } from '@codemirror/state';
import { EditorView, keymap, lineNumbers, highlightActiveLineGutter, highlightActiveLine, drawSelection, rectangularSelection, highlightSpecialChars } from '@codemirror/view';
import { defaultKeymap, history, historyKeymap, indentWithTab } from '@codemirror/commands';
import { syntaxHighlighting, defaultHighlightStyle, bracketMatching, foldGutter, indentOnInput, foldKeymap } from '@codemirror/language';
import { python } from '@codemirror/lang-python';
import { javascript } from '@codemirror/lang-javascript';
import { rust } from '@codemirror/lang-rust';
import { oneDark } from '@codemirror/theme-one-dark';

interface CodeEditorProps {
  code: string;
  language: string;
  onChange: (value: string) => void;
}

function getLanguageExtension(lang: string) {
  switch (lang) {
    case 'python':
      return python();
    case 'javascript':
      return javascript();
    case 'rust':
      return rust();
    default:
      return [];
  }
}

export function CodeEditor({ code, language, onChange }: CodeEditorProps) {
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);

  useEffect(() => {
    onChangeRef.current = onChange;
  }, [onChange]);

  // Sync external code changes into the editor
  useEffect(() => {
    const view = viewRef.current;
    if (!view) return;
    const current = view.state.doc.toString();
    if (current !== code) {
      view.dispatch({
        changes: { from: 0, to: current.length, insert: code },
      });
    }
  }, [code]);

  const refCallback = useCallback(
    (node: HTMLDivElement | null) => {
      // Cleanup previous view
      if (viewRef.current) {
        viewRef.current.destroy();
        viewRef.current = null;
      }

      if (!node) return;

      const updateListener = EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChangeRef.current(update.state.doc.toString());
        }
      });

      const state = EditorState.create({
        doc: code,
        extensions: [
          lineNumbers(),
          highlightActiveLineGutter(),
          highlightSpecialChars(),
          history(),
          foldGutter(),
          drawSelection(),
          EditorState.allowMultipleSelections.of(true),
          indentOnInput(),
          syntaxHighlighting(defaultHighlightStyle, { fallback: true }),
          bracketMatching(),
          rectangularSelection(),
          highlightActiveLine(),
          keymap.of([
            ...defaultKeymap,
            ...historyKeymap,
            ...foldKeymap,
            indentWithTab,
          ]),
          getLanguageExtension(language),
          oneDark,
          updateListener,
          EditorView.theme({
            '&': {
              fontSize: '0.78rem',
              borderRadius: '6px',
              border: '1px solid var(--color-border)',
            },
            '.cm-content': {
              fontFamily: 'var(--font-mono)',
              padding: '0.4rem 0',
            },
            '.cm-scroller': {
              fontFamily: 'var(--font-mono)',
              lineHeight: '1.5',
              overflow: 'auto',
              maxHeight: '400px',
            },
            '.cm-gutters': {
              borderRadius: '6px 0 0 6px',
            },
          }),
          EditorView.editable.of(true),
        ],
      });

      viewRef.current = new EditorView({
        state,
        parent: node,
      });
    },
    // eslint-disable-next-line react-hooks/exhaustive-deps
    [language],
  );

  return <div ref={refCallback} className="sandbox-cm-editor" />;
}
