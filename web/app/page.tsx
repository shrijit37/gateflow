"use client";

// Workflow list + create. Talks straight to the Rust control-plane.

import { useEffect, useState } from "react";
import { useRouter } from "next/navigation";
import Link from "next/link";
import { api, defaultDag, slugify } from "@/lib/api";
import { useWorkflows } from "@/lib/workflow-store";

export default function Home() {
  const router = useRouter();
  const { workflows, loading, error, refresh } = useWorkflows();
  const [name, setName] = useState("");
  const [slug, setSlug] = useState("");
  const [slugTouched, setSlugTouched] = useState(false);
  const [creating, setCreating] = useState(false);
  const [createError, setCreateError] = useState<string | null>(null);
  const [deploying, setDeploying] = useState<string | null>(null);

  useEffect(() => {
    refresh();
  }, [refresh]);

  function onNameChange(value: string) {
    setName(value);
    if (!slugTouched) setSlug(slugify(value));
  }

  async function create() {
    if (!name.trim() || !slug.trim()) return;
    setCreating(true);
    setCreateError(null);
    try {
      const detail = await api.createWorkflow(
        name.trim(),
        slug.trim(),
        defaultDag(),
      );
      await refresh();
      router.push(`/builder/${detail.id}`);
    } catch (e) {
      setCreateError(e instanceof Error ? e.message : "create failed");
    } finally {
      setCreating(false);
    }
  }

  async function deploy(id: string) {
    setDeploying(id);
    try {
      await api.deployWorkflow(id);
      await refresh();
    } catch (e) {
      alert(e instanceof Error ? e.message : "deploy failed");
    } finally {
      setDeploying(null);
    }
  }

  return (
    <main className="mx-auto max-w-3xl px-6 py-10">
      <h1 className="text-2xl font-bold">Gateflow</h1>
      <p className="mt-1 text-sm text-zinc-500">
        Streaming workflow gateway — Ingress → Egress, zero-copy passthrough.
      </p>

      <section className="mt-8 rounded-lg border border-zinc-200 p-4 dark:border-zinc-800">
        <h2 className="text-sm font-semibold">New workflow</h2>
        <div className="mt-3 flex flex-col gap-2 sm:flex-row">
          <input
            className="input flex-1"
            placeholder="Name (e.g. OpenAI chat passthrough)"
            value={name}
            onChange={(e) => onNameChange(e.target.value)}
          />
          <input
            className="input w-56 font-mono"
            placeholder="slug"
            value={slug}
            onChange={(e) => {
              setSlugTouched(true);
              setSlug(slugify(e.target.value));
            }}
          />
          <button className="btn-primary" disabled={creating || !name.trim()} onClick={create}>
            {creating ? "Creating…" : "Create"}
          </button>
        </div>
        {createError && <p className="mt-2 text-xs text-red-600">{createError}</p>}
        <p className="hint mt-2">
          Starts as Ingress → Egress pointed at the local mock upstream
          (<code>127.0.0.1:9099/chat</code>). Open the builder to rewire it.
        </p>
      </section>

      <section className="mt-8">
        <div className="mb-3 flex items-center justify-between">
          <h2 className="text-sm font-semibold">Workflows</h2>
          <button className="btn-ghost" onClick={refresh} disabled={loading}>
            Refresh
          </button>
        </div>
        {loading && <p className="text-sm text-zinc-500">Loading…</p>}
        {error && <p className="text-sm text-red-600">{error}</p>}
        {!loading && !error && workflows.length === 0 && (
          <p className="text-sm text-zinc-500">No workflows yet — create one above.</p>
        )}
        <ul className="space-y-2">
          {workflows.map((w) => (
            <li
              key={w.id}
              className="flex flex-wrap items-center gap-3 rounded-lg border border-zinc-200 p-3 dark:border-zinc-800"
            >
              <div className="min-w-0 flex-1">
                <a className="link font-medium" href={`/builder/${w.id}`}>
                  {w.name}
                </a>
                <p className="font-mono text-xs text-zinc-500">
                  /flow/{w.slug} · v{w.version}
                </p>
              </div>
              {w.deployed ? (
                <span className="badge-green">deployed</span>
              ) : (
                <span className="badge-amber">draft</span>
              )}
              <button
                className="btn-ghost"
                disabled={deploying === w.id}
                onClick={() => deploy(w.id)}
              >
                {deploying === w.id ? "Deploying…" : "Deploy"}
              </button>
              <Link className="btn-ghost" href={`/builder/${w.id}`}>
                Open builder
              </Link>
            </li>
          ))}
        </ul>
      </section>

      <footer className="mt-10 text-xs text-zinc-400">
        API: <code className="font-mono">{api.flowUrl(":slug")}</code> · data-plane
        streams upstream bytes with no buffering.
      </footer>
    </main>
  );
}
