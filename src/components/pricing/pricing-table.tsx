import { ModelPricing } from '@/types'
import {
  Table,
  TableBody,
  TableCell,
  TableHead,
  TableHeader,
  TableRow,
} from '@/components/ui/table'
import { Button } from '@/components/ui/button'
import { ChevronLeft, ChevronRight } from 'lucide-react'
import { useI18n } from '@/lib/i18n'

interface PricingTableProps {
  pricingData: ModelPricing[]
  currentPage: number
  pageSize: number
  onPageChange: (page: number) => void
  totalItems?: number
}

export function PricingTable({
  pricingData,
  currentPage,
  pageSize,
  onPageChange,
}: PricingTableProps) {
  const { t } = useI18n()
  const formatCost = (cost: number) => {
    return `$${cost.toFixed(4)}`
  }

  const formatDate = (timestamp: number) => {
    return new Date(timestamp * 1000).toLocaleDateString()
  }

  const hasNextPage = pricingData.length === pageSize
  const hasPrevPage = currentPage > 0

  return (
    <div className="space-y-4">
      <div className="rounded-md border">
        <Table>
          <TableHeader>
            <TableRow>
              <TableHead>{t('Model Name')}</TableHead>
              <TableHead>{t('Provider')}</TableHead>
              <TableHead className="text-right">{t('Input Cost (per 1M tokens)')}</TableHead>
              <TableHead className="text-right">{t('Output Cost (per 1M tokens)')}</TableHead>
              <TableHead className="text-right">{t('Last Updated')}</TableHead>
            </TableRow>
          </TableHeader>
          <TableBody>
            {pricingData.length === 0 ? (
              <TableRow>
                <TableCell colSpan={5} className="text-center text-muted-foreground">
                  {t('No pricing data available')}
                </TableCell>
              </TableRow>
            ) : (
              pricingData.map((pricing) => (
                <TableRow key={pricing.id}>
                  <TableCell className="font-medium">{pricing.model_name}</TableCell>
                  <TableCell>
                    <span className="inline-flex items-center rounded-full px-2 py-1 text-xs font-medium bg-primary/10 text-primary">
                      {pricing.provider}
                    </span>
                  </TableCell>
                  <TableCell className="text-right font-mono">
                    {formatCost(pricing.input_cost_per_1m)}
                  </TableCell>
                  <TableCell className="text-right font-mono">
                    {formatCost(pricing.output_cost_per_1m)}
                  </TableCell>
                  <TableCell className="text-right text-muted-foreground">
                    {formatDate(pricing.last_updated)}
                  </TableCell>
                </TableRow>
              ))
            )}
          </TableBody>
        </Table>
      </div>

      <div className="flex items-center justify-between">
        <div className="text-sm text-muted-foreground">
          {t('Page {page} • Showing {count} items', {
            page: currentPage + 1,
            count: pricingData.length,
          })}
        </div>
        <div className="flex gap-2">
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(currentPage - 1)}
            disabled={!hasPrevPage}
          >
            <ChevronLeft className="h-4 w-4 mr-1" />
            {t('Previous')}
          </Button>
          <Button
            variant="outline"
            size="sm"
            onClick={() => onPageChange(currentPage + 1)}
            disabled={!hasNextPage}
          >
            {t('Next')}
            <ChevronRight className="h-4 w-4 ml-1" />
          </Button>
        </div>
      </div>
    </div>
  )
}
