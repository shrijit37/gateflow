"use client";

// Right-hand config panel for the Egress node.

import { useState } from "react";
import type { EgressConfig } from "@/lib/api";
import { Field } from "./IngressPanel";

const METHODS = ["GET", "POST", "PUT", "PATCH", "DELETE"];

function headersToText(headers: Record<string, string>): string {
  return Object.entries(headers)
    .map(([k, v]) => `${k}: ${v}`)
    .join("\n");
}

function textToHeaders(text: string): Record<string, string> {
  const out: Record<string, string> = {};
  for (const line of text.split("\n")) {
    const trimmed = line.trim();
    if (!trimmed) continue;
    const idx = trimmed.indexOf(":");
    if (idx <= 0) continue;
    const key = trimmed.slice(0, idx).trim().toLowerCase();
    const value = trimmed.slice(idx + 1).trim();
    if (key) out[key] = value;
  }
  return out;
}

function listToText(list: string[]): string {
  return list.join(", ");
}

function textToList(text: string): string[] {
  return text
    .split(",")
    .map((s) => s.trim().toLowerCase())
    .filter(Boolean);
}

function numOrNull(value: string): number | null {
  if (value.trim() === "") return null;
  const n = Number(value);
  return Number.isFinite(n) && n > 0 ? n : null;
}

export function EgressPanel({
  config,
  onChange,
}: {
  config: EgressConfig;
  onChange: (patch: Partial<EgressConfig>) => void;
}) {
  // Text buffers so half-typed headers don't fight the parent state.
  // The parent remounts this panel per selected node (key={node.id}), so
  // initializing from props here is correct without a sync effect.
  const [headersText, setHeadersText] = useState(() => headersToText(config.headers));
  const [passthroughText, setPassthroughText] = useState(() =>
    listToText(config.passthrough_headers),
  );
  const [forwardText, setForwardText] = useState(() =>
    listToText(config.forward_response_headers),
  );

  return (
    <div className="space-y-4">
      <Field label="Upstream URL">
        <input
          className="input font-mono text-xs"
          type="url"
          placeholder="http://127.0.0.1:9099/chat"
          value={config.upstream_url}
          onChange={(e) => onChange({ upstream_url: e.target.value })}
        />
      </Field>

      <Field label="Upstream method">
        <select
          className="input"
          value={config.method}
          onChange={(e) => onChange({ method: e.target.value })}
        >
          {METHODS.map((m) => (
            <option key={m} value={m}>
              {m}
            </option>
          ))}
        </select>
      </Field>

      <Field label="Static headers (one `Key: Value` per line, override passthrough)">
        <textarea
          className="input font-mono text-xs"
          rows={3}
          value={headersText}
          onChange={(e) => {
            setHeadersText(e.target.value);
            onChange({ headers: textToHeaders(e.target.value) });
          }}
        />
      </Field>

      <Field label="Passthrough request headers (comma-separated)">
        <textarea
          className="input font-mono text-xs"
          rows={2}
          value={passthroughText}
          onChange={(e) => {
            setPassthroughText(e.target.value);
            onChange({ passthrough_headers: textToList(e.target.value) });
          }}
        />
        <p className="hint">Client headers forwarded unless overridden above.</p>
      </Field>

      <Field label="Forwarded response headers (comma-separated)">
        <textarea
          className="input font-mono text-xs"
          rows={2}
          value={forwardText}
          onChange={(e) => {
            setForwardText(e.target.value);
            onChange({ forward_response_headers: textToList(e.target.value) });
          }}
        />
      </Field>

      <div className="grid grid-cols-2 gap-3">
        <Field label="Connect timeout ms (blank = default)">
          <input
            className="input"
            type="number"
            min={0}
            placeholder="default"
            value={config.connect_timeout_ms ?? ""}
            onChange={(e) => onChange({ connect_timeout_ms: numOrNull(e.target.value) })}
          />
        </Field>
        <Field label="Total timeout ms (blank = default)">
          <input
            className="input"
            type="number"
            min={0}
            placeholder="default"
            value={config.timeout_ms ?? ""}
            onChange={(e) => onChange({ timeout_ms: numOrNull(e.target.value) })}
          />
        </Field>
      </div>
    </div>
  );
}
