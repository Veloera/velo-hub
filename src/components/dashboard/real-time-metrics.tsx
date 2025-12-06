import { useEffect } from "react";
import {
  Activity,
  AlertCircle,
  Users,
  Clock,
  Zap,
  TrendingUp,
} from "lucide-react";
import { MetricsCard } from "./metrics-card";
import { useDashboardStore } from "@/stores/dashboard-store";
import { useDashboardMetrics } from "@/hooks/use-tauri-query";
import { useI18n } from "@/lib/i18n";

export function RealTimeMetrics() {
  const { metrics, setMetrics, setLoading, setError } = useDashboardStore();
  const { t } = useI18n();

  // Fetch dashboard metrics with 5-second auto-refresh
  const { data, isLoading, error } = useDashboardMetrics();

  useEffect(() => {
    if (data) {
      setMetrics(data);
    }
    setLoading(isLoading);
    if (error) {
      setError(error.message || t("Failed to fetch metrics"));
    }
  }, [data, isLoading, error, setMetrics, setLoading, setError, t]);

  // Format numbers for display
  const formatNumber = (num: number, decimals: number = 2): string => {
    if (num >= 1000000) {
      return `${(num / 1000000).toFixed(decimals)}M`;
    }
    if (num >= 1000) {
      return `${(num / 1000).toFixed(decimals)}K`;
    }
    return num.toFixed(decimals);
  };

  const formatPercentage = (value: number): string => {
    return `${(value * 100).toFixed(2)}%`;
  };

  return (
    <div className="grid gap-4 md:grid-cols-2 lg:grid-cols-3">
      <MetricsCard
        title={t("Queries Per Second")}
        value={formatNumber(metrics.qps)}
        description={t("Average QPS over last hour")}
        icon={Activity}
      />
      
      <MetricsCard
        title={t("Error Rate")}
        value={formatPercentage(metrics.error_rate)}
        description={t("Failed requests / Total requests")}
        icon={AlertCircle}
      />
      
      <MetricsCard
        title={t("Active Sessions")}
        value={metrics.active_sessions}
        description={t("Currently active sessions")}
        icon={Users}
      />
      
      <MetricsCard
        title={t("Total Requests (1h)")}
        value={formatNumber(metrics.total_requests_1h, 0)}
        description={t("Requests in the last hour")}
        icon={TrendingUp}
      />
      
      <MetricsCard
        title={t("Total Tokens (1h)")}
        value={formatNumber(metrics.total_tokens_1h, 0)}
        description={t("Tokens consumed in the last hour")}
        icon={Zap}
      />
      
      <MetricsCard
        title={t("Avg Latency")}
        value={`${metrics.avg_latency_ms.toFixed(0)}ms`}
        description={t("Average response time")}
        icon={Clock}
      />
    </div>
  );
}
