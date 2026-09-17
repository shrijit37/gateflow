// Direct HTTP client to the Rust control-plane. No Next.js route handlers in
// the hot path — the browser talks straight to the gateway.

export type ProtocolTag = "openai" | "anthropic" | "gemini" | "raw";

export interface IngressConfig {
  method: string; // "ANY" | "GET" | "POST" | ...
  protocol: "auto" | ProtocolTag;
  timeout_ms: number;
}

export interface EgressConfig {
  upstream_url: string;
  method: string;
  headers: Record<string, string>;
  passthrough_headers: string[];
  forward_response_headers: string[];
  connect_timeout_ms: number | null;
  timeout_ms: number | null;
}

export type NodeConfig = IngressConfig | EgressConfig | Record<string, unknown>;

export interface NodeDef {
  id: string;
  kind: "ingress" | "egress";
  name: string;
  config: NodeConfig;
}

export interface EdgeDef {
  from: string;
  to: string;
}

export interface WorkflowDag {
  nodes: NodeDef[];
  edges: EdgeDef[];
}

export interface WorkflowSummary {
  id: string;
  name: string;
  slug: string;
  version: number;
  deployed: boolean;
  updated_at: string;
}

export interface WorkflowDetail extends WorkflowSummary {
  definition: WorkflowDag;
}

export interface DeployResponse {
  id: string;
  slug: string;
  deployed: boolean;
  version: number;
  errors: string[];
}

export const API_URL =
  process.env.NEXT_PUBLIC_API_URL?.replace(/\/$/, "") ?? "http://localhost:8081";

async function json<T>(res: Response): Promise<T> {
  const body = (await res.json().catch(() => ({}))) as T & { error?: string };
  if (!res.ok) {
    throw new Error(body.error ?? `HTTP ${res.status}`);
  }
  return body;
}

export const api = {
  async listWorkflows(): Promise<WorkflowSummary[]> {
    return json(await fetch(`${API_URL}/api/workflows`));
  },

  async createWorkflow(name: string, slug: string, definition: WorkflowDag): Promise<WorkflowDetail> {
    return json(
      await fetch(`${API_URL}/api/workflows`, {
        method: "POST",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name, slug, definition }),
      }),
    );
  },

  async getWorkflow(id: string): Promise<WorkflowDetail> {
    return json(await fetch(`${API_URL}/api/workflows/${id}`));
  },

  async updateWorkflow(id: string, name: string, definition: WorkflowDag): Promise<WorkflowDetail> {
    return json(
      await fetch(`${API_URL}/api/workflows/${id}`, {
        method: "PUT",
        headers: { "content-type": "application/json" },
        body: JSON.stringify({ name, definition }),
      }),
    );
  },

  async deployWorkflow(id: string): Promise<DeployResponse> {
    return json(
      await fetch(`${API_URL}/api/workflows/${id}/deploy`, { method: "POST" }),
    );
  },

  flowUrl(slug: string): string {
    return `${API_URL}/flow/${slug}`;
  },
};

/** Default starting graph: Ingress → Egress aimed at the local mock upstream. */
export function defaultDag(): WorkflowDag {
  return {
    nodes: [
      {
        id: "ingress",
        kind: "ingress",
        name: "Ingress",
        config: {
          method: "POST",
          protocol: "auto",
          timeout_ms: 120_000,
        },
      },
      {
        id: "egress",
        kind: "egress",
        name: "Egress",
        config: {
          upstream_url: "http://127.0.0.1:9099/chat",
          method: "POST",
          headers: {},
          passthrough_headers: ["authorization", "accept", "content-type"],
          forward_response_headers: [
            "content-type",
            "cache-control",
            "x-request-id",
            "retry-after",
          ],
          connect_timeout_ms: null,
          timeout_ms: null,
        },
      },
    ],
    edges: [{ from: "ingress", to: "egress" }],
  };
}

/** Sample request bodies per protocol for the test drawer. */
export function sampleBody(protocol: "auto" | ProtocolTag): string {
  switch (protocol) {
    case "anthropic":
      return JSON.stringify(
        {
          model: "claude-sonnet-4-5",
          max_tokens: 256,
          stream: true,
          anthropic_version: "2023-06-01",
          messages: [{ role: "user", content: "Say hello as a stream" }],
        },
        null,
        2,
      );
    case "gemini":
      return JSON.stringify(
        {
          contents: [{ parts: [{ text: "Say hello as a stream" }] }],
          generationConfig: { temperature: 0.7 },
        },
        null,
        2,
      );
    case "openai":
    default:
      return JSON.stringify(
        {
          model: "gpt-4o-mini",
          stream: true,
          messages: [{ role: "user", content: "Say hello as a stream" }],
        },
        null,
        2,
      );
  }
}

export function slugify(name: string): string {
  return (
    name
      .toLowerCase()
      .replace(/[^a-z0-9]+/g, "-")
      .replace(/^-+|-+$/g, "")
      .slice(0, 64) || "workflow"
  );
}