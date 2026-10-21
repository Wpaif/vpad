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

export interface ControlInfo {
  id: string;
  label: string;
  group: ControlGroup;
}

const c = (id: string, label: string, group: ControlGroup): ControlInfo => ({ id, label, group });

export const CONTROLS: readonly ControlInfo[] = [
  c("A", "Baixo (A)", "Botões de face"),
  c("B", "Direita (B)", "Botões de face"),
  c("X", "Esquerda (X)", "Botões de face"),
  c("Y", "Cima (Y)", "Botões de face"),
  c("LB", "LB", "Ombros e menu"),
  c("RB", "RB", "Ombros e menu"),
  c("Select", "Select", "Ombros e menu"),
  c("Start", "Start", "Ombros e menu"),
  c("L3", "Clique esquerdo", "Ombros e menu"),
  c("R3", "Clique direito", "Ombros e menu"),
  c("Up", "Cima", "Direcional"),
  c("Down", "Baixo", "Direcional"),
  c("Left", "Esquerda", "Direcional"),
  c("Right", "Direita", "Direcional"),
  c("LT", "LT", "Gatilhos"),
  c("RT", "RT", "Gatilhos"),
  c("LS_Up", "Cima", "Analógico esquerdo"),
  c("LS_Down", "Baixo", "Analógico esquerdo"),
  c("LS_Left", "Esquerda", "Analógico esquerdo"),
  c("LS_Right", "Direita", "Analógico esquerdo"),
  c("RS_Up", "Cima", "Analógico direito"),
  c("RS_Down", "Baixo", "Analógico direito"),
  c("RS_Left", "Esquerda", "Analógico direito"),
  c("RS_Right", "Direita", "Analógico direito"),
];
