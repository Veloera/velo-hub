import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { invoke } from "@tauri-apps/api/core";
import type { Provider, DashboardMetrics, SessionInfo, DecisionRecord } from "@/types";

// Query keys
export const queryKeys = {
  providers: ["providers"] as const,
  provider: (id: string) => ["provider", id] as const,
  dashboardMetrics: ["dashboard-metrics"] as const,
  activeSessions: ["active-sessions"] as const,
  sessionHistory: ["session-history"] as const,
  sessionDecisionChain: (id: string) => ["session-decision-chain", id] as const,
};

// Provider queries
export function useProviders() {
  return useQuery({
    queryKey: queryKeys.providers,
    queryFn: async () => {
      const result = await invoke<Provider[]>("get_providers");
      return result;
    },
  });
}

export function useCreateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (provider: Omit<Provider, "id" | "created_at" | "updated_at">) => {
      const result = await invoke<Provider>("create_provider", { provider });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
  });
}

export function useUpdateProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async ({ id, provider }: { id: string; provider: Partial<Provider> }) => {
      const result = await invoke<Provider>("update_provider", { id, provider });
      return result;
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
  });
}

export function useDeleteProvider() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (id: string) => {
      await invoke("delete_provider", { id });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
  });
}

export function useTestProviderConnection() {
  return useMutation({
    mutationFn: async (provider: Provider) => {
      const result = await invoke<boolean>("test_provider_connection", {
        provider,
      });
      return result;
    },
  });
}

// Dashboard queries
export function useDashboardMetrics() {
  return useQuery({
    queryKey: queryKeys.dashboardMetrics,
    queryFn: async () => {
      const result = await invoke<DashboardMetrics>("get_dashboard_metrics");
      return result;
    },
    refetchInterval: 5000, // Refresh every 5 seconds
  });
}

export function useActiveSessions() {
  return useQuery({
    queryKey: queryKeys.activeSessions,
    queryFn: async () => {
      const result = await invoke<SessionInfo[]>("get_active_sessions");
      return result;
    },
    refetchInterval: 5000, // Refresh every 5 seconds
  });
}

// Session queries
export function useSessionHistory(page: number = 0, pageSize: number = 50) {
  return useQuery({
    queryKey: [...queryKeys.sessionHistory, page, pageSize],
    queryFn: async () => {
      const result = await invoke<SessionInfo[]>("get_session_history", {
        page,
        pageSize,
      });
      return result;
    },
  });
}

export function useSessionDecisionChain(sessionId: string | null) {
  return useQuery({
    queryKey: queryKeys.sessionDecisionChain(sessionId || ''),
    queryFn: async () => {
      const result = await invoke<DecisionRecord[]>("get_session_decision_chain", {
        sessionId,
      });
      return result;
    },
    enabled: !!sessionId,
  });
}

// Consumption rankings
export function useConsumptionRankings(limit: number = 10) {
  return useQuery({
    queryKey: ["consumption-rankings", limit],
    queryFn: async () => {
      const result = await invoke<Array<{
        user_id: string;
        request_count: number;
        total_tokens: number;
        cost_usd: number;
      }>>("get_consumption_rankings", { limit });
      return result;
    },
    refetchInterval: 10000, // Refresh every 10 seconds
  });
}

// Circuit breaker status
export function useCircuitBreakerStatus() {
  return useQuery({
    queryKey: ["circuit-breaker-status"],
    queryFn: async () => {
      const result = await invoke<Array<{
        provider_id: string;
        provider_name: string;
        state: string;
        failure_count: number;
        success_count: number;
        last_failure?: number;
        opened_at?: number;
      }>>("get_circuit_breaker_status");
      return result;
    },
    refetchInterval: 5000, // Refresh every 5 seconds
  });
}

// Config management
export function useExportConfig() {
  return useMutation({
    mutationFn: async () => {
      const result = await invoke<string>("export_config");
      return result;
    },
  });
}

export function useImportConfig() {
  const queryClient = useQueryClient();
  return useMutation({
    mutationFn: async (configJson: string) => {
      await invoke("import_config", { configJson });
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: queryKeys.providers });
    },
  });
}
