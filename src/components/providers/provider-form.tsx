import { useEffect, useState } from "react";
import { z } from "zod";
import { Provider, ProviderType, ProxyType } from "@/types";
import { Button } from "@/components/ui/button";
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from "@/components/ui/dialog";
import { Input } from "@/components/ui/input";
import { Label } from "@/components/ui/label";
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from "@/components/ui/select";
import { Switch } from "@/components/ui/switch";
import { useI18n } from "@/lib/i18n";

const providerSchema = z.object({
  name: z.string().min(1, "Name is required").max(100, "Name too long"),
  provider_type: z.enum(["anthropic", "openai", "gemini", "custom"]),
  endpoint: z.string().url("Invalid URL"),
  api_key: z.string().min(10, "API key must be at least 10 characters"),
  priority: z.number().int().min(0).max(100),
  weight: z.number().int().min(1).max(100),
  enabled: z.boolean(),
  proxy_enabled: z.boolean(),
  proxy_type: z.enum(["http", "https", "socks5"]).optional(),
  proxy_url: z.string().url("Invalid proxy URL").optional().or(z.literal("")),
  proxy_username: z.string().optional(),
  proxy_password: z.string().optional(),
});

type ProviderFormData = z.infer<typeof providerSchema>;

interface ProviderFormProps {
  open: boolean;
  onOpenChange: (open: boolean) => void;
  provider?: Provider | null;
  onSubmit: (provider: Partial<Provider>) => Promise<void>;
  onTest?: (provider: Provider) => Promise<boolean>;
}

