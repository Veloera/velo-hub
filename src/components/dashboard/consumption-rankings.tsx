import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useConsumptionRankings } from "@/hooks/use-tauri-query";
import { TrendingUp, Zap, DollarSign } from "lucide-react";
import { useI18n } from "@/lib/i18n";

export function ConsumptionRankings() {
  const { data: rankings, isLoading, error } = useConsumptionRankings(10);
  const { t } = useI18n();

  const formatNumber = (num: number): string => {
    if (num >= 1000000) {
      return `${(num / 1000000).toFixed(2)}M`;
    }
    if (num >= 1000) {
      return `${(num / 1000).toFixed(2)}K`;
    }
    return num.toString();
  };

  const formatCost = (cost: number): string => {
    return `$${cost.toFixed(4)}`;
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <TrendingUp className="h-5 w-5" />
          {t("Top Consumers (24h)")}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading && (
          <div className="text-sm text-muted-foreground">
            {t("Loading rankings...")}
          </div>
        )}
        
        {error && (
          <div className="text-sm text-red-600">
            {t("Failed to load rankings: {message}", {
              message: error.message,
            })}
          </div>
        )}
        
        {rankings && rankings.length === 0 && (
          <div className="text-sm text-muted-foreground">
            {t("No consumption data available")}
          </div>
        )}
        
        {rankings && rankings.length > 0 && (
          <div className="space-y-3">
            {rankings.map((ranking, index) => (
              <div
                key={ranking.user_id}
                className="flex items-center gap-3 p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
              >
                <div className="flex items-center justify-center w-8 h-8 rounded-full bg-primary/10 text-primary font-bold text-sm">
                  {index + 1}
                </div>
                
                <div className="flex-1 min-w-0">
                  <div className="font-mono text-sm font-medium truncate">
                    {ranking.user_id === 'anonymous' 
                      ? t('Anonymous') 
                      : ranking.user_id.substring(0, 12) + '...'}
                  </div>
                  <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <TrendingUp className="h-3 w-3" />
                      {formatNumber(ranking.request_count)} {t("requests")}
                    </span>
                    <span className="flex items-center gap-1">
                      <Zap className="h-3 w-3" />
                      {formatNumber(ranking.total_tokens)} {t("tokens")}
                    </span>
                  </div>
                </div>
                
                <div className="flex items-center gap-1 text-sm font-medium">
                  <DollarSign className="h-4 w-4 text-muted-foreground" />
                  {formatCost(ranking.cost_usd)}
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
