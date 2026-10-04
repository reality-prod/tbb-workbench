/**
 * Minimal custom CodeMirror StreamLanguage for Makefiles. CodeMirror's
 * official legacy-modes bundle doesn't ship a Makefile mode, so this is a
 * small, honest, real implementation covering the constructs that matter
 * for reading/editing an RBM/tor-browser-build Makefile: comments,
 * variable references/assignments, target rules, and tab-indented recipe
 * lines (highlighted distinctly since leading-tab-vs-space is semantically
 * significant in Make).
 */
import { StreamLanguage } from "@codemirror/language";
import type { StreamParser } from "@codemirror/language";

interface MakefileState {
  inRecipe: boolean;
}

const makefileParser: StreamParser<MakefileState> = {
  startState(): MakefileState {
    return { inRecipe: false };
  },
  token(stream, state) {
    if (stream.sol()) {
      state.inRecipe = stream.peek() === "\t";
    }

    if (stream.match("#")) {
      stream.skipToEnd();
      return "comment";
    }

    if (state.inRecipe) {
      stream.eatSpace();
      if (stream.match(/\$[\(\{][^\)\}]*[\)\}]/)) return "variableName.special";
      stream.skipToEnd();
      return "string";
    }

    if (stream.match(/\$[\(\{][^\)\}]*[\)\}]/)) {
      return "variableName.special";
    }

    if (stream.match(/^[^:#=\s][^:#=]*(:=|::=|\?=|\+=|=)/)) {
      return "keyword";
    }

    if (stream.match(/^\.[A-Za-z_]+(?=\s*:)/)) {
      return "atom";
    }

    if (stream.match(/^[^\s:#][^:#]*(?=\s*:(?!=))/)) {
      return "def";
    }

    stream.next();
    return null;
  },
};

export const makefileLanguage = StreamLanguage.define(makefileParser);
