import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useCircuitBreakerStatus } from "@/hooks/use-tauri-query";
import { Shield, CheckCircle, XCircle, AlertTriangle } from "lucide-react";
import { useI18n } from "@/lib/i18n";

export function CircuitBreakerStatusList() {
  const { data: statuses, isLoading, error } = useCircuitBreakerStatus();
  const { t } = useI18n();

  const getStateIcon = (state: string) => {
    switch (state) {
      case 'closed':
        return <CheckCircle className="h-5 w-5 text-green-500" />;
      case 'open':
        return <XCircle className="h-5 w-5 text-red-500" />;
      case 'half_open':
        return <AlertTriangle className="h-5 w-5 text-yellow-500" />;
      default:
        return <Shield className="h-5 w-5 text-gray-500" />;
    }
  };

  const getStateLabel = (state: string) => {
    switch (state) {
      case 'closed':
        return t('Healthy');
      case 'open':
        return t('Circuit Open');
      case 'half_open':
        return t('Testing');
      default:
        return t('Unknown');
    }
  };

  const getStateBadgeClass = (state: string) => {
    switch (state) {
      case 'closed':
        return 'bg-green-100 text-green-800 dark:bg-green-900/30 dark:text-green-400';
      case 'open':
        return 'bg-red-100 text-red-800 dark:bg-red-900/30 dark:text-red-400';
      case 'half_open':
        return 'bg-yellow-100 text-yellow-800 dark:bg-yellow-900/30 dark:text-yellow-400';
      default:
        return 'bg-gray-100 text-gray-800 dark:bg-gray-900/30 dark:text-gray-400';
    }
  };

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    
    if (diffMins < 1) return t('Just now');
    if (diffMins < 60) return t('{count}m ago', { count: diffMins });
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return t('{count}h ago', { count: diffHours });
    const diffDays = Math.floor(diffHours / 24);
    return t('{count}d ago', { count: diffDays });
  };

  const calculateHealthScore = (status: {
    provider_id: string;
    provider_name: string;
    state: string;
    failure_count: number;
    success_count: number;
    last_failure?: number;
    opened_at?: number;
  }): number => {
    const total = status.success_count + status.failure_count;
    if (total === 0) return 100;
    return Math.round((status.success_count / total) * 100);
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Shield className="h-5 w-5" />
          {t('Circuit Breaker Status')}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading && (
          <div className="text-sm text-muted-foreground">
            {t('Loading status...')}
          </div>
        )}
        
        {error && (
          <div className="text-sm text-red-600">
            {t('Failed to load status: {message}', { message: error.message })}
          </div>
        )}
        
        {statuses && statuses.length === 0 && (
          <div className="text-sm text-muted-foreground">
            {t('No circuit breaker data available')}
          </div>
        )}
        
        {statuses && statuses.length > 0 && (
          <div className="space-y-3">
            {statuses.map((status) => (
              <div
                key={status.provider_id}
                className="flex items-center gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
              >
                <div className="flex items-center justify-center">
                  {getStateIcon(status.state)}
                </div>
                
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-medium text-sm">
                      {status.provider_name}
                    </span>
                    <span className={`inline-flex items-center rounded-full px-2 py-0.5 text-xs font-medium ${getStateBadgeClass(status.state)}`}>
                      {getStateLabel(status.state)}
                    </span>
                  </div>
                  
                  <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                    <span>
                      {t('Health')}: {calculateHealthScore(status)}%
                    </span>
                    <span>
                      {t('Success')}: {status.success_count}
                    </span>
                    <span>
                      {t('Failures')}: {status.failure_count}
                    </span>
                    {status.last_failure && (
                      <span>
                        {t('Last failure: {time}', {
                          time: formatTimestamp(status.last_failure),
                        })}
                      </span>
                    )}
                  </div>
                </div>
                
                {status.state === 'open' && status.opened_at && (
                  <div className="text-xs text-muted-foreground">
                    {t('Opened {time}', {
                      time: formatTimestamp(status.opened_at),
                    })}
                  </div>
                )}
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
