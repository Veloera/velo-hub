import { create } from "zustand";
import { Provider } from "@/types";

interface ProviderState {
  providers: Provider[];
  selectedProvider: Provider | null;
  isLoading: boolean;
  error: string | null;
  setProviders: (providers: Provider[]) => void;
  addProvider: (provider: Provider) => void;
  updateProvider: (id: string, provider: Partial<Provider>) => void;
  deleteProvider: (id: string) => void;
  selectProvider: (provider: Provider | null) => void;
  setLoading: (loading: boolean) => void;
  setError: (error: string | null) => void;
}

export const useProviderStore = create<ProviderState>((set) => ({
  providers: [],
  selectedProvider: null,
  isLoading: false,
  error: null,
  setProviders: (providers) => set({ providers }),
  addProvider: (provider) =>
    set((state) => ({ providers: [...state.providers, provider] })),
  updateProvider: (id, updatedProvider) =>
    set((state) => ({
      providers: state.providers.map((p) =>
        p.id === id ? { ...p, ...updatedProvider } : p
      ),
    })),
  deleteProvider: (id) =>
    set((state) => ({
      providers: state.providers.filter((p) => p.id !== id),
    })),
  selectProvider: (provider) => set({ selectedProvider: provider }),
  setLoading: (loading) => set({ isLoading: loading }),
  setError: (error) => set({ error }),
}));
