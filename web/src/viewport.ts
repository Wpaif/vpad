const ROTATION_RECHECK_MS = [100, 400];

/**
 * Mantém --app-h igual à altura real da janela. No iOS as unidades dvh/100% ficam
 * desatualizadas depois de girar o celular, deixando uma faixa sem pintar embaixo.
 */
export function bindAppHeight(win: Window, root: HTMLElement): () => void {
  const update = () => root.style.setProperty("--app-h", `${win.innerHeight}px`);
  let timers: ReturnType<typeof setTimeout>[] = [];

  // O iOS ainda reporta a altura antiga no próprio evento de rotação.
  const onRotate = () => {
    timers.forEach(clearTimeout);
    timers = ROTATION_RECHECK_MS.map((ms) => setTimeout(update, ms));
  };

  update();
  win.addEventListener("resize", update);
  win.addEventListener("orientationchange", onRotate);
  return () => {
    win.removeEventListener("resize", update);
    win.removeEventListener("orientationchange", onRotate);
    timers.forEach(clearTimeout);
  };
}

export interface ViewportInfo {
  inner: [number, number];
  screen: [number, number];
  safe: { top: number; right: number; bottom: number; left: number };
  standalone: boolean;
}

export function formatViewportInfo({ inner, screen, safe, standalone }: ViewportInfo): string {
  return [
    `janela ${inner[0]}×${inner[1]}`,
    `tela ${screen[0]}×${screen[1]}`,
    `seguro ${safe.top}/${safe.right}/${safe.bottom}/${safe.left}`,
    `app ${standalone ? "sim" : "não"}`,
  ].join("\n");
}

function readSafeArea(doc: Document): ViewportInfo["safe"] {
  const probe = doc.createElement("div");
  probe.style.cssText =
    "position:fixed;visibility:hidden;pointer-events:none;" +
    "padding:env(safe-area-inset-top) env(safe-area-inset-right) env(safe-area-inset-bottom) env(safe-area-inset-left)";
  doc.body.append(probe);
  const s = getComputedStyle(probe);
  const px = (v: string) => Math.round(parseFloat(v) || 0);
  const safe = {
    top: px(s.paddingTop),
    right: px(s.paddingRight),
    bottom: px(s.paddingBottom),
    left: px(s.paddingLeft),
  };
  probe.remove();
  return safe;
}

/** Painel de diagnóstico (abrir a página com ?debug) para ver o que o iPhone realmente reporta. */
export function showViewportDebug(doc: Document, win: Window, standalone: boolean): void {
  const out = doc.createElement("pre");
  out.id = "debug";
  doc.body.append(out);
  const refresh = () => {
    out.textContent = formatViewportInfo({
      inner: [win.innerWidth, win.innerHeight],
      screen: [win.screen.width, win.screen.height],
      safe: readSafeArea(doc),
      standalone,
    });
  };
  refresh();
  win.addEventListener("resize", refresh);
  win.addEventListener("orientationchange", () => setTimeout(refresh, 400));
}
