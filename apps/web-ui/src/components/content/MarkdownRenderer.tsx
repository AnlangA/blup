import { useState, useEffect } from 'react';
import ReactMarkdown from 'react-markdown';
import remarkMath from 'remark-math';
import remarkGfm from 'remark-gfm';
import rehypeKatex from 'rehype-katex';
import rehypeRaw from 'rehype-raw';
import rehypeExpressiveCode from 'rehype-expressive-code';
import 'katex/dist/katex.min.css';

const THEMES = ['github-dark', 'github-light', 'dracula', 'nord'] as const;
type Theme = (typeof THEMES)[number];

const rehypeExpressiveCodeOptions = {
  themes: [...THEMES],
  themeCssSelector: (theme: { name: string }) => `[data-theme='${theme.name}']`,
};

function ThemeSelector() {
  const [theme, setTheme] = useState<Theme>('github-dark');

  useEffect(() => {
    document.documentElement.setAttribute('data-theme', theme);
  }, [theme]);

  return (
    <select
      className="theme-selector"
      value={theme}
      onChange={(e) => setTheme(e.target.value as Theme)}
    >
      {THEMES.map((t) => (
        <option key={t} value={t}>{t}</option>
      ))}
    </select>
  );
}

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  return (
    <div className="markdown-body">
      <ThemeSelector />
      <ReactMarkdown
        remarkPlugins={[remarkMath, remarkGfm]}
        rehypePlugins={[
          rehypeKatex,
          rehypeRaw,
          [rehypeExpressiveCode, rehypeExpressiveCodeOptions],
        ]}
        components={{
          code({ className, children, ...props }) {
            const match = /language-(\w+)/.exec(className || '');
            const inline = !match;

            if (inline) {
              return <code className="inline-code" {...props}>{children}</code>;
            }

            return <code className={className} {...props}>{children}</code>;
          },
          a({ href, children }) {
            const isExternal = href?.startsWith('http');
            return (
              <a
                href={href}
                target={isExternal ? '_blank' : undefined}
                rel={isExternal ? 'noopener noreferrer' : undefined}
              >
                {children}
              </a>
            );
          },
          table({ children }) {
            return (
              <div style={{ overflowX: 'auto' }}>
                <table>{children}</table>
              </div>
            );
          },
        }}
      >
        {content}
      </ReactMarkdown>
    </div>
  );
}
