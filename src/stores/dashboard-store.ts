import { create } from "zustand";
import { DashboardMetrics } from "@/types";

interface DashboardState {
  metrics: DashboardMetrics;
  isLoading: boolean;
  error: string | null;
  lastUpdated: number | null;
  setMetrics: (metrics: DashboardMetrics) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
  updateLastUpdated: () => void;
}

export const useDashboardStore = create<DashboardState>((set) => ({
  metrics: {
    qps: 0,
    error_rate: 0,
    active_sessions: 0,
    total_requests_1h: 0,
    total_tokens_1h: 0,
    avg_latency_ms: 0,
  },
  isLoading: false,
  error: null,
  lastUpdated: null,
  setMetrics: (metrics) =>
    set({ metrics, lastUpdated: Date.now(), error: null }),
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
  updateLastUpdated: () => set({ lastUpdated: Date.now() }),
}));
