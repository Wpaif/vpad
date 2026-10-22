import { Axis, Button, encodeAxis, encodeButton } from "./protocol";
import { stickFromOffset } from "./stick";

type Send = (msg: Uint8Array<ArrayBuffer>) => void;

const DEADZONE = 0.12;

const STICKS = {
  left: [Axis.LX, Axis.LY],
  right: [Axis.RX, Axis.RY],
} as const;

/** Liga botões, gatilhos e analógicos (marcados com data-*) ao `send`. */
export function bindController(root: HTMLElement, send: Send): void {
  bindButtons(root, send);
  root.querySelectorAll<HTMLElement>("[data-grupo=direcional]").forEach((el) => bindDpad(el, send));
  bindTriggers(root, send);
  root.querySelectorAll<HTMLElement>("[data-stick]").forEach((el) => bindStick(el, send));
}

/** Controle oculto pelo layout, ou pad em modo edição: o toque não vale como entrada. */
const blocked = (el: Element) => el.classList.contains("oculto") || el.closest("[data-editando]") !== null;

function capture(el: Element, id: number) {
  try {
    el.setPointerCapture?.(id);
  } catch {
    // ponteiro já liberado
  }
}

function bindHold(el: HTMLElement, press: () => void, release: () => void) {
  const active = new Set<number>();
  el.addEventListener("pointerdown", (e) => {
    if (blocked(el) || active.has(e.pointerId)) return;
    active.add(e.pointerId);
    capture(el, e.pointerId);
    if (active.size === 1) {
      el.setAttribute("data-pressed", "");
      press();
    }
  });
  const up = (e: PointerEvent) => {
    if (!active.delete(e.pointerId) || active.size > 0) return;
    el.removeAttribute("data-pressed");
    release();
  };
  el.addEventListener("pointerup", up);
  el.addEventListener("pointercancel", up);
}

function bindButtons(root: HTMLElement, send: Send) {
  root.querySelectorAll<HTMLElement>("[data-btn]").forEach((el) => {
    if (el.closest("[data-grupo=direcional]")) return; // o direcional trata o deslize
    const id = Button[el.dataset.btn as keyof typeof Button];
    bindHold(el, () => send(encodeButton(id, true)), () => send(encodeButton(id, false)));
  });
}

/** Direcional: o dedo pode deslizar de uma seta para outra sem levantar, como num controle. */
function bindDpad(group: HTMLElement, send: Send) {
  const arrows = [...group.querySelectorAll<HTMLElement>("[data-btn]")];
  const fingers = new Map<number, HTMLElement | null>();
  const holders = (el: HTMLElement) => [...fingers.values()].filter((v) => v === el).length;

  const arrowAt = (x: number, y: number) =>
    arrows.find((el) => {
      if (el.classList.contains("oculto")) return false;
      const r = el.getBoundingClientRect();
      return x >= r.left && x < r.right && y >= r.top && y < r.bottom;
    }) ?? null;

  const set = (id: number, next: HTMLElement | null) => {
    const prev = fingers.get(id) ?? null;
    if (prev === next) return;
    fingers.set(id, next);
    if (prev && holders(prev) === 0) {
      prev.removeAttribute("data-pressed");
      send(encodeButton(Button[prev.dataset.btn as keyof typeof Button], false));
    }
    if (next && holders(next) === 1) {
      next.setAttribute("data-pressed", "");
      send(encodeButton(Button[next.dataset.btn as keyof typeof Button], true));
    }
  };

  group.addEventListener("pointerdown", (e) => {
    if (blocked(group) || fingers.has(e.pointerId)) return;
    fingers.set(e.pointerId, null);
    capture(group, e.pointerId);
    set(e.pointerId, arrowAt(e.clientX, e.clientY));
  });
  group.addEventListener("pointermove", (e) => {
    if (fingers.has(e.pointerId)) set(e.pointerId, arrowAt(e.clientX, e.clientY));
  });
  const up = (e: PointerEvent) => {
    if (!fingers.has(e.pointerId)) return;
    set(e.pointerId, null);
    fingers.delete(e.pointerId);
  };
  group.addEventListener("pointerup", up);
  group.addEventListener("pointercancel", up);
}

function bindTriggers(root: HTMLElement, send: Send) {
  root.querySelectorAll<HTMLElement>("[data-trigger]").forEach((el) => {
    const id = Axis[el.dataset.trigger as keyof typeof Axis];
    bindHold(el, () => send(encodeAxis(id, 255)), () => send(encodeAxis(id, 0)));
  });
}

function bindStick(el: HTMLElement, send: Send) {
  const [axisX, axisY] = STICKS[el.dataset.stick as keyof typeof STICKS];
  const knob = el.querySelector<HTMLElement>(".knob");
  let owner: number | null = null;

  const update = (e: PointerEvent) => {
    const r = el.getBoundingClientRect();
    const radius = r.width / 2;
    // o botão interno vive dentro do grupo, que o layout pode ter escalado: px de tela viram px locais
    const k = el.offsetWidth > 0 ? r.width / el.offsetWidth : 1;
    const s = stickFromOffset(
      e.clientX - (r.left + radius),
      e.clientY - (r.top + r.height / 2),
      radius,
      DEADZONE,
    );
    if (knob) knob.style.transform = `translate(${s.knobX / k}px, ${s.knobY / k}px)`;
    send(encodeAxis(axisX, s.x));
    send(encodeAxis(axisY, s.y));
  };

  el.addEventListener("pointerdown", (e) => {
    if (blocked(el) || owner !== null) return;
    owner = e.pointerId;
    capture(el, e.pointerId);
    el.setAttribute("data-pressed", "");
    update(e);
  });
  el.addEventListener("pointermove", (e) => {
    if (e.pointerId === owner) update(e);
  });
  const end = (e: PointerEvent) => {
    if (e.pointerId !== owner) return;
    owner = null;
    el.removeAttribute("data-pressed");
    if (knob) knob.style.transform = "translate(0px, 0px)";
    send(encodeAxis(axisX, 0));
    send(encodeAxis(axisY, 0));
  };
  el.addEventListener("pointerup", end);
  el.addEventListener("pointercancel", end);
}
