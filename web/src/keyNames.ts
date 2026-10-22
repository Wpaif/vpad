import { t } from "./i18n";
import type { Key } from "./locales/pt-BR";

/**
 * O servidor manda o catálogo de teclas com os nomes dos grupos e rótulos em português. Aqui eles
 * ganham o idioma da página: o servidor continua mandando os mesmos ids de sempre, e o que não é
 * palavra (letras, setas, Ctrl, Esc…) segue como veio.
 */

const GROUPS: Record<string, Key> = {
  Letras: "keyGroup.letters",
  Números: "keyGroup.numbers",
  Setas: "keyGroup.arrows",
  Especiais: "keyGroup.special",
  Funções: "keyGroup.function",
  Símbolos: "keyGroup.symbols",
};

/** As teclas cujo rótulo é uma palavra do português (as demais são letras, símbolos ou siglas). */
const LABELS: Record<string, Key> = {
  Space: "key.space",
  Backspace: "key.backspace",
};

/** O nome do grupo de teclas no idioma atual; um grupo desconhecido aparece como o servidor mandou. */
export function keyGroupName(group: string): string {
  const key = GROUPS[group];
  return key ? t(key) : group;
}

/** O rótulo da tecla no idioma atual. */
export function keyLabel(key: { id: string; rotulo: string }): string {
  const translated = LABELS[key.id];
  return translated ? t(translated) : key.rotulo;
}
