"use client";

// Workflow builder: ReactFlow canvas + node config panels + save/deploy +
// live streaming test drawer.

import { useCallback, useEffect, useMemo, useState } from "react";
import { useParams } from "next/navigation";
import Link from "next/link";
import {
  Background,
  Controls,
  MiniMap,
  ReactFlow,
  useEdgesState,
  useNodesState,
  type Connection,
  type Edge,
} from "@xyflow/react";
import "@xyflow/react/dist/style.css";

import {
  api,
  defaultDag,
  type EgressConfig,
  type IngressConfig,
  type WorkflowDag,
  type WorkflowDetail,
} from "@/lib/api";
import { EgressPanel } from "@/components/EgressPanel";
import { IngressPanel } from "@/components/IngressPanel";
import { TestDrawer } from "@/components/TestDrawer";
import {
  nodeTypes,
  type EgressRFNode,
  type IngressRFNode,
} from "@/components/flow-nodes";

type RFNode = IngressRFNode | EgressRFNode;

const DEFAULT_INGRESS: IngressConfig = {
  method: "POST",
  protocol: "auto",
  timeout_ms: 120_000,
};

const DEFAULT_EGRESS: EgressConfig = {
  upstream_url: "http://127.0.0.1:9099/chat",
  method: "POST",
  headers: {},
  passthrough_headers: ["authorization", "accept", "content-type"],
  forward_response_headers: ["content-type", "cache-control", "x-request-id", "retry-after"],
  connect_timeout_ms: null,
  timeout_ms: null,
};

function dagToNodes(def: WorkflowDag): RFNode[] {
  return def.nodes.map((n, i) => {
    const base = {
      id: n.id,
      position: { x: 80 + i * 340, y: 180 },
    };
    if (n.kind === "ingress") {
      return {
        ...base,
        type: "ingress",
        data: {
          label: n.name,
          config: { ...DEFAULT_INGRESS, ...(n.config as Partial<IngressConfig>) },
        },
      } satisfies IngressRFNode;
    }
    return {
      ...base,
      type: "egress",
      data: {
        label: n.name,
        config: { ...DEFAULT_EGRESS, ...(n.config as Partial<EgressConfig>) },
      },
    } satisfies EgressRFNode;
  });
}

function dagToEdges(def: WorkflowDag): Edge[] {
  return def.edges.map((e) => ({
    id: `e-${e.from}-${e.to}`,
    source: e.from,
    target: e.to,
    animated: true,
  }));
}

