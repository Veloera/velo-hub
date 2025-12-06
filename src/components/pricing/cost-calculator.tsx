import { useState, useMemo } from 'react'
import { Card, CardContent, CardDescription, CardHeader, CardTitle } from '@/components/ui/card'
import { Input } from '@/components/ui/input'
import { Label } from '@/components/ui/label'
import { ModelPricing } from '@/types'
import {
  Select,
  SelectContent,
  SelectItem,
  SelectTrigger,
  SelectValue,
} from '@/components/ui/select'
import { useI18n } from '@/lib/i18n'

interface CostCalculatorProps {
  pricingData: ModelPricing[]
}

export function CostCalculator({ pricingData }: CostCalculatorProps) {
  const [selectedModelId, setSelectedModelId] = useState<string>('')
  const [inputTokens, setInputTokens] = useState<string>('1000')
  const [outputTokens, setOutputTokens] = useState<string>('500')
  const { t } = useI18n()

  const selectedModel = useMemo(() => {
    return pricingData.find((p) => p.id === selectedModelId)
  }, [pricingData, selectedModelId])

  const calculatedCost = useMemo(() => {
    if (!selectedModel) return null

    const input = parseInt(inputTokens) || 0
    const output = parseInt(outputTokens) || 0

    const inputCost = (input / 1_000_000) * selectedModel.input_cost_per_1m
    const outputCost = (output / 1_000_000) * selectedModel.output_cost_per_1m
    const totalCost = inputCost + outputCost

    return {
      inputCost,
      outputCost,
      totalCost,
    }
  }, [selectedModel, inputTokens, outputTokens])

  const formatCost = (cost: number) => {
    if (cost < 0.01) {
      return `$${cost.toFixed(6)}`
    }
    return `$${cost.toFixed(4)}`
  }

  return (
    <Card>
      <CardHeader>
        <CardTitle>{t('Cost Calculator')}</CardTitle>
        <CardDescription>
          {t('Calculate estimated costs based on token usage and model pricing')}
        </CardDescription>
      </CardHeader>
      <CardContent className="space-y-4">
        <div className="space-y-2">
          <Label htmlFor="model-select">{t('Select Model')}</Label>
          <Select value={selectedModelId} onValueChange={setSelectedModelId}>
            <SelectTrigger id="model-select">
              <SelectValue placeholder={t('Choose a model...')} />
            </SelectTrigger>
            <SelectContent>
              {pricingData.map((pricing) => (
                <SelectItem key={pricing.id} value={pricing.id}>
                  {pricing.model_name} ({pricing.provider})
                </SelectItem>
              ))}
            </SelectContent>
          </Select>
        </div>

        <div className="grid grid-cols-2 gap-4">
          <div className="space-y-2">
            <Label htmlFor="input-tokens">{t('Input Tokens')}</Label>
            <Input
              id="input-tokens"
              type="number"
              value={inputTokens}
              onChange={(e) => setInputTokens(e.target.value)}
              placeholder="1000"
              min="0"
            />
          </div>

          <div className="space-y-2">
            <Label htmlFor="output-tokens">{t('Output Tokens')}</Label>
            <Input
              id="output-tokens"
              type="number"
              value={outputTokens}
              onChange={(e) => setOutputTokens(e.target.value)}
              placeholder="500"
              min="0"
            />
          </div>
        </div>

        {calculatedCost && selectedModel && (
          <div className="rounded-lg border bg-muted/50 p-4 space-y-2">
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">{t('Input Cost:')}</span>
              <span className="font-mono">{formatCost(calculatedCost.inputCost)}</span>
            </div>
            <div className="flex justify-between text-sm">
              <span className="text-muted-foreground">{t('Output Cost:')}</span>
              <span className="font-mono">{formatCost(calculatedCost.outputCost)}</span>
            </div>
            <div className="flex justify-between border-t pt-2 font-semibold">
              <span>{t('Total Cost:')}</span>
              <span className="font-mono text-lg">{formatCost(calculatedCost.totalCost)}</span>
            </div>
            <div className="text-xs text-muted-foreground pt-2">
              {t('Based on {model} pricing: {input}/1M input tokens, {output}/1M output tokens', {
                model: selectedModel.model_name,
                input: formatCost(selectedModel.input_cost_per_1m),
                output: formatCost(selectedModel.output_cost_per_1m),
              })}
            </div>
          </div>
        )}

        {!selectedModel && (
          <div className="rounded-lg border border-dashed p-4 text-center text-sm text-muted-foreground">
            {t('Select a model to calculate costs')}
          </div>
        )}
      </CardContent>
    </Card>
  )
}
