import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { useActiveSessions } from "@/hooks/use-tauri-query";
import { Clock, Server } from "lucide-react";
import { useI18n } from "@/lib/i18n";

export function ActiveSessionsList() {
  const { data: sessions, isLoading, error } = useActiveSessions();
  const { t } = useI18n();

  const formatDuration = (seconds: number): string => {
    if (seconds < 60) {
      return `${seconds}s`;
    }
    const minutes = Math.floor(seconds / 60);
    const remainingSeconds = seconds % 60;
    if (minutes < 60) {
      return `${minutes}m ${remainingSeconds}s`;
    }
    const hours = Math.floor(minutes / 60);
    const remainingMinutes = minutes % 60;
    return `${hours}h ${remainingMinutes}m`;
  };

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleTimeString();
  };

  return (
    <Card>
      <CardHeader>
        <CardTitle className="flex items-center gap-2">
          <Server className="h-5 w-5" />
          {t("Active Sessions")}
        </CardTitle>
      </CardHeader>
      <CardContent>
        {isLoading && (
          <div className="text-sm text-muted-foreground">
            {t("Loading sessions...")}
          </div>
        )}
        
        {error && (
          <div className="text-sm text-red-600">
            {t("Failed to load sessions: {message}", {
              message: error.message,
            })}
          </div>
        )}
        
        {sessions && sessions.length === 0 && (
          <div className="text-sm text-muted-foreground">
            {t("No active sessions")}
          </div>
        )}
        
        {sessions && sessions.length > 0 && (
          <div className="space-y-3">
            {sessions.map((session) => (
              <div
                key={session.id}
                className="flex items-center justify-between p-3 rounded-lg border bg-card hover:bg-accent/50 transition-colors"
              >
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-mono text-sm font-medium truncate">
                      {session.id.substring(0, 8)}...
                    </span>
                    <span className="inline-flex items-center rounded-full bg-primary/10 px-2 py-0.5 text-xs font-medium text-primary">
                      {session.provider_name}
                    </span>
                  </div>
                  <div className="flex items-center gap-3 mt-1 text-xs text-muted-foreground">
                    <span className="flex items-center gap-1">
                      <Clock className="h-3 w-3" />
                      {formatDuration(session.duration_seconds)}
                    </span>
                    <span>
                      {t("Started: {time}", {
                        time: formatTimestamp(session.created_at),
                      })}
                    </span>
                  </div>
                </div>
                <div className="flex items-center">
                  <div className="h-2 w-2 rounded-full bg-green-500 animate-pulse" />
                </div>
              </div>
            ))}
          </div>
        )}
      </CardContent>
    </Card>
  );
}
