import { useEffect, useRef } from "react";
import { EditorState, type Extension } from "@codemirror/state";
import {
  EditorView,
  keymap,
  lineNumbers,
  highlightActiveLine,
  highlightActiveLineGutter,
  highlightWhitespace,
  drawSelection,
} from "@codemirror/view";
import { defaultKeymap, history, historyKeymap, indentWithTab } from "@codemirror/commands";
import { search, searchKeymap, openSearchPanel } from "@codemirror/search";
import { yaml } from "@codemirror/lang-yaml";
import { json } from "@codemirror/lang-json";
import { markdown } from "@codemirror/lang-markdown";
import { StreamLanguage } from "@codemirror/language";
import { shell } from "@codemirror/legacy-modes/mode/shell";
import { perl } from "@codemirror/legacy-modes/mode/perl";
import { makefileLanguage } from "./makefileMode";
import styles from "./CodeEditor.module.css";

export type EditorLanguage = "yaml" | "json" | "markdown" | "shell" | "perl" | "makefile" | "plain";

export function detectLanguage(relPath: string): EditorLanguage {
  const name = relPath.split("/").pop() ?? relPath;
  const lower = name.toLowerCase();
  if (lower === "makefile" || lower === "gnumakefile" || lower.endsWith(".mk")) return "makefile";
  if (lower.endsWith(".yml") || lower.endsWith(".yaml") || lower.endsWith(".var") || lower.endsWith(".rbm")) {
    return "yaml";
  }
  if (lower.endsWith(".json")) return "json";
  if (lower.endsWith(".md") || lower.endsWith(".markdown")) return "markdown";
  if (lower.endsWith(".sh") || lower.endsWith(".bash")) return "shell";
  if (lower.endsWith(".pl") || lower.endsWith(".pm")) return "perl";
  return "plain";
}

function languageExtension(lang: EditorLanguage): Extension[] {
  switch (lang) {
    case "yaml":
      return [yaml()];
    case "json":
      return [json()];
    case "markdown":
      return [markdown()];
    case "shell":
      return [StreamLanguage.define(shell)];
    case "perl":
      return [StreamLanguage.define(perl)];
    case "makefile":
      return [makefileLanguage];
    default:
      return [];
  }
}

export interface CodeEditorHandle {
  openSearch: () => void;
}

export function CodeEditor({
  value,
  language,
  onChange,
  tabSize,
  insertSpaces,
  showWhitespace,
  readOnly,
  editorRef,
}: {
  value: string;
  language: EditorLanguage;
  onChange: (next: string) => void;
  tabSize: number;
  insertSpaces: boolean;
  showWhitespace: boolean;
  readOnly?: boolean;
  editorRef?: (handle: CodeEditorHandle) => void;
}): JSX.Element {
  const containerRef = useRef<HTMLDivElement>(null);
  const viewRef = useRef<EditorView | null>(null);
  const onChangeRef = useRef(onChange);
  onChangeRef.current = onChange;

  useEffect(() => {
    if (!containerRef.current) return;

    const extensions: Extension[] = [
      lineNumbers(),
      highlightActiveLine(),
      highlightActiveLineGutter(),
      drawSelection(),
      history(),
      search(),
      keymap.of([...defaultKeymap, ...historyKeymap, ...searchKeymap, indentWithTab]),
      EditorState.tabSize.of(tabSize),
      EditorView.contentAttributes.of({ "aria-label": "File contents" }),
      ...languageExtension(language),
      EditorView.updateListener.of((update) => {
        if (update.docChanged) {
          onChangeRef.current(update.state.doc.toString());
        }
      }),
      EditorState.readOnly.of(!!readOnly),
    ];
    if (showWhitespace) {
      extensions.push(highlightWhitespace());
    }
    if (insertSpaces) {
      extensions.push(EditorState.tabSize.of(tabSize));
    }

    const state = EditorState.create({ doc: value, extensions });
    const view = new EditorView({ state, parent: containerRef.current });
    viewRef.current = view;
    editorRef?.({ openSearch: () => openSearchPanel(view) });

    return () => {
      view.destroy();
      viewRef.current = null;
    };
    // Intentionally re-create the editor when file identity (language) or
    // static settings change, not on every value change (that would fight
    // the user's cursor position); external content reloads use the
    // `resetSignal` pattern via key= in the parent instead.
    // eslint-disable-next-line react-hooks/exhaustive-deps
  }, [language, tabSize, insertSpaces, showWhitespace, readOnly]);

  return <div ref={containerRef} className={styles.editorHost} />;
}
