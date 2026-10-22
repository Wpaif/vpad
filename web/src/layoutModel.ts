/**
 * Modelo do layout do controle: **ajustes sobre o padrão**, não posições absolutas. Cada grupo
 * guarda um deslocamento (em fração da área útil da tela) e uma escala; os controles ocultos
 * ficam numa lista. Um grupo sem ajuste e uma lista vazia são o layout padrão, que continua
 * responsivo exatamente como o CSS o define. Tudo aqui é puro e imutável.
 */

export const LAYOUT_VERSION = 1;

export const GROUP_IDS = [
  "ombros-esq",
  "ombros-dir",
  "analogico-esq",
  "analogico-dir",
  "direcional",
  "face",
  "menu",
] as const;
export type GroupId = (typeof GROUP_IDS)[number];

/** Controles que podem ser ocultados um a um (os ids do protocolo, mais os dois analógicos). */
export const CONTROL_IDS = [
  "A", "B", "X", "Y",
  "LB", "RB", "LT", "RT",
  "Select", "Start",
  "Up", "Down", "Left", "Right",
  "StickLeft", "StickRight",
] as const;
export type ControlId = (typeof CONTROL_IDS)[number];

export const GROUP_OF: Readonly<Record<ControlId, GroupId>> = {
  A: "face", B: "face", X: "face", Y: "face",
  LB: "ombros-esq", LT: "ombros-esq",
  RB: "ombros-dir", RT: "ombros-dir",
  Select: "menu", Start: "menu",
  Up: "direcional", Down: "direcional", Left: "direcional", Right: "direcional",
  StickLeft: "analogico-esq",
  StickRight: "analogico-dir",
};

/** Controles de um grupo, na ordem do catálogo. */
export function controlsOf(group: GroupId): ControlId[] {
  return CONTROL_IDS.filter((c) => GROUP_OF[c] === group);
}

export const MIN_SCALE = 0.5;
export const MAX_SCALE = 2;
/** Opacidade dos botões: nunca some de vez (20% ainda se enxerga e se acerta). */
export const MIN_OPACITY = 0.2;
/** O deslocamento é limitado a uma tela inteira para cada lado. */
export const MAX_OFFSET = 1;

export interface GroupAdjust {
  /** fração da largura da área útil; positivo vai para a direita */
  dx: number;
  /** fração da altura da área útil; positivo vai para baixo */
  dy: number;
  scale: number;
}

/** Deslocamento de um controle solto do grupo, em fração da área útil, na escala do grupo. */
export interface ControlAdjust {
  dx: number;
  dy: number;
}

export interface Layout {
  version: typeof LAYOUT_VERSION;
  groups: Partial<Record<GroupId, GroupAdjust>>;
  hidden: ControlId[];
  /** só existe quando algum controle foi movido individualmente */
  controls?: Partial<Record<ControlId, ControlAdjust>>;
  /** opacidade de todos os botões (MIN_OPACITY a 1); só existe quando é menor que 1 */
  opacity?: number;
}

const NEUTRAL: GroupAdjust = { dx: 0, dy: 0, scale: 1 };

export function defaultLayout(): Layout {
  return { version: LAYOUT_VERSION, groups: {}, hidden: [] };
}

const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));
const round = (v: number, casas: number) => Math.round(v * 10 ** casas) / 10 ** casas;

/** Limita e arredonda; um número inválido (NaN, infinito) volta ao valor neutro do campo. */
function clean(adjust: Partial<GroupAdjust>): GroupAdjust {
  const num = (v: number | undefined, neutro: number) => (typeof v === "number" && Number.isFinite(v) ? v : neutro);
  return {
    dx: round(clamp(num(adjust.dx, NEUTRAL.dx), -MAX_OFFSET, MAX_OFFSET), 4),
    dy: round(clamp(num(adjust.dy, NEUTRAL.dy), -MAX_OFFSET, MAX_OFFSET), 4),
    scale: round(clamp(num(adjust.scale, NEUTRAL.scale), MIN_SCALE, MAX_SCALE), 3),
  };
}

const NO_OFFSET: ControlAdjust = { dx: 0, dy: 0 };

function cleanOffset(adjust: Partial<ControlAdjust>): ControlAdjust {
  const num = (v: number | undefined) => (typeof v === "number" && Number.isFinite(v) ? v : 0);
  return {
    dx: round(clamp(num(adjust.dx), -MAX_OFFSET, MAX_OFFSET), 4),
    dy: round(clamp(num(adjust.dy), -MAX_OFFSET, MAX_OFFSET), 4),
  };
}

const isNeutral = (a: GroupAdjust) => a.dx === NEUTRAL.dx && a.dy === NEUTRAL.dy && a.scale === NEUTRAL.scale;

/** Remove o que é desconhecido, repetido ou neutro e põe tudo na ordem do catálogo. */
export function normalizeLayout(layout: Layout): Layout {
  const groups: Layout["groups"] = {};
  for (const id of GROUP_IDS) {
    const raw = Object.hasOwn(layout.groups, id) ? layout.groups[id] : undefined;
    if (!raw) continue;
    const adjust = clean(raw);
    if (!isNeutral(adjust)) groups[id] = adjust;
  }
  const hidden = CONTROL_IDS.filter((c) => layout.hidden.includes(c));
  const controls: NonNullable<Layout["controls"]> = {};
  for (const id of CONTROL_IDS) {
    const raw = layout.controls && Object.hasOwn(layout.controls, id) ? layout.controls[id] : undefined;
    if (!raw) continue;
    const offset = cleanOffset(raw);
    if (offset.dx !== 0 || offset.dy !== 0) controls[id] = offset;
  }
  const result: Layout = { version: LAYOUT_VERSION, groups, hidden };
  if (Object.keys(controls).length > 0) result.controls = controls;
  if (typeof layout.opacity === "number" && Number.isFinite(layout.opacity)) {
    const opacity = round(clamp(layout.opacity, MIN_OPACITY, 1), 2);
    if (opacity < 1) result.opacity = opacity;
  }
  return result;
}

