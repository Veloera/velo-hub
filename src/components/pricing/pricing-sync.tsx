import { useState } from 'react'
import { Button } from '@/components/ui/button'
import { RefreshCw, CheckCircle2, AlertCircle } from 'lucide-react'
import { syncPricingFromLitellm } from '@/lib/tauri-api'
import { useI18n } from '@/lib/i18n'

interface PricingSyncProps {
  onSyncComplete?: (count: number) => void
}

export function PricingSync({ onSyncComplete }: PricingSyncProps) {
  const [isSyncing, setIsSyncing] = useState(false)
  const [syncStatus, setSyncStatus] = useState<'idle' | 'success' | 'error'>('idle')
  const [syncCount, setSyncCount] = useState(0)
  const [errorMessage, setErrorMessage] = useState('')
  const { t } = useI18n()

  const handleSync = async () => {
    setIsSyncing(true)
    setSyncStatus('idle')
    setErrorMessage('')

    try {
      const count = await syncPricingFromLitellm()
      setSyncCount(count)
      setSyncStatus('success')
      onSyncComplete?.(count)
    } catch (error) {
      setSyncStatus('error')
      setErrorMessage(
        error instanceof Error ? error.message : t('Failed to sync pricing data')
      )
    } finally {
      setIsSyncing(false)
      // Reset status after 3 seconds
      setTimeout(() => setSyncStatus('idle'), 3000)
    }
  }

  return (
    <div className="flex flex-col gap-2">
      <Button
        onClick={handleSync}
        disabled={isSyncing}
        variant={syncStatus === 'success' ? 'default' : syncStatus === 'error' ? 'destructive' : 'outline'}
      >
        {isSyncing ? (
          <>
            <RefreshCw className="mr-2 h-4 w-4 animate-spin" />
            {t('Syncing...')}
          </>
        ) : syncStatus === 'success' ? (
          <>
            <CheckCircle2 className="mr-2 h-4 w-4" />
            {t('Synced')}
          </>
        ) : syncStatus === 'error' ? (
          <>
            <AlertCircle className="mr-2 h-4 w-4" />
            {t('Failed')}
          </>
        ) : (
          <>
            <RefreshCw className="mr-2 h-4 w-4" />
            {t('Sync from LiteLLM')}
          </>
        )}
      </Button>
      {syncStatus === 'success' && (
        <p className="text-sm text-muted-foreground">
          {t('Successfully synced {count} model pricing records', { count: syncCount })}
        </p>
      )}
      {syncStatus === 'error' && (
        <p className="text-sm text-destructive">
          {errorMessage}
        </p>
      )}
    </div>
  )
}
