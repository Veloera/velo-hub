import { useState } from "react";
import { useQuery, useMutation, useQueryClient } from "@tanstack/react-query";
import { Provider } from "@/types";
import {
  getProviders,
  createProvider,
  updateProvider,
  deleteProvider,
  testProviderConnection,
} from "@/lib/tauri-api";
import { useProviderStore } from "@/stores/provider-store";
import { ProviderList } from "@/components/providers/provider-list";
import { ProviderForm } from "@/components/providers/provider-form";
import { DeleteProviderDialog } from "@/components/providers/delete-provider-dialog";
import { useI18n } from "@/lib/i18n";

export default function ProvidersPage() {
  const queryClient = useQueryClient();
  const { setProviders, setLoading, setError } = useProviderStore();
  const { t } = useI18n();

  const [isFormOpen, setIsFormOpen] = useState(false);
  const [isDeleteDialogOpen, setIsDeleteDialogOpen] = useState(false);
  const [selectedProvider, setSelectedProvider] = useState<Provider | null>(
    null
  );
  const [providerToDelete, setProviderToDelete] = useState<Provider | null>(
    null
  );

  // Fetch providers
  const { data: providers = [], isLoading } = useQuery({
    queryKey: ["providers"],
    queryFn: async () => {
      setLoading(true);
      try {
        const data = await getProviders();
        setProviders(data);
        return data;
      } catch (error) {
        setError(
          error instanceof Error ? error.message : t("Failed to load providers")
        );
        throw error;
      } finally {
        setLoading(false);
      }
    },
  });

  // Create provider mutation
  const createMutation = useMutation({
    mutationFn: async (provider: Partial<Provider>) => {
      const newProvider: Provider = {
        id: crypto.randomUUID(),
        name: provider.name!,
        provider_type: provider.provider_type!,
        endpoint: provider.endpoint!,
        api_key: provider.api_key!,
        priority: provider.priority ?? 0,
        weight: provider.weight ?? 1,
        enabled: provider.enabled ?? true,
        proxy: provider.proxy,
        created_at: Date.now(),
        updated_at: Date.now(),
      };
      return createProvider(newProvider);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });

  // Update provider mutation
  const updateMutation = useMutation({
    mutationFn: async (provider: Partial<Provider>) => {
      const updatedProvider: Provider = {
        ...(providers.find((p) => p.id === provider.id) as Provider),
        ...provider,
        updated_at: Date.now(),
      };
      return updateProvider(updatedProvider);
    },
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
    },
  });

  // Delete provider mutation
  const deleteMutation = useMutation({
    mutationFn: (providerId: string) => deleteProvider(providerId),
    onSuccess: () => {
      queryClient.invalidateQueries({ queryKey: ["providers"] });
      setIsDeleteDialogOpen(false);
      setProviderToDelete(null);
    },
  });

  // Test connection mutation
  const testMutation = useMutation({
    mutationFn: async (provider: Provider) => {
      return testProviderConnection(provider);
    },
  });

  const handleToggleEnabled = async (id: string, enabled: boolean) => {
    await updateMutation.mutateAsync({ id, enabled });
  };

  const handleCreate = () => {
    setSelectedProvider(null);
    setIsFormOpen(true);
  };

  const handleEdit = (provider: Provider) => {
    setSelectedProvider(provider);
    setIsFormOpen(true);
  };

  const handleDelete = (provider: Provider) => {
    setProviderToDelete(provider);
    setIsDeleteDialogOpen(true);
  };

  const handleFormSubmit = async (provider: Partial<Provider>) => {
    if (selectedProvider) {
      await updateMutation.mutateAsync({ ...provider, id: selectedProvider.id });
    } else {
      await createMutation.mutateAsync(provider);
    }
  };

  const handleTestConnection = async (provider: Provider) => {
    try {
      const result = await testMutation.mutateAsync(provider);
      return result;
    } catch (error) {
      console.error("Test connection failed:", error);
      return false;
    }
  };

  const handleConfirmDelete = () => {
    if (providerToDelete) {
      deleteMutation.mutate(providerToDelete.id);
    }
  };

  return (
    <div className="space-y-6">
      <div>
        <h1 className="text-3xl font-bold tracking-tight">{t("Providers")}</h1>
        <p className="text-muted-foreground">
          {t("Manage your LLM service providers and configurations")}
        </p>
      </div>

      <ProviderList
        providers={providers}
        onToggleEnabled={handleToggleEnabled}
        onEdit={handleEdit}
        onDelete={handleDelete}
        onCreate={handleCreate}
        isLoading={isLoading}
      />

      <ProviderForm
        open={isFormOpen}
        onOpenChange={setIsFormOpen}
        provider={selectedProvider}
        onSubmit={handleFormSubmit}
        onTest={handleTestConnection}
      />

      <DeleteProviderDialog
        open={isDeleteDialogOpen}
        onOpenChange={setIsDeleteDialogOpen}
        provider={providerToDelete}
        onConfirm={handleConfirmDelete}
        isDeleting={deleteMutation.isPending}
      />
    </div>
  );
}