export default function BuilderPage() {
  const params = useParams<{ id: string }>();
  const id = params.id;

  const [detail, setDetail] = useState<WorkflowDetail | null>(null);
  const [name, setName] = useState("");
  const [nodes, setNodes, onNodesChange] = useNodesState<RFNode>([]);
  const [edges, setEdges, onEdgesChange] = useEdgesState<Edge>([]);
  const [status, setStatus] = useState<string | null>(null);
  const [busy, setBusy] = useState(false);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const [loadError, setLoadError] = useState<string | null>(null);

  useEffect(() => {
    let cancelled = false;
    api
      .getWorkflow(id)
      .then((d) => {
        if (cancelled) return;
        setDetail(d);
        setName(d.name);
        setNodes(dagToNodes(d.definition));
        setEdges(dagToEdges(d.definition));
      })
      .catch((e) => {
        if (!cancelled) setLoadError(e instanceof Error ? e.message : "load failed");
      });
    return () => {
      cancelled = true;
    };
  }, [id, setNodes, setEdges]);

  const selected = useMemo(() => nodes.find((n) => n.selected), [nodes]);
  const dirty = useMemo(() => detail !== null && name !== detail.name, [name, detail]);

  const patchConfig = useCallback(
    (nodeId: string, patch: Record<string, unknown>) => {
      setNodes((nds) =>
        nds.map((n) =>
          n.id === nodeId
            ? ({
                ...n,
                data: { ...n.data, config: { ...n.data.config, ...patch } },
              } as RFNode)
            : n,
        ),
      );
    },
    [setNodes],
  );

  const onConnect = useCallback(
    (connection: Connection) => {
      // MVP is a single linear edge: any new connection replaces old ones.
      setEdges([
        {
          id: `e-${connection.source}-${connection.target}`,
          source: connection.source,
          target: connection.target,
          animated: true,
        } as Edge,
      ]);
    },
    [setEdges],
  );

  const isValidConnection = useCallback(
    (connection: Connection | Edge) => {
      const source = nodes.find((n) => n.id === connection.source);
      const target = nodes.find((n) => n.id === connection.target);
      return (
        !!source &&
        !!target &&
        source.id !== target.id &&
        source.type === "ingress" &&
        target.type === "egress"
      );
    },
    [nodes],
  );

  function currentDag(): WorkflowDag {
    return {
      nodes: nodes.map((n) => ({
        id: n.id,
        kind: n.type as "ingress" | "egress",
        name: n.data.label,
        config: n.data.config as unknown as Record<string, unknown>,
      })),
      edges: edges.map((e) => ({ from: e.source, to: e.target })),
    };
  }

  async function save() {
    setBusy(true);
    setStatus(null);
    try {
      const updated = await api.updateWorkflow(id, name, currentDag());
      setDetail(updated);
      setStatus(`Saved (v${updated.version}). Deploy to activate.`);
    } catch (e) {
      setStatus(`Save failed: ${e instanceof Error ? e.message : e}`);
    } finally {
      setBusy(false);
    }
  }

  async function deploy() {
    setBusy(true);
    setStatus(null);
    try {
      const res = await api.deployWorkflow(id);
      if (res.deployed) {
        const updated = await api.getWorkflow(id);
        setDetail(updated);
        setStatus(`Deployed v${res.version} → ${api.flowUrl(res.slug)}`);
        setEdges((es) => [...es]);
      } else {
        setStatus(`Deploy rejected: ${res.errors.join("; ")}`);
      }
    } catch (e) {
      setStatus(`Deploy failed: ${e instanceof Error ? e.message : e}`);
    } finally {
      setBusy(false);
    }
  }

  function resetCanvas() {
    const dag = defaultDag();
    setNodes(dagToNodes(dag));
    setEdges(dagToEdges(dag));
    setStatus("Canvas reset to Ingress → Egress. Save + Deploy to apply.");
  }

  if (loadError) {
    return (
      <main className="p-8">
        <p className="text-red-600">Failed to load workflow: {loadError}</p>
        <Link className="link" href="/">
          ← Back to workflows
        </Link>
      </main>
    );
  }

  if (!detail) {
    return (
      <main className="p-8 text-sm text-zinc-500">Loading workflow…</main>
    );
  }

  const ingressNode = nodes.find((n) => n.type === "ingress");

  return (
    <main className="flex h-screen flex-col">
      <header className="flex flex-wrap items-center gap-3 border-b border-zinc-200 px-4 py-2 dark:border-zinc-800">
        <Link className="link text-sm" href="/">
          ←
        </Link>
        <input
          className="input w-56"
          value={name}
          onChange={(e) => setName(e.target.value)}
          aria-label="Workflow name"
        />
        <span className="font-mono text-xs text-zinc-500">/{detail.slug}</span>
        <span className="text-xs text-zinc-500">v{detail.version}</span>
        {detail.deployed ? (
          <span className="badge-green">deployed</span>
        ) : (
          <span className="badge-amber">not deployed{dirty ? " · unsaved name" : ""}</span>
        )}
        <div className="ml-auto flex items-center gap-2">
          <button className="btn-ghost" onClick={resetCanvas}>
            Reset canvas
          </button>
          <button className="btn-ghost" disabled={busy} onClick={save}>
            Save
          </button>
          <button className="btn-primary" disabled={busy} onClick={deploy}>
            Deploy
          </button>
          <button
            className="btn-ghost"
            onClick={() => setDrawerOpen((v) => !v)}
            aria-expanded={drawerOpen}
          >
            {drawerOpen ? "Hide test" : "Test stream"}
          </button>
        </div>
      </header>

      {status && (
        <p className="border-b border-zinc-200 bg-zinc-50 px-4 py-1.5 font-mono text-xs text-zinc-700 dark:border-zinc-800 dark:bg-zinc-900 dark:text-zinc-300">
          {status}
        </p>
      )}

      <div className="flex min-h-0 flex-1">
        <div className="min-w-0 flex-1">
          <ReactFlow
            nodes={nodes}
            edges={edges}
            nodeTypes={nodeTypes}
            onNodesChange={onNodesChange}
            onEdgesChange={onEdgesChange}
            onConnect={onConnect}
            isValidConnection={isValidConnection}
            fitView
            proOptions={{ hideAttribution: true }}
          >
            <Background />
            <Controls />
            <MiniMap pannable zoomable />
          </ReactFlow>
        </div>

        <aside className="w-80 shrink-0 overflow-y-auto border-l border-zinc-200 p-4 dark:border-zinc-800">
          {!selected ? (
            <div className="text-sm text-zinc-500">
              <p className="font-semibold text-zinc-700 dark:text-zinc-300">
                Ingress → Egress
              </p>
              <p className="mt-1">
                Select a node to edit it. Only Ingress → Egress edges are allowed
                in MVP; the engine enforces the same rule on deploy.
              </p>
              {detail.deployed && (
                <p className="mt-3">
                  Live endpoint:
                  <br />
                  <code className="mt-1 block break-all rounded bg-zinc-100 p-2 font-mono text-[11px] dark:bg-zinc-900">
                    {api.flowUrl(detail.slug)}
                  </code>
                </p>
              )}
            </div>
          ) : selected.type === "ingress" ? (
            <div>
              <h2 className="mb-3 text-sm font-semibold">Ingress — {selected.data.label}</h2>
              <IngressPanel
                key={selected.id}
                config={selected.data.config as IngressConfig}
                onChange={(patch) => patchConfig(selected.id, patch)}
              />
            </div>
          ) : (
            <div>
              <h2 className="mb-3 text-sm font-semibold">Egress — {selected.data.label}</h2>
              <EgressPanel
                key={selected.id}
                config={selected.data.config as EgressConfig}
                onChange={(patch) => patchConfig(selected.id, patch)}
              />
            </div>
          )}
        </aside>

        {drawerOpen && (
          <div className="w-[480px] shrink-0">
            <TestDrawer
              slug={detail.slug}
              defaultMethod={
                ingressNode
                  ? (ingressNode.data.config as IngressConfig).method
                  : "POST"
              }
              defaultProtocol={
                ingressNode
                  ? (ingressNode.data.config as IngressConfig).protocol
                  : "auto"
              }
              onClose={() => setDrawerOpen(false)}
            />
          </div>
        )}
      </div>
    </main>
  );
}
