"use client";

// Right-hand config panel for the Ingress node.

import type { IngressConfig } from "@/lib/api";

const METHODS = ["ANY", "GET", "POST", "PUT", "PATCH", "DELETE"];
const PROTOCOLS = ["auto", "openai", "anthropic", "gemini", "raw"] as const;

export function IngressPanel({
  config,
  onChange,
}: {
  config: IngressConfig;
  onChange: (patch: Partial<IngressConfig>) => void;
}) {
  return (
    <div className="space-y-4">
      <Field label="Method">
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

      <Field label="Protocol">
        <select
          className="input"
          value={config.protocol}
          onChange={(e) =>
            onChange({ protocol: e.target.value as IngressConfig["protocol"] })
          }
        >
          {PROTOCOLS.map((p) => (
            <option key={p} value={p}>
              {p}
            </option>
          ))}
        </select>
        <p className="hint">
          <code>auto</code> sniffs headers + a 4 KiB body prefix per request.
        </p>
      </Field>

      <Field label="Timeout (ms)">
        <input
          className="input"
          type="number"
          min={1000}
          step={1000}
          value={config.timeout_ms}
          onChange={(e) => onChange({ timeout_ms: Number(e.target.value) })}
        />
      </Field>
    </div>
  );
}

export function Field({
  label,
  children,
}: {
  label: string;
  children: React.ReactNode;
}) {
  return (
    <label className="block">
      <span className="mb-1 block text-xs font-semibold uppercase tracking-wide text-zinc-500">
        {label}
      </span>
      {children}
    </label>
  );
}
