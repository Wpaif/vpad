const MAX = 32767;

export interface StickState {
  /** valores do eixo, -32767..32767 */
  x: number;
  y: number;
  /** posição visual do botão, em px relativos ao centro */
  knobX: number;
  knobY: number;
}

/** Converte o deslocamento do dedo (px) em eixo do analógico, com zona morta radial. */
export function stickFromOffset(dx: number, dy: number, radius: number, deadzone = 0): StickState {
  const dist = Math.hypot(dx, dy);
  if (dist === 0) return { x: 0, y: 0, knobX: 0, knobY: 0 };

  const limited = Math.min(dist, radius);
  const ux = dx / dist;
  const uy = dy / dist;

  const norm = limited / radius;
  const scaled = norm <= deadzone ? 0 : (norm - deadzone) / (1 - deadzone);

  return {
    x: Math.round(ux * scaled * MAX) || 0,
    y: Math.round(uy * scaled * MAX) || 0,
    knobX: ux * limited,
    knobY: uy * limited,
  };
}
