import { useSessionDecisionChain } from "@/hooks/use-tauri-query";
import { SessionInfo, DecisionRecord } from "@/types";
import {
  Sheet,
  SheetContent,
  SheetDescription,
  SheetHeader,
  SheetTitle,
} from "@/components/ui/sheet";
import { Badge } from "@/components/ui/badge";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Clock,
  Server,
  Activity,
  AlertCircle,
  CheckCircle,
  XCircle,
  RefreshCw,
  GitBranch,
  ShieldAlert,
  Timer,
  Loader2,
} from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface SessionDetailsDrawerProps {
  session: SessionInfo | null;
  open: boolean;
  onOpenChange: (open: boolean) => void;
}

export function SessionDetailsDrawer({
  session,
  open,
  onOpenChange,
}: SessionDetailsDrawerProps) {
  const { data: decisionChain, isLoading, error } = useSessionDecisionChain(
    session?.id || null
  );
  const { t } = useI18n();

  const formatTimestamp = (timestamp: number): string => {
    const date = new Date(timestamp * 1000);
    return date.toLocaleString();
  };

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

  const getActionIcon = (action: DecisionRecord["action"]) => {
    switch (action) {
      case "selected":
        return <CheckCircle className="h-4 w-4 text-green-500" />;
      case "failed":
        return <XCircle className="h-4 w-4 text-red-500" />;
      case "retried":
        return <RefreshCw className="h-4 w-4 text-yellow-500" />;
      case "redirected":
        return <GitBranch className="h-4 w-4 text-blue-500" />;
      case "circuit_breaker_open":
        return <ShieldAlert className="h-4 w-4 text-orange-500" />;
      case "rate_limited":
        return <Timer className="h-4 w-4 text-purple-500" />;
      default:
        return <Activity className="h-4 w-4 text-gray-500" />;
    }
  };

  const getActionBadgeVariant = (
    action: DecisionRecord["action"]
  ): "default" | "secondary" | "destructive" | "outline" => {
    switch (action) {
      case "selected":
        return "default";
      case "failed":
        return "destructive";
      case "retried":
      case "redirected":
        return "secondary";
      case "circuit_breaker_open":
      case "rate_limited":
        return "outline";
      default:
        return "secondary";
    }
  };

  const formatActionFallback = (value: string) =>
    value
      .split("_")
      .map((word) => word.charAt(0).toUpperCase() + word.slice(1))
      .join(" ");

  const formatActionLabel = (action: DecisionRecord["action"]): string => {
    switch (action) {
      case "selected":
        return t("Selected");
      case "failed":
        return t("Failed");
      case "retried":
        return t("Retried");
      case "redirected":
        return t("Redirected");
      case "circuit_breaker_open":
        return t("Circuit breaker opened");
      case "rate_limited":
        return t("Rate limited");
      default:
        return formatActionFallback(action);
    }
  };

  if (!session) {
    return null;
  }

  return (
    <Sheet open={open} onOpenChange={onOpenChange}>
      <SheetContent className="w-full sm:max-w-2xl overflow-y-auto">
        <SheetHeader>
          <SheetTitle>{t("Session Details")}</SheetTitle>
          <SheetDescription>
            {t("View session information and decision chain")}
          </SheetDescription>
        </SheetHeader>

        <div className="mt-6 space-y-6">
          {/* Session Info Card */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Server className="h-4 w-4" />
                {t("Session Information")}
              </CardTitle>
            </CardHeader>
            <CardContent className="space-y-3">
              <div className="grid grid-cols-2 gap-4">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">
                    {t("Session ID")}
                  </div>
                  <div className="font-mono text-sm break-all">
                    {session.id}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">
                    {t("Provider")}
                  </div>
                  <Badge variant="outline">{session.provider_name}</Badge>
                </div>
              </div>

              <div className="grid grid-cols-2 gap-4">
                <div>
                  <div className="text-xs text-muted-foreground mb-1">
                    {t("Created At")}
                  </div>
                  <div className="text-sm">
                    {formatTimestamp(session.created_at)}
                  </div>
                </div>
                <div>
                  <div className="text-xs text-muted-foreground mb-1">
                    {t("Last Activity")}
                  </div>
                  <div className="text-sm">
                    {formatTimestamp(session.last_activity)}
                  </div>
                </div>
              </div>

              <div>
                <div className="text-xs text-muted-foreground mb-1">
                  {t("Duration")}
                </div>
                <div className="flex items-center gap-2 text-sm">
                  <Clock className="h-4 w-4" />
                  {formatDuration(session.duration_seconds)}
                </div>
              </div>
            </CardContent>
          </Card>

          {/* Decision Chain Card */}
          <Card>
            <CardHeader>
              <CardTitle className="text-base flex items-center gap-2">
                <Activity className="h-4 w-4" />
                {t("Decision Chain")}
              </CardTitle>
            </CardHeader>
            <CardContent>
              {isLoading && (
                <div className="flex items-center justify-center py-8">
                  <Loader2 className="h-6 w-6 animate-spin text-muted-foreground" />
                </div>
              )}

              {error && (
                <div className="flex items-center gap-2 text-sm text-red-600 py-4">
                  <AlertCircle className="h-4 w-4" />
                  {t("Failed to load decision chain: {message}", {
                    message: error.message,
                  })}
                </div>
              )}

              {decisionChain && decisionChain.length === 0 && (
                <div className="text-sm text-muted-foreground text-center py-8">
                  {t("No decision records found for this session")}
                </div>
              )}

              {decisionChain && decisionChain.length > 0 && (
                <div className="space-y-3">
                  {decisionChain.map((decision, index) => (
                    <div
                      key={`${decision.step_number}-${index}`}
                      className="relative pl-8 pb-4 last:pb-0"
                    >
                      {/* Timeline line */}
                      {index < decisionChain.length - 1 && (
                        <div className="absolute left-[11px] top-6 bottom-0 w-0.5 bg-border" />
                      )}

                      {/* Timeline dot */}
                      <div className="absolute left-0 top-1 flex items-center justify-center">
                        {getActionIcon(decision.action)}
                      </div>

                      {/* Decision content */}
                      <div className="space-y-2">
                        <div className="flex items-center gap-2">
                          <Badge
                            variant={getActionBadgeVariant(decision.action)}
                            className="text-xs"
                          >
                            {t("Step {number}", {
                              number: decision.step_number,
                            })}
                          </Badge>
                          <Badge variant="outline" className="text-xs">
                            {formatActionLabel(decision.action)}
                          </Badge>
                          {decision.provider_id && (
                            <span className="text-xs text-muted-foreground">
                              {t("Provider: {id}", {
                                id: decision.provider_id.substring(0, 8),
                              })}
                            </span>
                          )}
                        </div>

                        <div className="text-sm text-muted-foreground">
                          {decision.reason}
                        </div>

                        <div className="text-xs text-muted-foreground">
                          {formatTimestamp(decision.timestamp)}
                        </div>
                      </div>
                    </div>
                  ))}
                </div>
              )}
            </CardContent>
          </Card>
        </div>
      </SheetContent>
    </Sheet>
  );
}
