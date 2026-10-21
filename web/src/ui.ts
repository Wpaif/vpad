import type { LatencySnapshot } from "./latency";
import type { Refusal } from "./protocol";
import { sparklinePoints } from "./sparkline";

export function bindVisibilityToggle(toggle: HTMLElement, pad: HTMLElement): void {
  toggle.setAttribute("aria-pressed", "false");
  toggle.addEventListener("click", () => {
    const hidden = pad.toggleAttribute("data-hidden");
    toggle.setAttribute("aria-pressed", String(hidden));
  });
}

/** Evita zoom, seleção e menus do Safari durante o uso. */
export function lockGestures(): void {
  for (const type of ["gesturestart", "gesturechange", "dblclick", "contextmenu"]) {
    document.addEventListener(type, (e) => e.preventDefault());
  }
  document.addEventListener(
    "touchmove",
    (e) => {
      // o editor de teclas é uma lista que precisa rolar
      if (e.target instanceof Element && e.target.closest("[data-scroll]")) return;
      e.preventDefault();
    },
    { passive: false },
  );
}

const LABELS = {
  connecting: "conectando…",
  open: "conectado",
  reconnecting: "reconectando…",
  closed: "offline",
} as const;

export function renderStatus(el: HTMLElement, status: keyof typeof LABELS, latencyMs: number | null): void {
  el.dataset.status = status;
  el.textContent = status === "open" && latencyMs !== null ? `${latencyMs} ms` : LABELS[status];
}

const fmtMs = (v: number | null) => (v === null ? "–" : `${v} ms`);

export function renderStats(panel: HTMLElement, s: LatencySnapshot): void {
  const set = (name: string, text: string) => {
    const el = panel.querySelector(`[data-stat=${name}]`);
    if (el) el.textContent = text;
  };
  set("last", fmtMs(s.last));
  set("avg", fmtMs(s.avg));
  set("min", fmtMs(s.min));
  set("max", fmtMs(s.max));
  set("jitter", fmtMs(s.jitter));
  set("loss", `${Math.round(s.loss * 100)}%`);
  const ceiling = Math.max(50, ...s.samples);
  panel
    .querySelector("[data-stat=spark]")
    ?.setAttribute("points", sparklinePoints(s.samples, 120, 32, ceiling));
}

export function bindPanelToggle(trigger: HTMLElement, panel: HTMLElement, onChange?: (open: boolean) => void): void {
  trigger.addEventListener("click", () => {
    panel.hidden = !panel.hidden;
    onChange?.(!panel.hidden);
  });
}

const GATE_MESSAGES: Record<Refusal, string> = {
  busy: "Outro aparelho está controlando o PC. Desconecte-o (ou espere ele sair) e tente de novo.",
  unauthorized: "Acesso não autorizado. Escaneie o QR code mostrado no terminal do PC para abrir esta página.",
};

/** Mostra por que o servidor recusou este aparelho; null esconde a tela. */
export function renderGate(gate: HTMLElement, reason: Refusal | null): void {
  gate.hidden = reason === null;
  if (reason === null) {
    delete gate.dataset.reason;
    return;
  }
  gate.dataset.reason = reason;
  const message = gate.querySelector("[data-role=message]");
  if (message) message.textContent = GATE_MESSAGES[reason];
}
