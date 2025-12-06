import { Provider } from "@/types";
import { Button } from "@/components/ui/button";
import { Card, CardContent, CardHeader, CardTitle } from "@/components/ui/card";
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from "@/components/ui/table";
import { Switch } from "@/components/ui/switch";
import { Badge } from "@/components/ui/badge";
import { Pencil, Trash2, Plus } from "lucide-react";
import { useI18n } from "@/lib/i18n";

interface ProviderListProps {
  providers: Provider[];
  onToggleEnabled: (id: string, enabled: boolean) => void;
  onEdit: (provider: Provider) => void;
  onDelete: (provider: Provider) => void;
  onCreate: () => void;
  isLoading?: boolean;
}

export function ProviderList({
  providers,
  onToggleEnabled,
  onEdit,
  onDelete,
  onCreate,
  isLoading = false,
}: ProviderListProps) {
  const { t } = useI18n();
  const getProviderTypeBadge = (type: string) => {
    const variants: Record<string, { label: string; variant: "default" | "secondary" | "outline" }> = {
      anthropic: { label: "Anthropic", variant: "default" },
      openai: { label: "OpenAI", variant: "secondary" },
      gemini: { label: "Gemini", variant: "outline" },
      custom: { label: "Custom", variant: "outline" },
    };
    const config = variants[type] || { label: type, variant: "outline" };
    return <Badge variant={config.variant}>{config.label}</Badge>;
  };

  if (isLoading) {
    return (
      <Card>
        <CardContent className="p-6">
          <div className="flex items-center justify-center py-8">
            <div className="text-muted-foreground">{t("Loading providers...")}</div>
          </div>
        </CardContent>
      </Card>
    );
  }

  return (
    <Card>
      <CardHeader className="flex flex-row items-center justify-between">
        <CardTitle>{t("Providers")}</CardTitle>
        <Button onClick={onCreate} size="sm">
          <Plus className="mr-2 h-4 w-4" />
          {t("Add Provider")}
        </Button>
      </CardHeader>
      <CardContent>
        {providers.length === 0 ? (
          <div className="flex flex-col items-center justify-center py-12 text-center">
            <p className="text-muted-foreground mb-4">
              {t("No providers configured yet")}
            </p>
            <Button onClick={onCreate} variant="outline">
              <Plus className="mr-2 h-4 w-4" />
              {t("Add Your First Provider")}
            </Button>
          </div>
        ) : (
          <Table>
            <TableHeader>
              <TableRow>
                <TableHead>{t("Name")}</TableHead>
                <TableHead>{t("Type")}</TableHead>
                <TableHead>{t("Endpoint")}</TableHead>
                <TableHead>{t("Priority")}</TableHead>
                <TableHead>{t("Weight")}</TableHead>
                <TableHead>{t("Status")}</TableHead>
                <TableHead className="text-right">{t("Actions")}</TableHead>
              </TableRow>
            </TableHeader>
            <TableBody>
              {providers.map((provider) => (
                <TableRow key={provider.id}>
                  <TableCell className="font-medium">{provider.name}</TableCell>
                  <TableCell>{getProviderTypeBadge(provider.provider_type)}</TableCell>
                  <TableCell className="max-w-xs truncate" title={provider.endpoint}>
                    {provider.endpoint}
                  </TableCell>
                  <TableCell>{provider.priority}</TableCell>
                  <TableCell>{provider.weight}</TableCell>
                  <TableCell>
                    <div className="flex items-center gap-2">
                      <Switch
                        checked={provider.enabled}
                        onCheckedChange={(checked) =>
                          onToggleEnabled(provider.id, checked)
                        }
                      />
                      <span className="text-sm text-muted-foreground">
                        {provider.enabled ? t("Enabled") : t("Disabled")}
                      </span>
                    </div>
                  </TableCell>
                  <TableCell className="text-right">
                    <div className="flex justify-end gap-2">
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => onEdit(provider)}
                      >
                        <Pencil className="h-4 w-4" />
                      </Button>
                      <Button
                        variant="ghost"
                        size="icon"
                        onClick={() => onDelete(provider)}
                      >
                        <Trash2 className="h-4 w-4 text-destructive" />
                      </Button>
                    </div>
                  </TableCell>
                </TableRow>
              ))}
            </TableBody>
          </Table>
        )}
      </CardContent>
    </Card>
  );
}
