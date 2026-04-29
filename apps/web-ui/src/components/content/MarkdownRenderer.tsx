import { useState, useEffect, useRef } from 'react';
import { unified } from 'unified';
import remarkParse from 'remark-parse';
import remarkMath from 'remark-math';
import remarkGfm from 'remark-gfm';
import remarkRehype from 'remark-rehype';
import rehypeKatex from 'rehype-katex';
import rehypeRaw from 'rehype-raw';
import rehypeExpressiveCode from 'rehype-expressive-code';
import rehypeStringify from 'rehype-stringify';
import type { PluggableList } from 'unified';

const rehypePlugins: PluggableList = [
  [rehypeRaw],
  [rehypeKatex],
  [rehypeExpressiveCode],
  [rehypeStringify],
];

async function renderMarkdown(content: string): Promise<string> {
  const file = await unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkMath)
    .use(remarkRehype, { allowDangerousHtml: true })
    .use(rehypePlugins)
    .process(content);

  return String(file);
}

function executeScripts(container: HTMLElement) {
  const scripts = container.querySelectorAll('script[type="module"]');
  scripts.forEach((oldScript) => {
    const newScript = document.createElement('script');
    newScript.type = 'module';
    newScript.textContent = oldScript.textContent;
    oldScript.replaceWith(newScript);
  });
}

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  const [html, setHtml] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  useEffect(() => {
    let cancelled = false;

    renderMarkdown(content)
      .then((result) => {
        if (!cancelled) {
          setHtml(result);
          setLoading(false);
        }
      })
      .catch((err: unknown) => {
        if (!cancelled) {
          console.error('Markdown render error:', err);
          setError(err instanceof Error ? err.message : 'Unknown error');
          setLoading(false);
        }
      });

    return () => { cancelled = true; };
  }, [content]);

  useEffect(() => {
    if (containerRef.current) {
      executeScripts(containerRef.current);
    }
  }, [html]);

  if (loading) {
    return <div className="markdown-body"><p>Loading...</p></div>;
  }

  if (error) {
    return (
      <div className="markdown-body" style={{ color: 'red' }}>
        <h3>渲染错误</h3>
        <pre>{error}</pre>
      </div>
    );
  }

  return (
    <div className="markdown-body">
      <div ref={containerRef} dangerouslySetInnerHTML={{ __html: html }} />
    </div>
  );
}
