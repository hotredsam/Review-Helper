import ReactMarkdown, { type Components } from "react-markdown";
import remarkGfm from "remark-gfm";

/** Markdown → themed React elements. Every element maps to theme tokens (no
 *  hardcoded colors, no typography plugin), so it renders correctly in all
 *  themes. GitHub-flavored markdown (tables, task lists, strikethrough). */
const COMPONENTS: Components = {
  h1: (p) => <h3 className="mb-1 mt-3 text-sm font-semibold text-fg" {...p} />,
  h2: (p) => <h4 className="mb-1 mt-3 text-sm font-semibold text-fg" {...p} />,
  h3: (p) => <h5 className="mb-1 mt-2 text-xs font-semibold uppercase tracking-wide text-fg-subtle" {...p} />,
  p: (p) => <p className="my-1.5 text-sm leading-relaxed text-fg-muted" {...p} />,
  ul: (p) => <ul className="my-1.5 list-disc space-y-1 pl-5 text-sm text-fg-muted" {...p} />,
  ol: (p) => <ol className="my-1.5 list-decimal space-y-1 pl-5 text-sm text-fg-muted" {...p} />,
  li: (p) => <li className="leading-relaxed" {...p} />,
  strong: (p) => <strong className="font-semibold text-fg" {...p} />,
  em: (p) => <em className="italic" {...p} />,
  a: (p) => <a className="text-accent hover:underline" {...p} />,
  code: (p) => <code className="rounded bg-surface-2 px-1 py-0.5 font-mono text-[0.85em] text-fg" {...p} />,
  pre: (p) => <pre className="my-2 overflow-auto rounded-md bg-surface-2 p-2 text-xs text-fg" {...p} />,
  blockquote: (p) => <blockquote className="my-2 border-l-2 border-border pl-3 text-fg-subtle" {...p} />,
  hr: () => <hr className="my-3 border-border" />,
  table: (p) => <table className="my-2 w-full border-collapse text-sm" {...p} />,
  th: (p) => <th className="border border-border px-2 py-1 text-left font-medium text-fg" {...p} />,
  td: (p) => <td className="border border-border px-2 py-1 text-fg-muted" {...p} />,
};

export function MarkdownBlock({ children }: { children: string }) {
  return (
    <ReactMarkdown remarkPlugins={[remarkGfm]} components={COMPONENTS}>
      {children}
    </ReactMarkdown>
  );
}
