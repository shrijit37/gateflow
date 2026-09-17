"use client";

// Shared zustand store for the workflow list (used by the home page).

import { create } from "zustand";
import { api, type WorkflowSummary } from "./api";

interface WorkflowListState {
  workflows: WorkflowSummary[];
  loading: boolean;
  error: string | null;
  refresh: () => Promise<void>;
}

export const useWorkflows = create<WorkflowListState>((set) => ({
  workflows: [],
  loading: false,
  error: null,
  refresh: async () => {
    set({ loading: true, error: null });
    try {
      const workflows = await api.listWorkflows();
      set({ workflows, loading: false });
    } catch (e) {
      set({
        loading: false,
        error: e instanceof Error ? e.message : "failed to load workflows",
      });
    }
  },
}));
