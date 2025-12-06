import { useState } from "react";
import { SessionInfo } from "@/types";
import { SessionHistoryList } from "@/components/sessions/session-history-list";
import { SessionDetailsDrawer } from "@/components/sessions/session-details-drawer";
import { useI18n } from "@/lib/i18n";

export default function SessionsPage() {
  const [selectedSession, setSelectedSession] = useState<SessionInfo | null>(null);
  const [drawerOpen, setDrawerOpen] = useState(false);
  const { t } = useI18n();

  const handleSelectSession = (session: SessionInfo) => {
    setSelectedSession(session);
    setDrawerOpen(true);
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("Sessions")}</h1>
        <p className="text-muted-foreground">
          {t("View active and historical session information")}
        </p>
      </div>

      <SessionHistoryList onSelectSession={handleSelectSession} />

      <SessionDetailsDrawer
        session={selectedSession}
        open={drawerOpen}
        onOpenChange={setDrawerOpen}
      />
    </div>
  );
}
