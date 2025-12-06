import { useState } from 'react'
import { invoke } from '@tauri-apps/api/core'
import { ModelRedirect } from '@/types'
import {
  AlertDialog,
  AlertDialogAction,
  AlertDialogCancel,
  AlertDialogContent,
  AlertDialogDescription,
  AlertDialogFooter,
  AlertDialogHeader,
  AlertDialogTitle,
} from '@/components/ui/alert-dialog'
import { useToast } from '@/hooks/use-toast'
import { useI18n } from '@/lib/i18n'

interface DeleteRedirectDialogProps {
  open: boolean
  onOpenChange: (open: boolean) => void
  redirect: ModelRedirect
  onSuccess?: () => void
}

export function DeleteRedirectDialog({
  open,
  onOpenChange,
  redirect,
  onSuccess,
}: DeleteRedirectDialogProps) {
  const { toast } = useToast()
  const [isDeleting, setIsDeleting] = useState(false)
  const { t } = useI18n()

  const handleDelete = async () => {
    setIsDeleting(true)
    try {
      await invoke('delete_model_redirect', { id: redirect.id })
      toast({
        title: t('Success'),
        description: t('Redirect deleted successfully'),
      })
      onSuccess?.()
      onOpenChange(false)
    } catch (error) {
      toast({
        title: t('Error'),
        description: t('Failed to delete redirect: {error}', {
          error: String(error),
        }),
        variant: 'destructive',
      })
    } finally {
      setIsDeleting(false)
    }
  }

  return (
    <AlertDialog open={open} onOpenChange={onOpenChange}>
      <AlertDialogContent>
        <AlertDialogHeader>
          <AlertDialogTitle>{t('Delete Redirect Rule')}</AlertDialogTitle>
          <AlertDialogDescription>
            {t('Are you sure you want to delete the redirect rule for {model}?', {
              model: redirect.source_model,
            })}
            <br />
            {t('This action cannot be undone.')}
          </AlertDialogDescription>
        </AlertDialogHeader>
        <AlertDialogFooter>
          <AlertDialogCancel disabled={isDeleting}>{t('Cancel')}</AlertDialogCancel>
          <AlertDialogAction
            onClick={handleDelete}
            disabled={isDeleting}
            className="bg-destructive text-destructive-foreground hover:bg-destructive/90"
          >
            {isDeleting ? t('Deleting...') : t('Delete')}
          </AlertDialogAction>
        </AlertDialogFooter>
      </AlertDialogContent>
    </AlertDialog>
  )
}
