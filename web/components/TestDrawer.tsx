"use client";

// Live streaming test drawer: POSTs to /flow/:slug and renders chunks as
// they arrive, with TTFB / total latency / byte stats and a curl equivalent.

import { useEffect, useRef, useState } from "react";
import { api, sampleBody } from "@/lib/api";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];
const BODYLESS = new Set(["GET", "DELETE"]);

interface Stats {
  status: number;
  contentType: string;
  ttfbMs: number | null;
  totalMs: number;
  bytes: number;
  chunks: number;
}

export function TestDrawer({
  slug,
  defaultMethod,
  defaultProtocol,
  onClose,
}: {
  slug: string;
  defaultMethod: string;
  defaultProtocol: string;
  onClose: () => void;
}) {
  const protocol = (["openai", "anthropic", "gemini"].includes(defaultProtocol)
    ? defaultProtocol
    : "openai") as "openai" | "anthropic" | "gemini";
  const [method, setMethod] = useState(
    defaultMethod === "ANY" ? "POST" : defaultMethod,
  );
  // Body initializes from the ingress protocol on mount. The parent mounts
  // a fresh drawer per open, so no sync effect is needed.
  const [body, setBody] = useState(() => sampleBody(protocol));
  const [output, setOutput] = useState("");
  const [stats, setStats] = useState<Stats | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [sending, setSending] = useState(false);
  const abortRef = useRef<AbortController | null>(null);
  const outRef = useRef<HTMLPreElement>(null);

  useEffect(() => {
    outRef.current?.scrollTo({ top: outRef.current.scrollHeight });
  }, [output]);

  useEffect(() => () => abortRef.current?.abort(), []);

  const url = api.flowUrl(slug);
  const curl = BODYLESS.has(method)
    ? `curl -N -X ${method} '${url}'`
    : `curl -N -X ${method} '${url}' \\\n  -H 'content-type: application/json' \\\n  -d '${body.replace(/\n/g, " ")}'`;

  async function send() {
    abortRef.current?.abort();
    const ctrl = new AbortController();
    abortRef.current = ctrl;
    setSending(true);
    setError(null);
    setOutput("");
    setStats(null);

    const start = performance.now();
    let ttfbMs: number | null = null;
    let bytes = 0;
    let chunks = 0;

    try {
      const res = await fetch(url, {
        method,
        headers: { "content-type": "application/json" },
        body: BODYLESS.has(method) ? undefined : body,
        signal: ctrl.signal,
      });

      const total = () =>
        ({
          status: res.status,
          contentType: res.headers.get("content-type") ?? "(none)",
          ttfbMs,
          totalMs: Math.round(performance.now() - start),
          bytes,
          chunks,
        }) satisfies Stats;

      if (!res.ok || !res.body) {
        const text = await res.text().catch(() => "");
        setError(`HTTP ${res.status}: ${text.slice(0, 500)}`);
        setStats(total());
        return;
      }

      const reader = res.body.getReader();
      const decoder = new TextDecoder();
      for (;;) {
        const { done, value } = await reader.read();
        if (done) break;
        if (ttfbMs === null) ttfbMs = Math.round(performance.now() - start);
        bytes += value.byteLength;
        chunks += 1;
        setOutput((prev) => prev + decoder.decode(value, { stream: true }));
      }
      setOutput((prev) => prev + decoder.decode());
      setStats(total());
    } catch (e) {
      if (e instanceof DOMException && e.name === "AbortError") {
        setError("aborted by user");
      } else {
        setError(e instanceof Error ? e.message : "request failed");
      }
      setStats((s) =>
        s ?? {
          status: 0,
          contentType: "(none)",
          ttfbMs,
          totalMs: Math.round(performance.now() - start),
          bytes,
          chunks,
        },
      );
    } finally {
      setSending(false);
    }
  }

  return (
    <div className="flex h-full flex-col border-l border-zinc-200 bg-white dark:border-zinc-800 dark:bg-zinc-950">
      <div className="flex items-center justify-between border-b border-zinc-200 px-4 py-3 dark:border-zinc-800">
        <div>
          <h2 className="text-sm font-semibold">Test stream</h2>
          <p className="font-mono text-xs text-zinc-500">{url}</p>
        </div>
        <button className="btn-ghost" onClick={onClose}>
          Close
        </button>
      </div>

      <div className="flex items-center gap-2 border-b border-zinc-200 px-4 py-2 dark:border-zinc-800">
        <select className="input w-28" value={method} onChange={(e) => setMethod(e.target.value)}>
          {METHODS.map((m) => (
            <option key={m}>{m}</option>
          ))}
        </select>
        <button className="btn-primary" disabled={sending} onClick={send}>
          {sending ? "Streaming…" : "Send"}
        </button>
        {sending && (
          <button className="btn-ghost" onClick={() => abortRef.current?.abort()}>
            Abort
          </button>
        )}
      </div>

      <div className="px-4 pt-2">
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-zinc-500">
          Request body
        </p>
        <textarea
          className="input font-mono text-xs"
          rows={8}
          value={body}
          onChange={(e) => setBody(e.target.value)}
        />
      </div>

      <div className="px-4 pt-2">
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-zinc-500">
          curl equivalent
        </p>
        <pre className="overflow-x-auto rounded bg-zinc-100 p-2 font-mono text-[11px] text-zinc-700 dark:bg-zinc-900 dark:text-zinc-300">
          {curl}
        </pre>
      </div>

      {stats && (
        <div className="flex flex-wrap gap-x-4 gap-y-1 px-4 pt-2 font-mono text-xs text-zinc-600 dark:text-zinc-400">
          <span>
            status <b>{stats.status}</b>
          </span>
          <span>
            ttfb <b>{stats.ttfbMs ?? "—"} ms</b>
          </span>
          <span>
            total <b>{stats.totalMs} ms</b>
          </span>
          <span>
            bytes <b>{stats.bytes}</b>
          </span>
          <span>
            chunks <b>{stats.chunks}</b>
          </span>
          <span className="truncate">
            ct <b>{stats.contentType}</b>
          </span>
        </div>
      )}
      {error && <p className="px-4 pt-2 text-xs text-red-600">{error}</p>}

      <div className="min-h-0 flex-1 px-4 py-2">
        <p className="mb-1 text-xs font-semibold uppercase tracking-wide text-zinc-500">
          Stream output
        </p>
        <pre
          ref={outRef}
          className="h-full min-h-40 overflow-auto rounded bg-zinc-950 p-3 font-mono text-[11px] leading-relaxed text-emerald-300"
        >
          {output || "(no chunks yet)"}
        </pre>
      </div>
    </div>
  );
}
