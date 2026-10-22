import type { Language } from "../i18n";
import { en } from "./en";
import { es } from "./es";
import { ja } from "./ja";
import { ko } from "./ko";
import { ptBR, type Key } from "./pt-BR";
import { zhCN } from "./zh-CN";

export const CATALOGS: Record<Language, Record<Key, string>> = {
  "pt-BR": ptBR,
  en,
  es,
  "zh-CN": zhCN,
  ja,
  ko,
};