export function ProviderForm({
  open,
  onOpenChange,
  provider,
  onSubmit,
  onTest,
}: ProviderFormProps) {
  const { t } = useI18n();
  const [formData, setFormData] = useState<ProviderFormData>({
    name: "",
    provider_type: "openai",
    endpoint: "",
    api_key: "",
    priority: 0,
    weight: 1,
    enabled: true,
    proxy_enabled: false,
    proxy_type: "http",
    proxy_url: "",
    proxy_username: "",
    proxy_password: "",
  });

  const [errors, setErrors] = useState<Record<string, string>>({});
  const [isSubmitting, setIsSubmitting] = useState(false);
  const [isTesting, setIsTesting] = useState(false);
  const [testResult, setTestResult] = useState<{
    success: boolean;
    message: string;
  } | null>(null);

  useEffect(() => {
    if (provider) {
      setFormData({
        name: provider.name,
        provider_type: provider.provider_type,
        endpoint: provider.endpoint,
        api_key: provider.api_key,
        priority: provider.priority,
        weight: provider.weight,
        enabled: provider.enabled,
        proxy_enabled: !!provider.proxy,
        proxy_type: provider.proxy?.proxy_type || "http",
        proxy_url: provider.proxy?.url || "",
        proxy_username: provider.proxy?.auth?.username || "",
        proxy_password: provider.proxy?.auth?.password || "",
      });
    } else {
      setFormData({
        name: "",
        provider_type: "openai",
        endpoint: "",
        api_key: "",
        priority: 0,
        weight: 1,
        enabled: true,
        proxy_enabled: false,
        proxy_type: "http",
        proxy_url: "",
        proxy_username: "",
        proxy_password: "",
      });
    }
    setErrors({});
    setTestResult(null);
  }, [provider, open]);

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault();
    setErrors({});
    setTestResult(null);

    try {
      const validated = providerSchema.parse(formData);

      const providerData: Partial<Provider> = {
        ...(provider?.id && { id: provider.id }),
        name: validated.name,
        provider_type: validated.provider_type,
        endpoint: validated.endpoint,
        api_key: validated.api_key,
        priority: validated.priority,
        weight: validated.weight,
        enabled: validated.enabled,
      };

      if (validated.proxy_enabled && validated.proxy_url) {
        providerData.proxy = {
          proxy_type: validated.proxy_type!,
          url: validated.proxy_url,
          ...(validated.proxy_username &&
            validated.proxy_password && {
              auth: {
                username: validated.proxy_username,
                password: validated.proxy_password,
              },
            }),
        };
      }

      setIsSubmitting(true);
      await onSubmit(providerData);
      onOpenChange(false);
    } catch (error) {
      if (error instanceof z.ZodError) {
        const fieldErrors: Record<string, string> = {};
        error.errors.forEach((err) => {
          if (err.path[0]) {
            fieldErrors[err.path[0].toString()] = err.message;
          }
        });
        setErrors(fieldErrors);
      }
    } finally {
      setIsSubmitting(false);
    }
  };

  const handleTest = async () => {
    setErrors({});
    setTestResult(null);

    try {
      const validated = providerSchema.parse(formData);

      const providerData: Partial<Provider> = {
        ...(provider?.id && { id: provider.id }),
        name: validated.name,
        provider_type: validated.provider_type,
        endpoint: validated.endpoint,
        api_key: validated.api_key,
        priority: validated.priority,
        weight: validated.weight,
        enabled: validated.enabled,
      };

      if (validated.proxy_enabled && validated.proxy_url) {
        providerData.proxy = {
          proxy_type: validated.proxy_type!,
          url: validated.proxy_url,
          ...(validated.proxy_username &&
            validated.proxy_password && {
              auth: {
                username: validated.proxy_username,
                password: validated.proxy_password,
              },
            }),
        };
      }

      if (onTest) {
        setIsTesting(true);
        const now = Date.now();
        const providerPayload: Provider = {
          id: providerData.id ?? crypto.randomUUID(),
          name: providerData.name!,
          provider_type: providerData.provider_type!,
          endpoint: providerData.endpoint!,
          api_key: providerData.api_key!,
          priority: providerData.priority ?? 0,
          weight: providerData.weight ?? 1,
          enabled: providerData.enabled ?? true,
          proxy: providerData.proxy,
          created_at: provider?.created_at ?? now,
          updated_at: provider?.updated_at ?? now,
        };
        const success = await onTest(providerPayload);
        setTestResult({
          success,
          message: success
            ? t("Connection successful!")
            : t("Connection failed. Please check your configuration."),
        });
      }
    } catch (error) {
      if (error instanceof z.ZodError) {
        const fieldErrors: Record<string, string> = {};
        error.errors.forEach((err) => {
          if (err.path[0]) {
            fieldErrors[err.path[0].toString()] = err.message;
          }
        });
        setErrors(fieldErrors);
      }
    } finally {
      setIsTesting(false);
    }
  };

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="max-w-2xl max-h-[90vh] overflow-y-auto">
        <DialogHeader>
          <DialogTitle>
            {provider ? t("Edit Provider") : t("Add New Provider")}
          </DialogTitle>
          <DialogDescription>
            {t("Configure your LLM provider settings. All fields are required unless marked optional.")}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="grid grid-cols-2 gap-4">
            <div className="space-y-2">
              <Label htmlFor="name">{t("Name")} *</Label>
              <Input
                id="name"
                value={formData.name}
                onChange={(e) =>
                  setFormData({ ...formData, name: e.target.value })
                }
                placeholder={t("My Provider")}
              />
              {errors.name && (
                <p className="text-sm text-destructive">{errors.name}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="provider_type">{t("Provider Type")} *</Label>
              <Select
                value={formData.provider_type}
                onValueChange={(value) =>
                  setFormData({
                    ...formData,
                    provider_type: value as ProviderType,
                  })
                }
              >
                <SelectTrigger id="provider_type">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="anthropic">Anthropic</SelectItem>
                  <SelectItem value="openai">OpenAI</SelectItem>
                  <SelectItem value="gemini">Gemini</SelectItem>
                  <SelectItem value="custom">Custom</SelectItem>
                </SelectContent>
              </Select>
              {errors.provider_type && (
                <p className="text-sm text-destructive">
                  {errors.provider_type}
                </p>
              )}
            </div>
          </div>

          <div className="space-y-2">
                <Label htmlFor="endpoint">{t("Endpoint URL")} *</Label>
            <Input
              id="endpoint"
              value={formData.endpoint}
              onChange={(e) =>
                setFormData({ ...formData, endpoint: e.target.value })
              }
              placeholder="https://api.openai.com/v1"
            />
            {errors.endpoint && (
              <p className="text-sm text-destructive">{errors.endpoint}</p>
            )}
          </div>

          <div className="space-y-2">
                <Label htmlFor="api_key">{t("API Key")} *</Label>
            <Input
              id="api_key"
              type="password"
              value={formData.api_key}
              onChange={(e) =>
                setFormData({ ...formData, api_key: e.target.value })
              }
              placeholder="sk-..."
            />
            {errors.api_key && (
              <p className="text-sm text-destructive">{errors.api_key}</p>
            )}
          </div>

          <div className="grid grid-cols-3 gap-4">
            <div className="space-y-2">
              <Label htmlFor="priority">{t("Priority")}</Label>
              <Input
                id="priority"
                type="number"
                min="0"
                max="100"
                value={formData.priority}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    priority: parseInt(e.target.value) || 0,
                  })
                }
              />
              {errors.priority && (
                <p className="text-sm text-destructive">{errors.priority}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="weight">{t("Weight")}</Label>
              <Input
                id="weight"
                type="number"
                min="1"
                max="100"
                value={formData.weight}
                onChange={(e) =>
                  setFormData({
                    ...formData,
                    weight: parseInt(e.target.value) || 1,
                  })
                }
              />
              {errors.weight && (
                <p className="text-sm text-destructive">{errors.weight}</p>
              )}
            </div>

            <div className="space-y-2">
              <Label htmlFor="enabled">{t("Enabled")}</Label>
              <div className="flex items-center h-10">
                <Switch
                  id="enabled"
                  checked={formData.enabled}
                  onCheckedChange={(checked) =>
                    setFormData({ ...formData, enabled: checked })
                  }
                />
              </div>
            </div>
          </div>

          <div className="border-t pt-4">
            <div className="flex items-center justify-between mb-4">
              <Label htmlFor="proxy_enabled">{t("Proxy Configuration")}</Label>
              <Switch
                id="proxy_enabled"
                checked={formData.proxy_enabled}
                onCheckedChange={(checked) =>
                  setFormData({ ...formData, proxy_enabled: checked })
                }
              />
            </div>

            {formData.proxy_enabled && (
              <div className="space-y-4 pl-4 border-l-2">
                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="proxy_type">{t("Proxy Type")}</Label>
                    <Select
                      value={formData.proxy_type}
                      onValueChange={(value) =>
                        setFormData({
                          ...formData,
                          proxy_type: value as ProxyType,
                        })
                      }
                    >
                      <SelectTrigger id="proxy_type">
                        <SelectValue />
                      </SelectTrigger>
                      <SelectContent>
                        <SelectItem value="http">HTTP</SelectItem>
                        <SelectItem value="https">HTTPS</SelectItem>
                        <SelectItem value="socks5">SOCKS5</SelectItem>
                      </SelectContent>
                    </Select>
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="proxy_url">{t("Proxy URL")}</Label>
                    <Input
                      id="proxy_url"
                      value={formData.proxy_url}
                      onChange={(e) =>
                        setFormData({ ...formData, proxy_url: e.target.value })
                      }
                      placeholder="http://proxy.example.com:8080"
                    />
                    {errors.proxy_url && (
                      <p className="text-sm text-destructive">
                        {errors.proxy_url}
                      </p>
                    )}
                  </div>
                </div>

                <div className="grid grid-cols-2 gap-4">
                  <div className="space-y-2">
                    <Label htmlFor="proxy_username">{t("Username (Optional)")}</Label>
                    <Input
                      id="proxy_username"
                      value={formData.proxy_username}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          proxy_username: e.target.value,
                        })
                      }
                      placeholder="username"
                    />
                  </div>

                  <div className="space-y-2">
                    <Label htmlFor="proxy_password">{t("Password (Optional)")}</Label>
                    <Input
                      id="proxy_password"
                      type="password"
                      value={formData.proxy_password}
                      onChange={(e) =>
                        setFormData({
                          ...formData,
                          proxy_password: e.target.value,
                        })
                      }
                      placeholder="password"
                    />
                  </div>
                </div>
              </div>
            )}
          </div>

          {testResult && (
            <div
              className={`p-3 rounded-md ${
                testResult.success
                  ? "bg-green-50 text-green-800 dark:bg-green-900/20 dark:text-green-400"
                  : "bg-red-50 text-red-800 dark:bg-red-900/20 dark:text-red-400"
              }`}
            >
              {testResult.message}
            </div>
          )}

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isSubmitting || isTesting}
            >
              {t("Cancel")}
            </Button>
            {onTest && (
              <Button
                type="button"
                variant="secondary"
                onClick={handleTest}
                disabled={isSubmitting || isTesting}
              >
                {isTesting ? t("Testing...") : t("Test Connection")}
              </Button>
            )}
            <Button type="submit" disabled={isSubmitting || isTesting}>
              {isSubmitting
                ? t("Saving...")
                : provider
                  ? t("Update")
                  : t("Create")}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  );
}
