import { useState } from 'react'
import { useQuery } from '@tanstack/react-query'
import { invoke } from '@tauri-apps/api/core'
import { ModelRedirect, Provider } from '@/types'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Badge } from '@/components/ui/badge'
import { Button } from '@/components/ui/button'
import { Switch } from '@/components/ui/switch'
import { Edit, Trash2 } from 'lucide-react'
import { RedirectDialog } from './redirect-dialog'
import { DeleteRedirectDialog } from './delete-redirect-dialog'
import { useToast } from '@/hooks/use-toast'
import { useI18n } from '@/lib/i18n'

export function RedirectList() {
  const { toast } = useToast()
  const [editingRedirect, setEditingRedirect] = useState<ModelRedirect | null>(null)
  const [deletingRedirect, setDeletingRedirect] = useState<ModelRedirect | null>(null)
  const { t } = useI18n()

  const { data: redirects = [], isLoading, refetch } = useQuery({
    queryKey: ['model-redirects'],
    queryFn: async () => {
      return await invoke<ModelRedirect[]>('get_model_redirects')
    },
  })

  const { data: providers = [] } = useQuery({
    queryKey: ['providers'],
    queryFn: async () => {
      return await invoke<Provider[]>('get_providers')
    },
  })

  const getProviderName = (providerId: string) => {
    const provider = providers.find((p) => p.id === providerId)
    return provider?.name || 'Unknown'
  }

  const handleToggle = async (redirect: ModelRedirect) => {
    try {
      await invoke('toggle_model_redirect', {
        id: redirect.id,
        enabled: !redirect.enabled,
      })
      refetch()
      toast({
        title: t('Success'),
        description: redirect.enabled
          ? t('Redirect disabled successfully')
          : t('Redirect enabled successfully'),
      })
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to toggle redirect: {error}', {
          error: String(error),
        }),
        variant: 'destructive',
      })
    }
  }

  if (isLoading) {
    return (
      <div className="flex items-center justify-center py-8">
        <p className="text-muted-foreground">{t('Loading redirects...')}</p>
      </div>
    )
  }

  if (redirects.length === 0) {
    return (
      <div className="flex flex-col items-center justify-center py-12 text-center">
        <p className="text-muted-foreground mb-2">{t('No redirect rules configured')}</p>
        <p className="text-sm text-muted-foreground">
          {t('Create a redirect rule to map model names to different models')}
        </p>
      </div>
    )
  }

  return (
    <>
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t('Source Model')}</TableHead>
              <TableHead>{t('Target Model')}</TableHead>
              <TableHead>{t('Target Provider')}</TableHead>
              <TableHead>{t('Status')}</TableHead>
              <TableHead>{t('Created')}</TableHead>
              <TableHead className="text-right">{t('Actions')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {redirects.map((redirect) => (
              <TableRow key={redirect.id}>
                <TableCell className="font-medium">{redirect.source_model}</TableCell>
                <TableCell>{redirect.target_model}</TableCell>
                <TableCell>{getProviderName(redirect.target_provider_id)}</TableCell>
                <TableCell>
                  <Badge variant={redirect.enabled ? 'default' : 'secondary'}>
                    {redirect.enabled ? t('Enabled') : t('Disabled')}
                  </Badge>
                </TableCell>
                <TableCell>
                  {new Date(redirect.created_at * 1000).toLocaleDateString()}
                </TableCell>
                <TableCell className="text-right">
                  <div className="flex items-center justify-end gap-2">
                    <Switch
                      checked={redirect.enabled}
                      onCheckedChange={() => handleToggle(redirect)}
                    />
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setEditingRedirect(redirect)}
                    >
                      <Edit className="h-4 w-4" />
                    </Button>
                    <Button
                      variant="ghost"
                      size="icon"
                      onClick={() => setDeletingRedirect(redirect)}
                    >
                      <Trash2 className="h-4 w-4" />
                    </Button>
                  </div>
                </TableCell>
              </TableRow>
            ))}
          </TableBody>
        </Table>
      </div>

      {editingRedirect && (
        <RedirectDialog
          open={!!editingRedirect}
          onOpenChange={(open: boolean) => !open && setEditingRedirect(null)}
          redirect={editingRedirect}
          onSuccess={() => {
            setEditingRedirect(null)
            refetch()
          }}
        />
      )}

      {deletingRedirect && (
        <DeleteRedirectDialog
          open={!!deletingRedirect}
          onOpenChange={(open) => !open && setDeletingRedirect(null)}
          redirect={deletingRedirect}
          onSuccess={() => {
            setDeletingRedirect(null)
            refetch()
          }}
        />
      )}
    </>
  )
}
