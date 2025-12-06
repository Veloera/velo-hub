import { useState, useEffect } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { Button } from '@/components/ui/button'
import { Select, SelectContent, SelectItem, SelectTrigger, SelectValue } from '@/components/ui/select'
import { Switch } from '@/components/ui/switch'
import { useToast } from '@/hooks/use-toast'
import { Download, Upload } from 'lucide-react'
import type { AppConfig } from '@/types'
import { SUPPORTED_LANGUAGES, useI18n, type Language } from '@/lib/i18n'

export default function SettingsPage() {
  const { toast } = useToast()
  const [config, setConfig] = useState<AppConfig | null>(null)
  const [loading, setLoading] = useState(true)
  const [saving, setSaving] = useState(false)
  const { t, language, setLanguage } = useI18n()

  useEffect(() => {
    loadConfig()
  }, [])

  const loadConfig = async () => {
    try {
      setLoading(true)
      const loadedConfig = await invoke<AppConfig>('get_global_config')
      setConfig(loadedConfig)
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to load configuration: {error}', {
          error: String(error),
        }),
        variant: 'destructive',
      })
    } finally {
      setLoading(false)
    }
  }

  const saveConfig = async () => {
    if (!config) return

    try {
      setSaving(true)
      await invoke('update_global_config', { config })
      toast({
        title: t('Success'),
        description: t('Configuration saved successfully'),
      })
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to save configuration: {error}', {
          error: String(error),
        }),
        variant: 'destructive',
      })
    } finally {
      setSaving(false)
    }
  }

  const handleExport = async () => {
    try {
      const configJson = await invoke<string>('export_config')
      
      // Create a download link
      const blob = new Blob([configJson], { type: 'application/json' })
      const url = URL.createObjectURL(blob)
      const a = document.createElement('a')
      a.href = url
      a.download = `velo-hub-config-${Date.now()}.json`
      document.body.appendChild(a)
      a.click()
      document.body.removeChild(a)
      URL.revokeObjectURL(url)
      
      toast({
        title: t('Success'),
        description: t('Configuration exported successfully'),
      })
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to export configuration: {error}', {
          error: String(error),
        }),
        variant: 'destructive',
      })
    }
  }

  const handleImport = async () => {
    try {
      // Create a file input
      const input = document.createElement('input')
      input.type = 'file'
      input.accept = 'application/json'
      
      input.onchange = async (e) => {
        const file = (e.target as HTMLInputElement).files?.[0]
        if (!file) return
        
        try {
          const configJson = await file.text()
          await invoke('import_config', { configJson })
          toast({
            title: t('Success'),
            description: t('Configuration imported successfully'),
          })
          // Reload config after import
          await loadConfig()
        } catch (error) {
          toast({
            title: t('Error'),
            description: t('Failed to import configuration: {error}', {
              error: String(error),
            }),
            variant: 'destructive',
          })
        }
      }
      
      input.click()
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to import configuration: {error}', {
          error: String(error),
        }),
        variant: 'destructive',
      })
    }
  }

  if (loading || !config) {
    return (
      <div className="space-y-6">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t('Settings')}</h1>
          <p className="text-muted-foreground">
            {t('Configure global settings and preferences')}
          </p>
        </div>
        <div className="flex items-center justify-center h-64">
          <p className="text-muted-foreground">{t('Loading configuration...')}</p>
        </div>
      </div>
    )
  }

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t('Settings')}</h1>
          <p className="text-muted-foreground">
            {t('Configure global settings and preferences')}
          </p>
        </div>
        <div className="flex gap-2">
          <Button variant="outline" onClick={handleImport}>
            <Upload className="mr-2 h-4 w-4" />
            {t('Import Config')}
          </Button>
          <Button variant="outline" onClick={handleExport}>
            <Download className="mr-2 h-4 w-4" />
            {t('Export Config')}
          </Button>
          <Button onClick={saveConfig} disabled={saving}>
            {saving ? t('Saving...') : t('Save Changes')}
          </Button>
        </div>
      </div>

      <div className="grid gap-6">
        <Card>
          <CardHeader>
            <CardTitle>{t('Language Preferences')}</CardTitle>
            <CardDescription>
              {t('Detect system language automatically or switch manually at any time')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-3">
            <div className="space-y-2">
              <Label htmlFor="language-select">{t('Interface Language')}</Label>
              <Select
                value={language}
                onValueChange={(value) => setLanguage(value as Language)}
              >
                <SelectTrigger id="language-select">
                  <SelectValue placeholder={t('Choose a language')} />
                </SelectTrigger>
                <SelectContent>
                  {SUPPORTED_LANGUAGES.map(({ value, labelKey }) => (
                    <SelectItem key={value} value={value}>
                      {t(labelKey)}
                    </SelectItem>
                  ))}
                </SelectContent>
              </Select>
            </div>
            <p className="text-sm text-muted-foreground">
              {t('The interface follows your system language on first launch and updates immediately when you switch here.')}
            </p>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('Server Configuration')}</CardTitle>
            <CardDescription>
              {t('Configure the HTTP server settings')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="server-host">{t('Host')}</Label>
                <Input
                  id="server-host"
                  value={config.server.host}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      server: { ...config.server, host: e.target.value },
                    })
                  }
                  placeholder="127.0.0.1"
                />
              </div>
              <div className="space-y-2">
                <Label htmlFor="server-port">{t('Port')}</Label>
                <Input
                  id="server-port"
                  type="number"
                  value={config.server.port}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      server: { ...config.server, port: parseInt(e.target.value) || 8080 },
                    })
                  }
                  placeholder="8080"
                />
              </div>
            </div>
            <div className="flex items-center space-x-2">
              <Switch
                id="enable-tls"
                checked={config.server.enable_tls}
                onCheckedChange={(checked) =>
                  setConfig({
                    ...config,
                    server: { ...config.server, enable_tls: checked },
                  })
                }
              />
              <Label htmlFor="enable-tls">{t('Enable TLS/HTTPS')}</Label>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('Circuit Breaker Configuration')}</CardTitle>
            <CardDescription>
              {t('Configure circuit breaker parameters for provider fault tolerance')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="failure-threshold">
                {t('Failure Threshold (0.0 - 1.0)')}
              </Label>
              <Input
                id="failure-threshold"
                type="number"
                step="0.01"
                min="0"
                max="1"
                value={config.circuit_breaker.failure_threshold}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    circuit_breaker: {
                      ...config.circuit_breaker,
                      failure_threshold: parseFloat(e.target.value) || 0.5,
                    },
                  })
                }
              />
              <p className="text-sm text-muted-foreground">
                {t('Error rate threshold to trigger circuit breaker (default: 0.5 = 50%)')}
              </p>
            </div>
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="window-duration">{t('Window Duration (seconds)')}</Label>
                <Input
                  id="window-duration"
                  type="number"
                  value={config.circuit_breaker.window_duration_secs}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      circuit_breaker: {
                        ...config.circuit_breaker,
                        window_duration_secs: parseInt(e.target.value) || 60,
                      },
                    })
                  }
                />
                <p className="text-sm text-muted-foreground">
                  {t('Time window for error rate calculation')}
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="cooldown-duration">{t('Cooldown Duration (seconds)')}</Label>
                <Input
                  id="cooldown-duration"
                  type="number"
                  value={config.circuit_breaker.cooldown_duration_secs}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      circuit_breaker: {
                        ...config.circuit_breaker,
                        cooldown_duration_secs: parseInt(e.target.value) || 30,
                      },
                    })
                  }
                />
                <p className="text-sm text-muted-foreground">
                  {t('Wait time before attempting recovery')}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('Session Configuration')}</CardTitle>
            <CardDescription>
              {t('Configure session management and cleanup settings')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="grid grid-cols-2 gap-4">
              <div className="space-y-2">
                <Label htmlFor="session-ttl">{t('Session TTL (minutes)')}</Label>
                <Input
                  id="session-ttl"
                  type="number"
                  value={config.session.ttl_minutes}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      session: {
                        ...config.session,
                        ttl_minutes: parseInt(e.target.value) || 5,
                      },
                    })
                  }
                />
                <p className="text-sm text-muted-foreground">
                  {t('Time before inactive sessions expire')}
                </p>
              </div>
              <div className="space-y-2">
                <Label htmlFor="cleanup-interval">{t('Cleanup Interval (seconds)')}</Label>
                <Input
                  id="cleanup-interval"
                  type="number"
                  value={config.session.cleanup_interval_secs}
                  onChange={(e) =>
                    setConfig({
                      ...config,
                      session: {
                        ...config.session,
                        cleanup_interval_secs: parseInt(e.target.value) || 60,
                      },
                    })
                  }
                />
                <p className="text-sm text-muted-foreground">
                  {t('How often to run session cleanup')}
                </p>
              </div>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('Logging Configuration')}</CardTitle>
            <CardDescription>
              {t('Configure application logging settings')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="log-level">{t('Log Level')}</Label>
              <Select
                value={config.logging.level}
                onValueChange={(value) =>
                  setConfig({
                    ...config,
                    logging: { ...config.logging, level: value },
                  })
                }
              >
                <SelectTrigger id="log-level">
                  <SelectValue />
                </SelectTrigger>
                <SelectContent>
                  <SelectItem value="trace">{t('Trace')}</SelectItem>
                  <SelectItem value="debug">{t('Debug')}</SelectItem>
                  <SelectItem value="info">{t('Info')}</SelectItem>
                  <SelectItem value="warn">{t('Warn')}</SelectItem>
                  <SelectItem value="error">{t('Error')}</SelectItem>
                </SelectContent>
              </Select>
              <p className="text-sm text-muted-foreground">
                {t('Minimum log level to record')}
              </p>
            </div>
            <div className="space-y-2">
              <Label htmlFor="log-file">{t('Log File Path (optional)')}</Label>
              <Input
                id="log-file"
                value={config.logging.file || ''}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    logging: {
                      ...config.logging,
                      file: e.target.value || undefined,
                    },
                  })
                }
                placeholder="~/.velo-hub/logs/app.log"
              />
              <p className="text-sm text-muted-foreground">
                {t('Leave empty to disable file logging')}
              </p>
            </div>
          </CardContent>
        </Card>

        <Card>
          <CardHeader>
            <CardTitle>{t('Database Configuration')}</CardTitle>
            <CardDescription>
              {t('Configure database storage location')}
            </CardDescription>
          </CardHeader>
          <CardContent className="space-y-4">
            <div className="space-y-2">
              <Label htmlFor="db-path">{t('Database Path')}</Label>
              <Input
                id="db-path"
                value={config.database.path}
                onChange={(e) =>
                  setConfig({
                    ...config,
                    database: { ...config.database, path: e.target.value },
                  })
                }
                placeholder="~/.velo-hub/data.db"
              />
              <p className="text-sm text-muted-foreground">
                {t('Path to SQLite database file')}
              </p>
            </div>
          </CardContent>
        </Card>
      </div>
    </div>
  )
}
