import type { Completion, CompletionContext, CompletionResult } from "@codemirror/autocomplete";
import { snippetCompletion } from "@codemirror/autocomplete";
import type { ProjectIndex } from "@/lib/tauri";

const COMMANDS = [
  "section",
  "subsection",
  "subsubsection",
  "chapter",
  "paragraph",
  "part",
  "label",
  "ref",
  "eqref",
  "autoref",
  "cref",
  "pageref",
  "cite",
  "citep",
  "citet",
  "parencite",
  "textcite",
  "footnote",
  "textbf",
  "textit",
  "emph",
  "underline",
  "texttt",
  "textsc",
  "caption",
  "centering",
  "includegraphics",
  "input",
  "include",
  "usepackage",
  "documentclass",
  "newcommand",
  "renewcommand",
  "newenvironment",
  "begin",
  "end",
  "item",
  "hline",
  "toprule",
  "midrule",
  "bottomrule",
  "frac",
  "sqrt",
  "sum",
  "int",
  "prod",
  "lim",
  "infty",
  "alpha",
  "beta",
  "gamma",
  "delta",
  "epsilon",
  "theta",
  "lambda",
  "mu",
  "pi",
  "sigma",
  "phi",
  "omega",
  "left",
  "right",
  "mathbb",
  "mathcal",
  "mathrm",
  "mathbf",
  "text",
  "vspace",
  "hspace",
  "newpage",
  "clearpage",
  "tableofcontents",
  "listoffigures",
  "listoftables",
  "bibliography",
  "bibliographystyle",
  "printbibliography",
  "maketitle",
  "title",
  "author",
  "date",
  "today",
  "url",
  "href",
  "ldots",
  "cdots",
  "quad",
  "qquad",
  "noindent",
  "linebreak",
  "newline",
  "small",
  "large",
  "Large",
  "setlength",
  "graphicspath",
  "nocite",
  "appendix",
  "abstract",
];

const ENVIRONMENTS = [
  "document",
  "abstract",
  "itemize",
  "enumerate",
  "description",
  "figure",
  "table",
  "tabular",
  "equation",
  "equation*",
  "align",
  "align*",
  "gather",
  "cases",
  "matrix",
  "pmatrix",
  "bmatrix",
  "center",
  "flushleft",
  "flushright",
  "quote",
  "verbatim",
  "lstlisting",
  "minted",
  "theorem",
  "lemma",
  "proof",
  "definition",
  "frame",
  "tikzpicture",
  "algorithm",
  "algorithmic",
  "subfigure",
  "minipage",
];

const REF_COMMANDS = ["ref", "eqref", "autoref", "cref", "Cref", "pageref", "vref", "nameref"];
const CITE_COMMANDS = [
  "cite",
  "citep",
  "citet",
  "parencite",
  "textcite",
  "autocite",
  "citeauthor",
  "citeyear",
  "nocite",
];

function snippets(): Completion[] {
  return [
    snippetCompletion("\\begin{${env}}\n\t${}\n\\end{${env}}", {
      label: "begin",
      detail: "environment",
      type: "keyword",
    }),
    snippetCompletion(
      "\\begin{figure}[htbp]\n\t\\centering\n\t\\includegraphics[width=0.8\\linewidth]{${file}}\n\t\\caption{${caption}}\n\t\\label{fig:${label}}\n\\end{figure}",
      { label: "figure", detail: "snippet", type: "keyword" },
    ),
    snippetCompletion(
      "\\begin{table}[htbp]\n\t\\centering\n\t\\begin{tabular}{${ll}}\n\t\t\\toprule\n\t\t${} \\\\\n\t\t\\bottomrule\n\t\\end{tabular}\n\t\\caption{${caption}}\n\t\\label{tab:${label}}\n\\end{table}",
      { label: "table", detail: "snippet", type: "keyword" },
    ),
    snippetCompletion("\\begin{itemize}\n\t\\item ${}\n\\end{itemize}", {
      label: "itemize",
      detail: "snippet",
      type: "keyword",
    }),
    snippetCompletion("\\begin{enumerate}\n\t\\item ${}\n\\end{enumerate}", {
      label: "enumerate",
      detail: "snippet",
      type: "keyword",
    }),
    snippetCompletion("\\begin{equation}\n\t${}\n\t\\label{eq:${label}}\n\\end{equation}", {
      label: "equation",
      detail: "snippet",
      type: "keyword",
    }),
    snippetCompletion("\\begin{align}\n\t${}\n\\end{align}", {
      label: "align",
      detail: "snippet",
      type: "keyword",
    }),
    snippetCompletion("\\section{${title}}\n\\label{sec:${label}}", {
      label: "section",
      detail: "snippet",
      type: "keyword",
    }),
    snippetCompletion("\\frac{${num}}{${den}}", {
      label: "frac",
      detail: "snippet",
      type: "function",
    }),
  ];
}

const SNIPPETS = snippets();

export type IndexSource = () => ProjectIndex | undefined;

function argumentContext(
  context: CompletionContext,
): { readonly command: string; readonly from: number } | undefined {
  const match = context.matchBefore(/\\([A-Za-z*]+)(\[[^\]]*\])?\{([^}]*)$/);

  if (match === null) {
    return undefined;
  }

  const command = /\\([A-Za-z*]+)/.exec(match.text)?.[1];

  if (command === undefined) {
    return undefined;
  }

  const brace = match.text.lastIndexOf("{");

  return { command: command.replace("*", ""), from: match.from + brace + 1 };
}

export function latexCompletions(
  index: IndexSource,
): (context: CompletionContext) => CompletionResult | null {
  return (context) => {
    const argument = argumentContext(context);

    if (argument !== undefined) {
      const current = index();

      if (argument.command === "begin" || argument.command === "end") {
        const custom = current?.environments.map((symbol) => symbol.name) ?? [];
        const names = [...new Set([...ENVIRONMENTS, ...custom])];

        return {
          from: argument.from,
          options: names.map((name) => ({ label: name, type: "type" })),
          validFor: /^[\w*]*$/,
        };
      }

      if (REF_COMMANDS.includes(argument.command)) {
        return {
          from: argument.from,
          options: (current?.labels ?? []).map((symbol) => ({
            label: symbol.name,
            type: "variable",
            detail: `${symbol.file}:${symbol.line}`,
          })),
          validFor: /^[^}]*$/,
        };
      }

      if (CITE_COMMANDS.includes(argument.command)) {
        const prefix = context.matchBefore(/[^{,]*$/);

        return {
          from: prefix?.from ?? argument.from,
          options: (current?.citations ?? []).map((symbol) => ({
            label: symbol.name,
            type: "constant",
            detail: symbol.file,
          })),
          validFor: /^[^},]*$/,
        };
      }

      return null;
    }

    const word = context.matchBefore(/\\[A-Za-z]*$/);

    if (word === null || (word.from === word.to && !context.explicit)) {
      return null;
    }

    const custom = index()?.commands.map((symbol) => symbol.name) ?? [];
    const names = [...new Set([...COMMANDS, ...custom])];
    const options: Completion[] = names.map((name) => ({ label: `\\${name}`, type: "function" }));

    return {
      from: word.from,
      options: [
        ...options,
        ...SNIPPETS.map((snippet) => ({ ...snippet, label: `\\${snippet.label}` })),
      ],
      validFor: /^\\[A-Za-z]*$/,
    };
  };
}
