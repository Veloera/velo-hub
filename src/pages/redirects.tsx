import { useState } from 'react'
import { Plus } from 'lucide-react'
import { Button } from '@/components/ui/button'
import { RedirectList } from '@/components/redirects/redirect-list'
import { RedirectDialog } from '@/components/redirects/redirect-dialog'
import { useI18n } from '@/lib/i18n'

export default function RedirectsPage() {
  const [isDialogOpen, setIsDialogOpen] = useState(false)
  const { t } = useI18n()

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold">{t('Model Redirects')}</h1>
          <p className="text-muted-foreground mt-2">
            {t('Configure model name mappings to redirect requests to different models')}
          </p>
        </div>
        <Button onClick={() => setIsDialogOpen(true)}>
          <Plus className="mr-2 h-4 w-4" />
          {t('Add Redirect')}
        </Button>
      </div>

      <RedirectList />

      <RedirectDialog
        open={isDialogOpen}
        onOpenChange={setIsDialogOpen}
      />
    </div>
  )
}
