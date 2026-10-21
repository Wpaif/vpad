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
  document.addEventListener("touchmove", (e) => e.preventDefault(), { passive: false });
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
