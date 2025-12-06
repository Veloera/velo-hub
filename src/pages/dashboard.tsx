import { RealTimeMetrics } from "@/components/dashboard/real-time-metrics";
import { ActiveSessionsList } from "@/components/dashboard/active-sessions-list";
import { ConsumptionRankings } from "@/components/dashboard/consumption-rankings";
import { CircuitBreakerStatusList } from "@/components/dashboard/circuit-breaker-status";
import { useI18n } from "@/lib/i18n";

export default function DashboardPage() {
  const { t } = useI18n();
  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("Dashboard")}</h1>
        <p className="text-muted-foreground">
          {t("Monitor your Velo Hub performance and usage")}
        </p>
      </div>

      <RealTimeMetrics />

      <div className="grid gap-6 md:grid-cols-2">
        <ActiveSessionsList />
        <ConsumptionRankings />
      </div>

      <CircuitBreakerStatusList />
    </div>
  );
}
