import { useState, useEffect, useRef } from 'react';
import { createPortal } from 'react-dom';
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
import { SUPPORTED_LANGUAGES } from '../../api/generated-sandbox';
import { SandboxRunner } from '../sandbox/SandboxRunner';

const CACHE_MAX = 20;
const renderCache = new Map<string, string>();

const rehypePlugins: PluggableList = [
  [rehypeRaw],
  [rehypeKatex],
  [rehypeExpressiveCode],
  [rehypeStringify],
];

async function renderMarkdown(content: string): Promise<string> {
  const cached = renderCache.get(content);
  if (cached) return cached;

  const file = await unified()
    .use(remarkParse)
    .use(remarkGfm)
    .use(remarkMath)
    .use(remarkRehype, { allowDangerousHtml: true })
    .use(rehypePlugins)
    .process(content);

  const result = String(file);

  if (renderCache.size >= CACHE_MAX) {
    const first = renderCache.keys().next().value;
    if (first !== undefined) renderCache.delete(first);
  }
  renderCache.set(content, result);

  return result;
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

interface CodeBlock {
  language: string;
  code: string;
  container: HTMLDivElement;
  key: string;
}

interface MarkdownRendererProps {
  content: string;
}

export function MarkdownRenderer({ content }: MarkdownRendererProps) {
  const [html, setHtml] = useState<string>('');
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [codeBlocks, setCodeBlocks] = useState<CodeBlock[]>([]);
  const containerRef = useRef<HTMLDivElement>(null);
  const portalCleanupRef = useRef<(() => void) | null>(null);

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
      injectSandboxContainers(containerRef.current, setCodeBlocks, portalCleanupRef);
    }
    const cleanup = portalCleanupRef.current;
    return () => {
      if (cleanup) {
        cleanup();
      }
    };
  }, [html]);

  if (loading) {
    return <div className="markdown-body"><p>Loading...</p></div>;
  }

  if (error) {
    return (
      <div className="markdown-body" style={{ color: 'red' }}>
        <h3>Render Error</h3>
        <pre>{error}</pre>
      </div>
    );
  }

  return (
    <div className="markdown-body">
      <div ref={containerRef} dangerouslySetInnerHTML={{ __html: html }} />
      {codeBlocks.map((cb) =>
        createPortal(
          <SandboxRunner language={cb.language} code={cb.code} />,
          cb.container,
          cb.key,
        ),
      )}
    </div>
  );
}

function injectSandboxContainers(
  container: HTMLElement,
  setCodeBlocks: React.Dispatch<React.SetStateAction<CodeBlock[]>>,
  cleanupRef: React.MutableRefObject<(() => void) | null>,
) {
  const blocks: CodeBlock[] = [];
  let id = 0;

  // rehype-expressive-code v0.41+ wraps code in <div class="expressive-code">
  // containing <figure class="frame"> with <pre data-language="xxx"><code>.
  // Also handle older <figure class="expressive-code"> and plain <pre><code class="language-xxx">.
  const ecWrappers = container.querySelectorAll(
    'div.expressive-code, figure.expressive-code, figure[class*="expressive"]',
  );
  ecWrappers.forEach((wrapper) => {
    const codeEl = wrapper.querySelector('code');
    if (!codeEl) return;

    // Determine language: prefer data-language on <pre>, fall back to
    // language-xxx class on <code> for older rehype-expressive-code versions.
    let lang = '';
    const pre = wrapper.querySelector('pre[data-language]');
    if (pre) {
      lang = pre.getAttribute('data-language') || '';
    }
    if (!lang) {
      const classList = codeEl.className.split(/\s+/);
      const langClass = classList.find((c) => c.startsWith('language-'));
      lang = langClass ? langClass.replace('language-', '') : '';
    }

    const normalized = SUPPORTED_LANGUAGES[lang.toLowerCase()];
    if (!normalized) return;

    // Extract code line-by-line from .ec-line elements.
    // rehype-expressive-code renders each line as <div class="ec-line">
    // with no newline text nodes between them, so codeEl.textContent
    // would concatenate all lines into a single string without line
    // breaks (e.g. "line1line2" instead of "line1\nline2").
    const lineEls = codeEl.querySelectorAll('.ec-line');
    const code = lineEls.length > 0
      ? Array.from(lineEls).map(el => el.textContent || '').join('\n')
      : (codeEl.textContent || '');
    if (code.trim().length === 0) return;

    const portalDiv = document.createElement('div');
    portalDiv.className = 'sandbox-runner-portal';
    // Hide the expressive-code rendered block since CodeMirror editor replaces it
    (wrapper as HTMLElement).style.display = 'none';
    wrapper.insertAdjacentElement('afterend', portalDiv);

    blocks.push({
      language: lang,
      code,
      container: portalDiv,
      key: `sandbox-${id++}`,
    });
  });

  // Handle plain <pre><code class="language-xxx"> not already wrapped
  const handledPres = new Set<Element>();
  ecWrappers.forEach((w) => {
    w.querySelectorAll('pre').forEach((p) => handledPres.add(p));
  });
  container.querySelectorAll('pre').forEach((pre) => {
    if (handledPres.has(pre)) return;
    const codeEl = pre.querySelector('code');
    if (!codeEl) return;

    const classList = codeEl.className.split(/\s+/);
    const langClass = classList.find((c) => c.startsWith('language-'));
    const lang = langClass ? langClass.replace('language-', '') : '';
    const normalized = SUPPORTED_LANGUAGES[lang.toLowerCase()];
    if (!normalized) return;

    const code = codeEl.textContent || '';
    if (code.trim().length === 0) return;

    const portalDiv = document.createElement('div');
    portalDiv.className = 'sandbox-runner-portal';
    // Hide the rendered pre block since CodeMirror editor replaces it
    (pre as HTMLElement).style.display = 'none';
    pre.insertAdjacentElement('afterend', portalDiv);

    blocks.push({
      language: lang,
      code,
      container: portalDiv,
      key: `sandbox-${id++}`,
    });
  });

  // Clean up previous portal divs
  if (cleanupRef.current) {
    cleanupRef.current();
  }

  const cleanup = () => {
    blocks.forEach((b) => {
      if (b.container.parentElement) {
        b.container.remove();
      }
    });
  };
  cleanupRef.current = cleanup;

  setCodeBlocks(blocks);
}
