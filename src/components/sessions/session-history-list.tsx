import { useState, useCallback } from "react";
import { useSessionHistory } from "@/hooks/use-tauri-query";
import { SessionInfo } from "@/types";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { Button } from "@/components/ui/button";
import { Clock, Server, ChevronRight, Loader2 } from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface SessionHistoryListProps {
  onSelectSession: (session: SessionInfo) => void;
}

export function SessionHistoryList({ onSelectSession }: SessionHistoryListProps) {
  const [page, setPage] = useState(0);
  const pageSize = 50;
  const { t } = useI18n();
  
  const { data: sessions, isLoading, error } = useSessionHistory(page, pageSize);

  const formatDuration = useCallback((seconds: number): string => {
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
  }, []);

  const formatTimestamp = useCallback((timestamp: number): string => {
    const date = new Date(timestamp * 1000);
    const now = new Date();
    const diffMs = now.getTime() - date.getTime();
    const diffMins = Math.floor(diffMs / 60000);
    
    if (diffMins < 1) return t("Just now");
    if (diffMins < 60) return t("{count}m ago", { count: diffMins });
    
    const diffHours = Math.floor(diffMins / 60);
    if (diffHours < 24) return t("{count}h ago", { count: diffHours });
    
    const diffDays = Math.floor(diffHours / 24);
    if (diffDays < 7) return t("{count}d ago", { count: diffDays });
    
    return date.toLocaleDateString();
  }, [t]);

  const getStatusBadge = useCallback((session: SessionInfo) => {
    const now = Math.floor(Date.now() / 1000);
    const isActive = now - session.last_activity < 300; // 5 minutes
    
    return isActive ? (
      <Badge variant="default" className="bg-green-500">
        {t("Active")}
      </Badge>
    ) : (
      <Badge variant="secondary">{t("Expired")}</Badge>
    );
  }, [t]);

  return (
    <Card>
      <CardHeader>
            <CardTitle className="flex items-center justify-between">
              <span className="flex items-center gap-2">
                <Server className="h-5 w-5" />
                {t("Session History")}
              </span>
              <span className="text-sm font-normal text-muted-foreground">
                {sessions ? t("{count} sessions", { count: sessions.length }) : ""}
              </span>
            </CardTitle>
          </CardHeader>
          <CardContent>
            {isLoading && (
              <div className="flex items-center justify-center py-8">
                <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
              </div>
            )}

            {error && (
              <div className="text-sm text-red-600 py-4">
                {t("Failed to load sessions: {message}", { message: error.message })}
              </div>
            )}

            {sessions && sessions.length === 0 && (
              <div className="text-sm text-muted-foreground text-center py-8">
                {t("No sessions found")}
              </div>
            )}

        {sessions && sessions.length > 0 && (
          <div className="space-y-2">
            <div className="max-h-[600px] overflow-y-auto space-y-2 pr-2">
              {sessions.map((session) => (
                <button
                  key={session.id}
                  onClick={() => onSelectSession(session)}
                  className="w-full text-left p-4 rounded-lg border bg-card hover:bg-accent/50 transition-colors group"
                >
                  <div className="flex items-start justify-between gap-4">
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 mb-2">
                        <span className="font-mono text-sm font-medium truncate">
                          {session.id.substring(0, 12)}...
                        </span>
                        {getStatusBadge(session)}
                      </div>
                      
                      <div className="flex items-center gap-2 mb-1">
                        <Badge variant="outline" className="text-xs">
                          {session.provider_name}
                        </Badge>
                      </div>
                      
                      <div className="flex items-center gap-4 text-xs text-muted-foreground">
                        <span className="flex items-center gap-1">
                          <Clock className="h-3 w-3" />
                          {formatDuration(session.duration_seconds)}
                        </span>
                        <span>
                          {formatTimestamp(session.created_at)}
                        </span>
                      </div>
                    </div>
                    
                    <ChevronRight className="h-5 w-5 text-muted-foreground group-hover:text-foreground transition-colors flex-shrink-0" />
                  </div>
                </button>
              ))}
            </div>

            <div className="flex items-center justify-between pt-4 border-t">
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => Math.max(0, p - 1))}
                disabled={page === 0}
              >
                {t("Previous")}
              </Button>
              <span className="text-sm text-muted-foreground">
                {t("Page {page}", { page: page + 1 })}
              </span>
              <Button
                variant="outline"
                size="sm"
                onClick={() => setPage((p) => p + 1)}
                disabled={!sessions || sessions.length < pageSize}
              >
                {t("Next")}
              </Button>
            </div>
          </div>
        )}
      </CardContent>
    </Card>
  );
}
