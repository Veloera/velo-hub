import { useState } from "react";
import { invoke } from "@tauri-apps/api/core";
import { openPath } from "@tauri-apps/plugin-opener";
import { Button } from "@/components/ui/button";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import {
  Card,
  CardContent,
  CardDescription,
  CardHeader,
  CardTitle,
} from "@/components/ui/card";
import { Badge } from "@/components/ui/badge";
import { useQuery } from "@tanstack/react-query";
import { FileText, FolderOpen, RefreshCw } from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface LogEntry {
  timestamp: string;
  level: string;
  target: string;
  message: string;
  file?: string;
  line?: number;
}

interface LogFilter {
  level?: string;
  search?: string;
  limit?: number;
}

export default function LogsPage() {
  const [filter, setFilter] = useState<LogFilter>({
    limit: 100,
  });
  const { t } = useI18n();

  const { data: logs, isLoading, refetch } = useQuery<LogEntry[]>({
    queryKey: ["logs", filter],
    queryFn: async () => {
      return await invoke<LogEntry[]>("get_logs", { filter });
    },
    refetchInterval: 5000, // Auto-refresh every 5 seconds
  });

  const { data: logDirectory } = useQuery<string>({
    queryKey: ["log-directory"],
    queryFn: async () => {
      return await invoke<string>("get_log_directory", {});
    },
  });

  const handleOpenLogDirectory = async () => {
    if (logDirectory) {
      await openPath(logDirectory);
    }
  };

  const getLevelColor = (level: string) => {
    switch (level.toUpperCase()) {
      case "ERROR":
        return "destructive";
      case "WARN":
        return "warning";
      case "INFO":
        return "default";
      case "DEBUG":
        return "secondary";
      case "TRACE":
        return "outline";
      default:
        return "default";
    }
  };

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t("Logs")}</h1>
          <p className="text-muted-foreground">
            {t("View and search application logs")}
          </p>
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={handleOpenLogDirectory}
            disabled={!logDirectory}
          >
            <FolderOpen className="mr-2 h-4 w-4" />
            {t("Open Log Directory")}
          </Button>
          <Button variant="outline" size="sm" onClick={() => refetch()}>
            <RefreshCw className="mr-2 h-4 w-4" />
            {t("Refresh")}
          </Button>
        </div>
      </div>

      <Card>
        <CardHeader>
          <CardTitle>{t("Filters")}</CardTitle>
          <CardDescription>
            {t("Filter and search log entries")}
          </CardDescription>
        </CardHeader>
        <CardContent>
          <div className="grid grid-cols-1 md:grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="level">{t("Log Level")}</Label>
              <Select
                value={filter.level || "all"}
                onValueChange={(value) =>
                  setFilter({
                    ...filter,
                    level: value === "all" ? undefined : value,
                  })
                }
              >
                <SelectTrigger id="level">
                  <SelectValue placeholder={t("All levels")} />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="all">{t("All Levels")}</SelectItem>
                  <SelectItem value="ERROR">{t("Error")}</SelectItem>
                  <SelectItem value="WARN">{t("Warning")}</SelectItem>
                  <SelectItem value="INFO">{t("Info")}</SelectItem>
                  <SelectItem value="DEBUG">{t("Debug")}</SelectItem>
                  <SelectItem value="TRACE">{t("Trace")}</SelectItem>
                </SelectContent>
              </Select>
            </div>

            <div className="space-y-2">
              <Label htmlFor="search">{t("Search")}</Label>
              <Input
                id="search"
                placeholder={t("Search in messages...")}
                value={filter.search || ""}
                onChange={(e) =>
                  setFilter({ ...filter, search: e.target.value || undefined })
                }
              />
            </div>

            <div className="space-y-2">
              <Label htmlFor="limit">{t("Limit")}</Label>
              <Select
                value={filter.limit?.toString() || "100"}
                onValueChange={(value) =>
                  setFilter({ ...filter, limit: parseInt(value) })
                }
              >
                <SelectTrigger id="limit">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="50">{t("50 entries")}</SelectItem>
                  <SelectItem value="100">{t("100 entries")}</SelectItem>
                  <SelectItem value="200">{t("200 entries")}</SelectItem>
                  <SelectItem value="500">{t("500 entries")}</SelectItem>
                </SelectContent>
              </Select>
            </div>
          </div>
        </CardContent>
      </Card>

      <Card>
        <CardHeader>
          <CardTitle className="flex items-center gap-2">
            <FileText className="h-5 w-5" />
            {t("Log Entries")}
            {logs && (
              <span className="text-sm font-normal text-muted-foreground">
                {t("({count} entries)", { count: logs.length })}
              </span>
            )}
          </CardTitle>
        </CardHeader>
        <CardContent>
          {isLoading ? (
            <div className="text-center py-8 text-muted-foreground">
              {t("Loading logs...")}
            </div>
          ) : !logs || logs.length === 0 ? (
            <div className="text-center py-8 text-muted-foreground">
              {t("No log entries found")}
            </div>
          ) : (
            <div className="space-y-2 max-h-[600px] overflow-y-auto">
              {logs.map((log, index) => (
                <div
                  key={index}
                  className="border rounded-lg p-3 hover:bg-muted/50 transition-colors font-mono text-sm"
                >
                  <div className="flex items-start gap-3">
                    <Badge variant={getLevelColor(log.level) as any}>
                      {log.level}
                    </Badge>
                    <div className="flex-1 min-w-0">
                      <div className="flex items-center gap-2 text-xs text-muted-foreground mb-1">
                        <span>{log.timestamp}</span>
                        <span>•</span>
                        <span className="truncate">{log.target}</span>
                        {log.file && log.line && (
                          <>
                            <span>•</span>
                            <span className="truncate">
                              {log.file}:{log.line}
                            </span>
                          </>
                        )}
                      </div>
                      <div className="text-foreground break-words">
                        {log.message}
                      </div>
                    </div>
                  </div>
                </div>
              ))}
            </div>
          )}
        </CardContent>
      </Card>
    </div>
  );
}
