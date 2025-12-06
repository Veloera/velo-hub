import { invoke } from '@tauri-apps/api/core'
import type { Provider, DashboardMetrics, SessionInfo, ModelPricing } from '@/types'

// Provider management commands
export async function getProviders(): Promise<Provider[]> {
  return invoke('get_providers')
}

export async function createProvider(provider: Provider): Promise<Provider> {
  return invoke('create_provider', { provider })
}

export async function updateProvider(provider: Provider): Promise<Provider> {
  return invoke('update_provider', { provider })
}

export async function deleteProvider(providerId: string): Promise<void> {
  return invoke('delete_provider', { providerId })
}

export async function testProviderConnection(provider: Provider): Promise<boolean> {
  return invoke('test_provider_connection', { provider })
}

// Monitoring commands
export async function getDashboardMetrics(): Promise<DashboardMetrics> {
  return invoke('get_dashboard_metrics')
}

export async function getActiveSessions(): Promise<SessionInfo[]> {
  return invoke('get_active_sessions')
}

// Configuration commands
export async function exportConfig(): Promise<string> {
  return invoke('export_config')
}

export async function importConfig(configJson: string): Promise<void> {
  return invoke('import_config', { configJson })
}

// Pricing management commands
export async function getModelPricing(page: number, pageSize: number): Promise<ModelPricing[]> {
  return invoke('get_model_pricing', { page, pageSize })
}

export async function searchModelPricing(query: string): Promise<ModelPricing[]> {
  return invoke('search_model_pricing', { query })
}

export async function syncPricingFromLitellm(): Promise<number> {
  return invoke('sync_pricing_from_litellm')
}

// Settings management commands
export async function getGlobalConfig(): Promise<import('@/types').AppConfig> {
  return invoke('get_global_config')
}

export async function updateGlobalConfig(config: import('@/types').AppConfig): Promise<void> {
  return invoke('update_global_config', { config })
}
