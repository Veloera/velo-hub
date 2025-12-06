import { useEffect, useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ModelRedirect, Provider } from '@/types'
import { useQuery } from '@tanstack/react-query'
import {
  Dialog,
  DialogContent,
  DialogDescription,
  DialogFooter,
  DialogHeader,
  DialogTitle,
} from '@/components/ui/dialog'
import { Button } from '@/components/ui/button'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useToast } from '@/hooks/use-toast'
import { useI18n } from '@/lib/i18n'

interface RedirectDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  redirect?: ModelRedirect
  onSuccess?: () => void
}

export function RedirectDialog({
  open,
  onOpenChange,
  redirect,
  onSuccess,
}: RedirectDialogProps) {
  const { toast } = useToast()
  const [isSubmitting, setIsSubmitting] = useState(false)
  const [formData, setFormData] = useState({
    source_model: '',
    target_model: '',
    target_provider_id: '',
  })
  const { t } = useI18n()

  const { data: providers = [] } = useQuery({
    queryKey: ['providers'],
    queryFn: async () => {
      return await invoke<Provider[]>('get_providers')
    },
  })

  useEffect(() => {
    if (redirect) {
      setFormData({
        source_model: redirect.source_model,
        target_model: redirect.target_model,
        target_provider_id: redirect.target_provider_id,
      })
    } else {
      setFormData({
        source_model: '',
        target_model: '',
        target_provider_id: '',
      })
    }
  }, [redirect, open])

  const handleSubmit = async (e: React.FormEvent) => {
    e.preventDefault()
    setIsSubmitting(true)

    try {
      if (redirect) {
        // Update existing redirect
        await invoke('update_model_redirect', {
          redirect: {
            ...redirect,
            ...formData,
          },
        })
        toast({
          title: t('Success'),
          description: t('Redirect updated successfully'),
        })
      } else {
        // Create new redirect
        await invoke('create_model_redirect', {
          redirect: {
            id: crypto.randomUUID(),
            ...formData,
            enabled: true,
            created_at: Math.floor(Date.now() / 1000),
          },
        })
        toast({
          title: t('Success'),
          description: t('Redirect created successfully'),
        })
      }

      onSuccess?.()
      onOpenChange(false)
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to {action} redirect: {error}', {
          action: redirect ? t('update') : t('create'),
          error: String(error),
        }),
        variant: 'destructive',
      })
    } finally {
      setIsSubmitting(false)
    }
  }

  return (
    <Dialog open={open} onOpenChange={onOpenChange}>
      <DialogContent className="sm:max-w-[500px]">
        <DialogHeader>
          <DialogTitle>
            {redirect ? t('Edit Redirect Rule') : t('Create Redirect Rule')}
          </DialogTitle>
          <DialogDescription>
            {t('Configure a model name mapping to redirect requests to a different model')}
          </DialogDescription>
        </DialogHeader>

        <form onSubmit={handleSubmit} className="space-y-4">
          <div className="space-y-2">
            <Label htmlFor="source_model">{t('Source Model')}</Label>
            <Input
              id="source_model"
              placeholder="e.g., gpt-4"
              value={formData.source_model}
              onChange={(e) =>
                setFormData({ ...formData, source_model: e.target.value })
              }
              required
            />
            <p className="text-xs text-muted-foreground">
              {t('The model name in incoming requests to redirect')}
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="target_model">{t('Target Model')}</Label>
            <Input
              id="target_model"
              placeholder="e.g., claude-3-5-sonnet-20241022"
              value={formData.target_model}
              onChange={(e) =>
                setFormData({ ...formData, target_model: e.target.value })
              }
              required
            />
            <p className="text-xs text-muted-foreground">
              {t('The actual model name to use for the request')}
            </p>
          </div>

          <div className="space-y-2">
            <Label htmlFor="target_provider">{t('Target Provider')}</Label>
            <Select
              value={formData.target_provider_id}
              onValueChange={(value) =>
                setFormData({ ...formData, target_provider_id: value })
              }
              required
            >
              <SelectTrigger id="target_provider">
                <SelectValue placeholder={t('Select a provider')} />
              </SelectTrigger>
              <SelectContent>
                {providers
                  .filter((p: Provider) => p.enabled)
                  .map((provider: Provider) => (
                    <SelectItem key={provider.id} value={provider.id}>
                      {provider.name} ({provider.provider_type})
                    </SelectItem>
                  ))}
              </SelectContent>
            </Select>
            <p className="text-xs text-muted-foreground">
              {t('The provider that supports the target model')}
            </p>
          </div>

          <DialogFooter>
            <Button
              type="button"
              variant="outline"
              onClick={() => onOpenChange(false)}
              disabled={isSubmitting}
            >
              {t('Cancel')}
            </Button>
            <Button type="submit" disabled={isSubmitting}>
              {isSubmitting
                ? t('Saving...')
                : redirect
                  ? t('Update')
                  : t('Create')}
            </Button>
          </DialogFooter>
        </form>
      </DialogContent>
    </Dialog>
  )
}
