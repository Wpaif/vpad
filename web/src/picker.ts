import { PROFILES } from "./profiles";

export interface ProfilePicker {
  /** Perfil ativo segundo o servidor; null quando não há conexão. */
  setCurrent: (index: number | null) => void;
  /** O PC está esperando o dono confirmar o pedido: o botão fica pendente por mais tempo. */
  waitingForPc: () => void;
  /** O pedido foi recusado ou expirou: o botão volta ao normal. */
  cancelPending: () => void;
}

const PENDING_TIMEOUT_MS = 4000;
/** Quanto o botão espera a confirmação no PC (o PC desiste antes, em 60 s). */
const PC_WAIT_TIMEOUT_MS = 90_000;

/**
 * Seletor de perfil. A escolha só vale quando o servidor confirma: até lá o botão fica
 * "pendente" e o perfil ativo continua o antigo (a troca pode falhar no servidor).
 */
export function mountProfilePicker(container: HTMLElement, onPick: (index: number) => void): ProfilePicker {
  let current: number | null = null;
  let pendingTimer: ReturnType<typeof setTimeout> | undefined;

  const buttons = PROFILES.map((profile, index) => {
    const button = document.createElement("button");
    button.type = "button";
    button.textContent = profile.short;
    button.setAttribute("aria-label", profile.label);
    button.addEventListener("click", () => {
      if (current === null || index === current) return;
      clearPending();
      button.setAttribute("data-pending", "");
      pendingTimer = setTimeout(clearPending, PENDING_TIMEOUT_MS);
      onPick(index);
    });
    return button;
  });
  container.replaceChildren(...buttons);

  function clearPending() {
    clearTimeout(pendingTimer);
    buttons.forEach((b) => b.removeAttribute("data-pending"));
  }

  function render() {
    buttons.forEach((b, i) => {
      b.setAttribute("aria-pressed", String(i === current));
      b.disabled = current === null;
    });
  }

  render();
  return {
    setCurrent(index) {
      current = index;
      clearPending();
      render();
    },
    waitingForPc() {
      clearTimeout(pendingTimer);
      pendingTimer = setTimeout(clearPending, PC_WAIT_TIMEOUT_MS);
    },
    cancelPending: clearPending,
  };
}
