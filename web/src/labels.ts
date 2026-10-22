/**
 * Rótulos dos botões por perfil. O protocolo identifica os botões pela posição no layout
 * do Xbox (A embaixo, B à direita, X à esquerda, Y em cima), então o rótulo muda mas o
 * `data-btn` nunca: no Switch Pro, o botão de baixo (id A) é o "B" da Nintendo.
 */

import { t } from "./i18n";
import { keyLabel } from "./keyNames";
import type { Key } from "./locales/pt-BR";

type Shape = "cross" | "circle" | "square" | "triangle";

interface Label {
  text?: string;
  shape?: Shape;
  /** chave do nome acessível, quando o texto/símbolo sozinho não basta (resolvida ao desenhar) */
  name?: Key;
}

type Control = "A" | "B" | "X" | "Y" | "LB" | "RB" | "LT" | "RT" | "Select" | "Start";

const text = (value: string, name?: Key): Label => ({ text: value, name });
const shape = (s: Shape, name: Key): Label => ({ shape: s, name });

const LABELS: Record<string, Record<Control, Label>> = {
  xbox360: {
    A: text("A"), B: text("B"), X: text("X"), Y: text("Y"),
    LB: text("LB"), RB: text("RB"), LT: text("LT"), RT: text("RT"),
    Select: text("Back"), Start: text("Start"),
  },
  ds4: {
    A: shape("cross", "shape.cross"), B: shape("circle", "shape.circle"),
    X: shape("square", "shape.square"), Y: shape("triangle", "shape.triangle"),
    LB: text("L1"), RB: text("R1"), LT: text("L2"), RT: text("R2"),
    Select: text("Share"), Start: text("Options"),
  },
  "switch-pro": {
    A: text("B"), B: text("A"), X: text("Y"), Y: text("X"),
    LB: text("L"), RB: text("R"), LT: text("ZL"), RT: text("ZR"),
    Select: text("−", "sign.minus"), Start: text("+", "sign.plus"),
  },
};

const SHAPES: Record<Shape, string> = {
  cross: '<path d="M6 6l12 12M18 6L6 18"/>',
  circle: '<circle cx="12" cy="12" r="7"/>',
  square: '<rect x="5.5" y="5.5" width="13" height="13" rx="1"/>',
  triangle: '<path d="M12 5l7.5 13h-15z"/>',
};

const selector = (control: Control) =>
  control === "LT" || control === "RT" ? `[data-trigger=${control}]` : `[data-btn=${control}]`;

function render(el: HTMLElement, label: Label) {
  if (label.shape) {
    el.innerHTML =
      `<svg viewBox="0 0 24 24" data-glyph="${label.shape}" aria-hidden="true">${SHAPES[label.shape]}</svg>`;
  } else {
    el.textContent = label.text ?? "";
  }
  el.toggleAttribute("data-symbol", [...(label.text ?? "")].length === 1);
  el.removeAttribute("data-keycap");
  if (label.name) el.setAttribute("aria-label", t(label.name));
  else el.removeAttribute("aria-label");
}

/** Devolve false (sem alterar nada) se o perfil for desconhecido. */
export function applyProfileLabels(pad: HTMLElement, profileId: string): boolean {
  const labels = LABELS[profileId];
  if (!labels) return false;
  for (const [control, label] of Object.entries(labels) as Array<[Control, Label]>) {
    const el = pad.querySelector<HTMLElement>(selector(control));
    if (el) render(el, label);
  }
  pad.dataset.profile = profileId;
  return true;
}

interface KeyInfo {
  id: string;
  rotulo: string;
}

/** No modo teclado cada botão mostra a tecla que aciona (controle sem tecla mostra um traço). */
export function applyKeyboardLabels(pad: HTMLElement, map: Record<string, string>, keys: readonly KeyInfo[]): void {
  const names = new Map(keys.map((k) => [k.id, keyLabel(k)]));
  const controls = Object.keys(LABELS.xbox360!) as Control[];
  for (const control of controls) {
    const el = pad.querySelector<HTMLElement>(selector(control));
    if (!el) continue;
    const keyId = map[control];
    const name = keyId === undefined ? undefined : (names.get(keyId) ?? keyId);
    el.textContent = name ?? "–";
    el.setAttribute("aria-label", name === undefined ? t("labels.keycap.empty") : t("labels.keycap", { name }));
    el.setAttribute("data-keycap", "");
    el.removeAttribute("data-symbol");
  }
  pad.dataset.profile = "teclado";
}

/**
 * O nome que o botão tem agora, para outras telas (como a lista de ocultar) acompanharem o perfil:
 * no teclado é a tecla; nos demais é o nome acessível (formas do DualShock, − e + do Switch) ou o
 * texto. Devolve null para controle sem rótulo mutável ou sem tecla, e quem chama usa o próprio nome.
 */
export function displayedLabel(pad: HTMLElement, control: string): string | null {
  if (!(control in LABELS.xbox360!)) return null;
  const el = pad.querySelector<HTMLElement>(selector(control as Control));
  if (!el) return null;
  const name = el.hasAttribute("data-keycap") ? el.textContent : (el.getAttribute("aria-label") ?? el.textContent);
  const trimmed = name?.trim();
  return trimmed && trimmed !== "–" ? trimmed : null;
}
