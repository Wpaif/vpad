// Protocolo binário celular -> PC (ver servidor).
export enum Button {
  A, B, X, Y, LB, RB, Select, Start, L3, R3, Up, Down, Left, Right,
}

export enum Axis {
  LX, LY, RX, RY, LT, RT,
}

const MSG_BUTTON = 1;
const MSG_AXIS = 2;
const MSG_PING = 3;
const MSG_PROFILE = 4;
const MSG_REFUSAL = 5;

const clamp = (v: number, min: number, max: number) => Math.min(max, Math.max(min, v));

export function encodeButton(id: Button, down: boolean): Uint8Array<ArrayBuffer> {
  return Uint8Array.of(MSG_BUTTON, id, down ? 1 : 0);
}

export function encodeAxis(id: Axis, value: number): Uint8Array<ArrayBuffer> {
  const trigger = id === Axis.LT || id === Axis.RT;
  const v = Math.round(trigger ? clamp(value, 0, 255) : clamp(value, -32768, 32767));
  const out = new Uint8Array(4);
  out[0] = MSG_AXIS;
  out[1] = id;
  new DataView(out.buffer).setInt16(2, v, true);
  return out;
}

export function encodePing(timestamp: number): Uint8Array<ArrayBuffer> {
  const out = new Uint8Array(9);
  out[0] = MSG_PING;
  new DataView(out.buffer).setFloat64(1, timestamp, true);
  return out;
}

export function decodePing(data: Uint8Array): number | null {
  if (data.length !== 9 || data[0] !== MSG_PING) return null;
  return new DataView(data.buffer, data.byteOffset).getFloat64(1, true);
}

/** Pede ao servidor para usar o perfil de índice `index` (ver profiles.ts). */
export function encodeProfile(index: number): Uint8Array<ArrayBuffer> {
  return Uint8Array.of(MSG_PROFILE, index);
}

/** Índice do perfil que o servidor informa estar ativo, ou null se não for esse tipo de mensagem. */
export function decodeProfile(data: Uint8Array): number | null {
  return data.length === 2 && data[0] === MSG_PROFILE ? data[1]! : null;
}

export type Refusal = "busy" | "unauthorized";

/** Motivo pelo qual o servidor recusou a conexão, ou null se a mensagem for outra. */
export function decodeRefusal(data: Uint8Array): Refusal | null {
  if (data.length !== 2 || data[0] !== MSG_REFUSAL) return null;
  return data[1] === 1 ? "busy" : data[1] === 2 ? "unauthorized" : null;
}
