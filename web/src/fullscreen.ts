/**
 * Tela cheia nativa só existe fora do Safari do iPhone, e mesmo lá o navegador
 * exige um gesto do usuário: por isso o pedido acontece no toque, não ao girar.
 */
export function enterFullscreenOnTouch(doc: Document): () => void {
  const onTouch = () => {
    if (!doc.fullscreenEnabled || doc.fullscreenElement) return;
    doc.documentElement.requestFullscreen({ navigationUI: "hide" }).catch(() => {});
  };
  doc.addEventListener("pointerdown", onTouch, { capture: true });
  return () => doc.removeEventListener("pointerdown", onTouch, { capture: true });
}

export interface InstallEnv {
  /** true quando aberto pelo ícone da tela inicial (navigator.standalone / display-mode) */
  standalone: boolean;
  fullscreenEnabled: boolean;
}

export function shouldShowInstallHint(env: InstallEnv & { dismissed: boolean }): boolean {
  return !env.standalone && !env.fullscreenEnabled && !env.dismissed;
}

const DISMISSED_KEY = "vpad:dica-instalar";

type Storage = Pick<globalThis.Storage, "getItem" | "setItem">;

/** No iPhone o único caminho para tela cheia é instalar na tela inicial; a dica explica isso. */
export function bindInstallHint(hint: HTMLElement, close: HTMLElement, env: InstallEnv, storage: Storage): void {
  let dismissed = false;
  try {
    dismissed = storage.getItem(DISMISSED_KEY) !== null;
  } catch {
    // armazenamento bloqueado: a dica só reaparece na próxima visita
  }

  hint.hidden = !shouldShowInstallHint({ ...env, dismissed });

  close.addEventListener("click", () => {
    hint.hidden = true;
    try {
      storage.setItem(DISMISSED_KEY, "1");
    } catch {
      // idem
    }
  });
}
