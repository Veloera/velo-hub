import { BrowserRouter, Routes, Route } from "react-router-dom";
import { QueryClientProvider } from "@tanstack/react-query";
import { ThemeProvider } from "@/hooks/use-theme";
import { ToastProvider } from "@/components/ui/toast";
import { queryClient } from "@/lib/query-client";
import { MainLayout } from "@/components/layout/main-layout";
import DashboardPage from "@/pages/dashboard";
import ProvidersPage from "@/pages/providers";
import SessionsPage from "@/pages/sessions";
import PricingPage from "@/pages/pricing";
import RedirectsPage from "@/pages/redirects";
import SettingsPage from "@/pages/settings";
import LogsPage from "@/pages/logs";

function App() {
  return (
    <ThemeProvider defaultTheme="system" storageKey="velo-hub-theme">
      <QueryClientProvider client={queryClient}>
        <ToastProvider>
          <BrowserRouter>
            <Routes>
              <Route path="/" element={<MainLayout />}>
                <Route index element={<DashboardPage />} />
                <Route path="providers" element={<ProvidersPage />} />
                <Route path="sessions" element={<SessionsPage />} />
                <Route path="pricing" element={<PricingPage />} />
                <Route path="redirects" element={<RedirectsPage />} />
                <Route path="logs" element={<LogsPage />} />
                <Route path="settings" element={<SettingsPage />} />
              </Route>
            </Routes>
          </BrowserRouter>
        </ToastProvider>
      </QueryClientProvider>
    </ThemeProvider>
  );
}

export default App;
