import { t } from "./i18n";
import type { Key } from "./locales/pt-BR";

/**
 * Controles que podem ganhar uma tecla no modo teclado, na mesma ordem de `Controle::todos`
 * em src/mapa.rs (o servidor também envia esta lista em GET /api/teclado).
 */
export const CONTROL_GROUPS = [
  "Botões de face",
  "Ombros e menu",
  "Direcional",
  "Gatilhos",
  "Analógico esquerdo",
  "Analógico direito",
] as const;

export type ControlGroup = (typeof CONTROL_GROUPS)[number];

const GROUP_KEYS: Record<ControlGroup, Key> = {
  "Botões de face": "controls.group.faceButtons",
  "Ombros e menu": "controls.group.shouldersMenu",
  "Direcional": "controls.group.dpad",
  "Gatilhos": "controls.group.triggers",
  "Analógico esquerdo": "controls.group.leftStick",
  "Analógico direito": "controls.group.rightStick",
};

/** O nome do grupo no idioma atual (o id do grupo, acima, é interno e não muda). */
export const groupName = (group: ControlGroup): string => t(GROUP_KEYS[group]);

export interface ControlInfo {
  id: string;
  /** o nome do controle no idioma atual (calculado na hora, porque o idioma só é escolhido depois do carregamento) */
  readonly label: string;
  group: ControlGroup;
}

/** Sem `key`, o nome é o próprio id (LB, Select, LT…), que não muda de idioma. */
const c = (id: string, group: ControlGroup, key?: Key): ControlInfo => ({
  id,
  group,
  get label() {
    return key ? t(key) : id;
  },
});

export const CONTROLS: readonly ControlInfo[] = [
  c("A", "Botões de face", "controls.A"),
  c("B", "Botões de face", "controls.B"),
  c("X", "Botões de face", "controls.X"),
  c("Y", "Botões de face", "controls.Y"),
  c("LB", "Ombros e menu"),
  c("RB", "Ombros e menu"),
  c("Select", "Ombros e menu"),
  c("Start", "Ombros e menu"),
  c("L3", "Ombros e menu", "controls.L3"),
  c("R3", "Ombros e menu", "controls.R3"),
  c("Up", "Direcional", "direction.up"),
  c("Down", "Direcional", "direction.down"),
  c("Left", "Direcional", "direction.left"),
  c("Right", "Direcional", "direction.right"),
  c("LT", "Gatilhos"),
  c("RT", "Gatilhos"),
  c("LS_Up", "Analógico esquerdo", "direction.up"),
  c("LS_Down", "Analógico esquerdo", "direction.down"),
  c("LS_Left", "Analógico esquerdo", "direction.left"),
  c("LS_Right", "Analógico esquerdo", "direction.right"),
  c("RS_Up", "Analógico direito", "direction.up"),
  c("RS_Down", "Analógico direito", "direction.down"),
  c("RS_Left", "Analógico direito", "direction.left"),
  c("RS_Right", "Analógico direito", "direction.right"),
];
