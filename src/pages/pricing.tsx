import { useState, useCallback } from 'react'
import { useQuery } from '@tanstack/react-query'
import { getModelPricing, searchModelPricing } from '@/lib/tauri-api'
import { PricingTable } from '@/components/pricing/pricing-table'
import { PricingSearch } from '@/components/pricing/pricing-search'
import { PricingSync } from '@/components/pricing/pricing-sync'
import { CostCalculator } from '@/components/pricing/cost-calculator'
import { Tabs, TabsContent, TabsList, TabsTrigger } from '@/components/ui/tabs'
import { useI18n } from '@/lib/i18n'

const PAGE_SIZE = 20

export default function PricingPage() {
  const [currentPage, setCurrentPage] = useState(0)
  const [searchQuery, setSearchQuery] = useState('')
  const [activeTab, setActiveTab] = useState('browse')
  const { t } = useI18n()

  // Query for paginated pricing data
  const { data: pricingData = [], isLoading, refetch } = useQuery({
    queryKey: ['model-pricing', currentPage, searchQuery],
    queryFn: async () => {
      if (searchQuery.trim()) {
        return searchModelPricing(searchQuery)
      }
      return getModelPricing(currentPage, PAGE_SIZE)
    },
  })

  const handleSearch = useCallback((query: string) => {
    setSearchQuery(query)
    setCurrentPage(0) // Reset to first page on search
  }, [])

  const handlePageChange = useCallback((page: number) => {
    setCurrentPage(page)
  }, [])

  const handleSyncComplete = useCallback(() => {
    refetch()
  }, [refetch])

  return (
    <div className="space-y-6">
      <div className="flex items-center justify-between">
        <div>
          <h1 className="text-3xl font-bold tracking-tight">{t('Model Pricing')}</h1>
          <p className="text-muted-foreground">
            {t('View and manage model pricing information')}
          </p>
        </div>
        <PricingSync onSyncComplete={handleSyncComplete} />
      </div>

      <Tabs value={activeTab} onValueChange={setActiveTab}>
        <TabsList>
          <TabsTrigger value="browse">{t('Browse Pricing')}</TabsTrigger>
          <TabsTrigger value="calculator">{t('Cost Calculator')}</TabsTrigger>
        </TabsList>

        <TabsContent value="browse" className="space-y-4">
          <PricingSearch onSearch={handleSearch} />

          {isLoading ? (
            <div className="flex items-center justify-center py-12">
              <div className="text-muted-foreground">{t('Loading pricing data...')}</div>
            </div>
          ) : (
            <PricingTable
              pricingData={pricingData}
              currentPage={currentPage}
              pageSize={PAGE_SIZE}
              onPageChange={handlePageChange}
            />
          )}
        </TabsContent>

        <TabsContent value="calculator">
          <CostCalculator pricingData={pricingData} />
        </TabsContent>
      </Tabs>
    </div>
  )
}
