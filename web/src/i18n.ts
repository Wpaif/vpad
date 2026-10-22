import type { Store } from "./auth";
import { CATALOGS } from "./locales";
import { ptBR, type Key } from "./locales/pt-BR";

/** Idiomas da página: só os predefinidos, cada um com o nome na própria língua. */
export const LANGUAGES = [
  { code: "pt-BR", name: "Português" },
  { code: "en", name: "English" },
  { code: "es", name: "Español" },
  { code: "zh-CN", name: "中文" },
  { code: "ja", name: "日本語" },
  { code: "ko", name: "한국어" },
] as const;

export type Language = (typeof LANGUAGES)[number]["code"];
export const DEFAULT_LANGUAGE: Language = "pt-BR";
export const LANGUAGE_KEY = "vpad:idioma";

const isLanguage = (code: unknown): code is Language => LANGUAGES.some((l) => l.code === code);

const BY_PREFIX: Record<string, Language> = {
  pt: "pt-BR",
  en: "en",
  es: "es",
  zh: "zh-CN",
  ja: "ja",
  ko: "ko",
};

/**
 * O primeiro idioma suportado da lista de preferência do navegador. Quem fala uma língua sem
 * tradução recebe o inglês. (Todo chinês vira o simplificado: é o único que existe aqui.)
 */
export function detectLanguage(tags: readonly string[]): Language {
  for (const tag of tags) {
    const found = BY_PREFIX[tag.toLowerCase().split("-")[0] ?? ""];
    if (found) return found;
  }
  return "en";
}

// Até alguém chamar `initLanguage` (só o main.ts faz isso), vale o português: os testes e
// qualquer uso fora da página enxergam os textos de sempre.
let current: Language = DEFAULT_LANGUAGE;

export function getLanguage(): Language {
  return current;
}

export function setLanguage(language: Language): void {
  current = language;
  document.documentElement.lang = language;
}

/** Escolhe o idioma ao abrir a página: o que o usuário guardou, ou o do celular. */
export function initLanguage({ languages, storage }: { languages: readonly string[]; storage: Store }): Language {
  let chosen: unknown = null;
  try {
    chosen = storage.getItem(LANGUAGE_KEY);
  } catch {
    // sem armazenamento: vale o do celular
  }
  const language = isLanguage(chosen) ? chosen : detectLanguage(languages);
  setLanguage(language);
  return language;
}

/**
 * Guarda a escolha do usuário e recarrega a página: todo texto montado em código reaparece no
 * idioma novo, sem ter de refazer cada tela. A conexão e o pareamento voltam sozinhos.
 */
export function chooseLanguage(language: Language, storage: Store, reload: () => void = () => location.reload()): void {
  if (!isLanguage(language)) return;
  try {
    storage.setItem(LANGUAGE_KEY, language);
  } catch {
    // sem armazenamento: vale só até recarregar
  }
  if (language !== current) reload();
}

/** O texto da chave no idioma atual, com os `{marcadores}` preenchidos. */
export function t(key: Key, params: Record<string, string | number> = {}): string {
  const text = CATALOGS[current][key] ?? ptBR[key] ?? key;
  return text.replace(/\{(\w+)\}/g, (marcador, nome: string) => (nome in params ? String(params[nome]) : marcador));
}

const hasKey = (key: string | null): key is Key => key !== null && key in ptBR;

/**
 * Traduz o que o markup marcou: `data-i18n` (conteúdo), `data-i18n-aria-label`, `data-i18n-title`
 * e `data-i18n-placeholder`. O texto escrito no markup continua valendo se a chave não existir.
 */
export function applyStaticTranslations(root: ParentNode): void {
  for (const el of root.querySelectorAll<HTMLElement>("[data-i18n]")) {
    const key = el.getAttribute("data-i18n");
    if (hasKey(key)) el.textContent = t(key);
  }
  for (const attr of ["aria-label", "title", "placeholder"]) {
    for (const el of root.querySelectorAll<HTMLElement>(`[data-i18n-${attr}]`)) {
      const key = el.getAttribute(`data-i18n-${attr}`);
      if (hasKey(key)) el.setAttribute(attr, t(key));
    }
  }
}
