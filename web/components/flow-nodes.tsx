"use client";

// Custom ReactFlow node renderers for the two MVP node kinds.

import { Handle, Position, type Node, type NodeProps } from "@xyflow/react";
import type { EgressConfig, IngressConfig } from "@/lib/api";

export type IngressRFNode = Node<{ label: string; config: IngressConfig }, "ingress">;
export type EgressRFNode = Node<{ label: string; config: EgressConfig }, "egress">;

function Shell({
  title,
  subtitle,
  accent,
  selected,
  children,
}: {
  title: string;
  subtitle: string;
  accent: string;
  selected?: boolean;
  children: React.ReactNode;
}) {
  return (
    <div
      className={`min-w-56 rounded-lg border bg-white shadow-sm dark:bg-zinc-900 ${
        selected ? "border-blue-500 ring-1 ring-blue-500" : "border-zinc-300 dark:border-zinc-700"
      }`}
    >
      <div className={`rounded-t-lg px-3 py-1.5 text-xs font-semibold text-white ${accent}`}>
        {title}
      </div>
      <div className="px-3 py-2">
        <div className="text-sm font-medium text-zinc-900 dark:text-zinc-100">{subtitle}</div>
        <div className="mt-1 text-xs text-zinc-500 dark:text-zinc-400">{children}</div>
      </div>
    </div>
  );
}

export function IngressNode({ data, selected }: NodeProps<IngressRFNode>) {
  const cfg = data.config;
  return (
    <Shell
      title="INGRESS"
      subtitle={data.label}
      accent="bg-emerald-600"
      selected={selected}
    >
      <Handle type="target" position={Position.Left} className="!bg-emerald-600" />
      {cfg.method} · {cfg.protocol}
      <Handle type="source" position={Position.Right} className="!bg-emerald-600" />
    </Shell>
  );
}

export function EgressNode({ data, selected }: NodeProps<EgressRFNode>) {
  const cfg = data.config;
  let host = cfg.upstream_url;
  try {
    host = new URL(cfg.upstream_url).host || cfg.upstream_url;
  } catch {
    /* leave raw while the user is still typing */
  }
  return (
    <Shell title="EGRESS" subtitle={data.label} accent="bg-violet-600" selected={selected}>
      <Handle type="target" position={Position.Left} className="!bg-violet-600" />
      {cfg.method} → {host || "(no upstream)"}
      <Handle type="source" position={Position.Right} className="!bg-violet-600" />
    </Shell>
  );
}

export const nodeTypes = {
  ingress: IngressNode,
  egress: EgressNode,
};
