/**
 * Catálogo em português: é ele que define as chaves. Os outros idiomas são tipados como
 * `Record<Key, string>`, então esquecer uma tradução é erro de compilação (e o teste de
 * catálogos confere também os {marcadores}).
 */
export const ptBR = {
  "language.label": "Idioma",
} as const;

export type Key = keyof typeof ptBR;
