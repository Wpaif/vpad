import { MAX_SCALE, MIN_SCALE } from "./layoutModel";

/** Geometria do arrastar: limites da tela e ímã nas bordas/centros de outros grupos. Tudo em px. */

export interface Rect {
  left: number;
  top: number;
  right: number;
  bottom: number;
}

export interface Shift {
  x: number;
  y: number;
}

export interface Snapped extends Shift {
  /** coordenada da linha de alinhamento a desenhar, ou null se não grudou */
  guideX: number | null;
  guideY: number | null;
}

const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

/** Faz `rect` deslocado por `shift` caber em `bounds`. Se for maior, fica alinhado à esquerda/topo. */
export function limitShift(rect: Rect, shift: Shift, bounds: Rect): Shift {
  const axis = (lo: number, hi: number, bLo: number, bHi: number, s: number) =>
    hi - lo >= bHi - bLo ? bLo - lo : clamp(s, bLo - lo, bHi - hi);
  return {
    x: axis(rect.left, rect.right, bounds.left, bounds.right, shift.x),
    y: axis(rect.top, rect.bottom, bounds.top, bounds.bottom, shift.y),
  };
}

/** Melhor alinhamento (borda ou centro do retângulo nas bordas/centro de um alvo) dentro do limiar. */
function snapAxis(lo: number, hi: number, targets: Array<[number, number]>, threshold: number) {
  let best: { delta: number; guide: number } | null = null;
  const mine = [lo, (lo + hi) / 2, hi];
  for (const [tLo, tHi] of targets) {
    for (const t of [tLo, (tLo + tHi) / 2, tHi]) {
      for (const m of mine) {
        const delta = t - m;
        if (Math.abs(delta) <= threshold && (!best || Math.abs(delta) < Math.abs(best.delta))) {
          best = { delta, guide: t };
        }
      }
    }
  }
  return best;
}

/** Gruda o retângulo deslocado nos alvos próximos, cada eixo por si. `threshold` 0 desliga. */
export function snapShift(rect: Rect, shift: Shift, targets: Rect[], threshold: number): Snapped {
  if (threshold <= 0) return { ...shift, guideX: null, guideY: null };
  const moved = {
    left: rect.left + shift.x,
    right: rect.right + shift.x,
    top: rect.top + shift.y,
    bottom: rect.bottom + shift.y,
  };
  const sx = snapAxis(moved.left, moved.right, targets.map((t) => [t.left, t.right]), threshold);
  const sy = snapAxis(moved.top, moved.bottom, targets.map((t) => [t.top, t.bottom]), threshold);
  return {
    x: shift.x + (sx?.delta ?? 0),
    y: shift.y + (sy?.delta ?? 0),
    guideX: sx?.guide ?? null,
    guideY: sy?.guide ?? null,
  };
}

/** Menor lado, em px, que um controle pode ter na tela. */
export const MIN_TARGET_PX = 44;

export interface Point {
  x: number;
  y: number;
}

/** Quanto o toque se afastou (ou aproximou) do centro; vira o fator de escala do grupo. */
export function scaleRatio(center: Point, from: Point, to: Point): number {
  const dist = (p: Point) => Math.hypot(p.x - center.x, p.y - center.y);
  return dist(to) / Math.max(dist(from), 1);
}

/**
 * Escala mínima de um grupo: o menor controle não fica abaixo de 44 px, nem abaixo do que já é
 * por padrão (um controle de 34 px não pode encolher), e nunca abaixo do piso global.
 * `smallestPx` é o menor lado do menor controle com o grupo em escala 1.
 */
export function minScaleFor(smallestPx: number, absoluteMin = MIN_SCALE): number {
  if (smallestPx <= 0) return absoluteMin;
  return Math.max(absoluteMin, Math.min(MIN_TARGET_PX / smallestPx, 1));
}

/**
 * Limita a escala pedida. A escala cresce em torno do centro, que não se move; por isso o
 * máximo que cabe na tela vem da distância do centro a cada borda. `rect` é o retângulo
 * medido agora, já na escala `current`.
 */
export function limitScale(
  scale: number,
  rect: Rect,
  current: number,
  bounds: Rect,
  min: number,
  max = MAX_SCALE,
): number {
  const halfW = (rect.right - rect.left) / 2 / current;
  const halfH = (rect.bottom - rect.top) / 2 / current;
  const cx = (rect.left + rect.right) / 2;
  const cy = (rect.top + rect.bottom) / 2;
  const fit = Math.min(
    halfW > 0 ? Math.min(cx - bounds.left, bounds.right - cx) / halfW : Infinity,
    halfH > 0 ? Math.min(cy - bounds.top, bounds.bottom - cy) / halfH : Infinity,
  );
  return clamp(scale, min, Math.max(min, Math.min(max, fit)));
}