export function isDefault(layout: Layout): boolean {
  const l = normalizeLayout(layout);
  return Object.keys(l.groups).length === 0 && l.hidden.length === 0 && l.controls === undefined && l.opacity === undefined;
}

export function adjustOf(layout: Layout, group: GroupId): GroupAdjust {
  const found = Object.hasOwn(layout.groups, group) ? layout.groups[group] : undefined;
  return found ? { ...found } : { ...NEUTRAL };
}

/** Muda só os campos de `patch` do grupo. Devolve um layout novo. */
export function withGroup(layout: Layout, group: GroupId, patch: Partial<GroupAdjust>): Layout {
  const next = { ...adjustOf(layout, group), ...patch };
  return normalizeLayout({ ...layout, groups: { ...layout.groups, [group]: next } });
}

/** Volta o grupo ao padrão: some o ajuste dele e o deslocamento individual dos controles dele. */
export function resetGroup(layout: Layout, group: GroupId): Layout {
  const groups = { ...layout.groups };
  delete groups[group];
  const controls = { ...layout.controls };
  for (const c of controlsOf(group)) delete controls[c];
  return normalizeLayout({ ...layout, groups, controls });
}

export function opacityOf(layout: Layout): number {
  return layout.opacity ?? 1;
}

/** Muda a opacidade de todos os botões (100% é o padrão e apaga o campo). Devolve um layout novo. */
export function withOpacity(layout: Layout, opacity: number): Layout {
  return normalizeLayout({ ...layout, opacity });
}

export function controlOffset(layout: Layout, control: ControlId): ControlAdjust {
  const found = layout.controls && Object.hasOwn(layout.controls, control) ? layout.controls[control] : undefined;
  return found ? { ...found } : { ...NO_OFFSET };
}

/** Move só o controle pedido (o grupo dele precisa estar destravado na tela). Devolve um layout novo. */
export function withControl(layout: Layout, control: ControlId, patch: Partial<ControlAdjust>): Layout {
  const next = { ...controlOffset(layout, control), ...patch };
  return normalizeLayout({ ...layout, controls: { ...layout.controls, [control]: next } });
}

export function isHidden(layout: Layout, control: ControlId): boolean {
  return layout.hidden.includes(control);
}

export function setHidden(layout: Layout, control: ControlId, hidden: boolean): Layout {
  const rest = layout.hidden.filter((c) => c !== control);
  return normalizeLayout({ ...layout, hidden: hidden ? [...rest, control] : rest });
}

export type ParseStatus = "ok" | "invalid" | "future-version";

export interface ParseResult {
  layout: Layout;
  status: ParseStatus;
}

const isObject = (v: unknown): v is Record<string, unknown> => typeof v === "object" && v !== null && !Array.isArray(v);

/**
 * Interpreta dados vindos do armazenamento (que podem estar velhos, corrompidos ou ser de uma
 * versão futura). Nunca lança: o que não presta vira o padrão, e entradas ruins são descartadas
 * uma a uma sem perder as boas.
 */
export function parseLayout(raw: unknown): ParseResult {
  if (!isObject(raw)) return { layout: defaultLayout(), status: "invalid" };
  const version = raw.version;
  if (typeof version !== "number" || !Number.isInteger(version) || version < 1) {
    return { layout: defaultLayout(), status: "invalid" };
  }
  if (version > LAYOUT_VERSION) return { layout: defaultLayout(), status: "future-version" };

  const groups: Layout["groups"] = {};
  const rawGroups = isObject(raw.groups) ? raw.groups : {};
  for (const id of GROUP_IDS) {
    const entry = Object.hasOwn(rawGroups, id) ? rawGroups[id] : undefined;
    if (!isObject(entry)) continue;
    const campos = [entry.dx, entry.dy, entry.scale];
    if (campos.some((v) => v !== undefined && typeof v !== "number")) continue; // entrada ruim: descarta só ela
    groups[id] = { dx: entry.dx as number, dy: entry.dy as number, scale: entry.scale as number };
  }
  const hidden = Array.isArray(raw.hidden)
    ? raw.hidden.filter((c): c is ControlId => typeof c === "string" && (CONTROL_IDS as readonly string[]).includes(c))
    : [];
  const controls: NonNullable<Layout["controls"]> = {};
  const rawControls = isObject(raw.controls) ? raw.controls : {};
  for (const id of CONTROL_IDS) {
    const entry = Object.hasOwn(rawControls, id) ? rawControls[id] : undefined;
    if (!isObject(entry)) continue;
    if ([entry.dx, entry.dy].some((v) => v !== undefined && typeof v !== "number")) continue;
    controls[id] = { dx: entry.dx as number, dy: entry.dy as number };
  }
  const opacity = typeof raw.opacity === "number" ? raw.opacity : undefined;
  return { layout: normalizeLayout({ version: LAYOUT_VERSION, groups, hidden, controls, opacity }), status: "ok" };
}

export function serializeLayout(layout: Layout): string {
  return JSON.stringify(normalizeLayout(layout));
}
