import { Link, useLocation } from "react-router-dom";
import {
  LayoutDashboard,
  Server,
  Activity,
  DollarSign,
  ArrowRightLeft,
  FileText,
  Settings,
} from "lucide-react";
import { cn } from "@/lib/cn";
import { useI18n } from "@/lib/i18n";

const navigation = [
  { label: "Dashboard", href: "/", icon: LayoutDashboard },
  { label: "Providers", href: "/providers", icon: Server },
  { label: "Sessions", href: "/sessions", icon: Activity },
  { label: "Pricing", href: "/pricing", icon: DollarSign },
  { label: "Redirects", href: "/redirects", icon: ArrowRightLeft },
  { label: "Logs", href: "/logs", icon: FileText },
  { label: "Settings", href: "/settings", icon: Settings },
];

export function Sidebar() {
  const location = useLocation();
  const { t } = useI18n();

  return (
    <div className="flex h-full w-64 flex-col border-r bg-card">
      <div className="flex h-16 items-center border-b px-6">
        <h1 className="text-xl font-bold">{t("Velo Hub")}</h1>
      </div>
      <nav className="flex-1 space-y-1 p-4">
        {navigation.map((item) => {
          const isActive = location.pathname === item.href;
          return (
            <Link
              key={item.label}
              to={item.href}
              className={cn(
                "flex items-center gap-3 rounded-lg px-3 py-2 text-sm font-medium transition-colors",
                isActive
                  ? "bg-primary text-primary-foreground"
                  : "text-muted-foreground hover:bg-accent hover:text-accent-foreground"
              )}
            >
              <item.icon className="h-5 w-5" />
              {t(item.label)}
            </Link>
          );
        })}
      </nav>
    </div>
  );
}
