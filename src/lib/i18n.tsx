import {
  createContext,
  useCallback,
  useContext,
  useEffect,
  useMemo,
  useState,
  type ReactNode,
} from "react";
import { zhTranslations } from "@/i18n/dictionary";

export type Language = "en" | "zh";

type TranslationVariables = Record<string, string | number>;

interface I18nContextValue {
  language: Language;
  setLanguage: (language: Language) => void;
  t: (key: string, vars?: TranslationVariables) => string;
}

const LANGUAGE_STORAGE_KEY = "velo-hub-language";
const FALLBACK_LANGUAGE: Language = "en";

const translations: Record<Language, Record<string, string>> = {
  en: {},
  zh: zhTranslations,
};

const I18nContext = createContext<I18nContextValue | undefined>(undefined);

const isLanguage = (value: unknown): value is Language =>
  value === "en" || value === "zh";

const detectLanguage = (): Language => {
  if (typeof navigator === "undefined") {
    return FALLBACK_LANGUAGE;
  }

  const locales = navigator.languages?.length
    ? navigator.languages
    : [navigator.language];

  const preferred = locales?.[0]?.toLowerCase() ?? "";
  if (preferred.startsWith("zh")) {
    return "zh";
  }

  return FALLBACK_LANGUAGE;
};

const getInitialLanguage = (): Language => {
  if (typeof window === "undefined") {
    return FALLBACK_LANGUAGE;
  }

  const stored = window.localStorage.getItem(LANGUAGE_STORAGE_KEY);
  if (isLanguage(stored)) {
    return stored;
  }

  const detected = detectLanguage();
  window.localStorage.setItem(LANGUAGE_STORAGE_KEY, detected);
  return detected;
};

const formatWithVariables = (
  value: string,
  vars?: TranslationVariables
): string => {
  if (!vars) {
    return value;
  }

  return value.replace(/\{(.*?)\}/g, (_, token: string) => {
    const trimmed = token.trim();
    return trimmed in vars ? String(vars[trimmed]) : `{${trimmed}}`;
  });
};

export const SUPPORTED_LANGUAGES: Array<{
  value: Language;
  labelKey: string;
}> = [
  { value: "en", labelKey: "English" },
  { value: "zh", labelKey: "Chinese (Simplified)" },
];

export function I18nProvider({
  children,
}: {
  children: ReactNode;
}) {
  const [language, setLanguageState] = useState<Language>(() =>
    getInitialLanguage()
  );

  useEffect(() => {
    if (typeof window === "undefined") {
      return;
    }
    window.localStorage.setItem(LANGUAGE_STORAGE_KEY, language);
  }, [language]);

  useEffect(() => {
    if (typeof document === "undefined") {
      return;
    }
    document.documentElement.lang = language === "zh" ? "zh-CN" : "en";
  }, [language]);

  const translate = useCallback(
    (key: string, vars?: TranslationVariables) => {
      const dictionary = translations[language] ?? {};
      const fallbackBase = translations[FALLBACK_LANGUAGE][key] ?? key;
      const base = dictionary[key] ?? fallbackBase;
      return formatWithVariables(base, vars);
    },
    [language]
  );

  const value = useMemo<I18nContextValue>(
    () => ({
      language,
      setLanguage: (next) => setLanguageState(next),
      t: translate,
    }),
    [language, translate]
  );

  return <I18nContext.Provider value={value}>{children}</I18nContext.Provider>;
}

export const useI18n = () => {
  const context = useContext(I18nContext);
  if (!context) {
    throw new Error("useI18n must be used within an I18nProvider");
  }
  return context;
};
