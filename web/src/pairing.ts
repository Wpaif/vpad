import { TOKEN_FORMAT } from "./auth";
import { t } from "./i18n";

type FetchFn = typeof fetch;

export class PairingError extends Error {
  /** segundos até poder tentar de novo, quando o servidor bloqueou por excesso de erros */
  retryAfter?: number;
}

export interface Paired {
  token: string;
  /** id público do aparelho, para o dono do PC liberar o modo teclado */
  id: string;
}

const DEVICE_ID = /^[0-9a-f]{8}$/;

/**
 * Troca o código de 6 dígitos do terminal por um token exclusivo deste aparelho. A resposta
 * só chega depois que quem está no PC aprova o pedido (ele aparece no terminal).
 */
export async function pairDevice(code: string, name: string, fetchFn: FetchFn = fetch): Promise<Paired> {
  let res: Response;
  try {
    res = await fetchFn("/api/parear", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ codigo: code, nome: name }),
    });
  } catch {
    throw new PairingError(t("pairing.error.offline"));
  }
  const body = (await res.json().catch(() => ({}))) as {
    token?: unknown;
    id?: unknown;
    segundos?: unknown;
    codigo?: unknown;
  };

  if (res.status === 401) {
    throw new PairingError(t("pairing.error.wrongCode"));
  }
  if (res.status === 403) {
    if (body.codigo === "pareamento_fechado") {
      throw new PairingError(t("pairing.error.closed"));
    }
    throw new PairingError(t("pairing.error.denied"));
  }
  if (res.status === 408) {
    throw new PairingError(t("pairing.error.expired"));
  }
  if (res.status === 409) {
    throw new PairingError(t("pairing.error.busy"));
  }
  if (res.status === 429) {
    const seconds = Number(body.segundos ?? res.headers.get("Retry-After")) || 60;
    const error = new PairingError(t("pairing.error.rateLimited", { seconds }));
    error.retryAfter = seconds;
    throw error;
  }
  if (!res.ok) throw new PairingError(t("pairing.error.http", { status: res.status }));
  if (typeof body.token !== "string" || !TOKEN_FORMAT.test(body.token)) {
    throw new PairingError(t("pairing.error.badResponse"));
  }
  const id = typeof body.id === "string" && DEVICE_ID.test(body.id) ? body.id : "";
  return { token: body.token, id };
}

/** Nome curto que identifica este aparelho na lista do servidor. */
export function deviceName(userAgent: string, touchPoints = 0): string {
  if (/iPhone/.test(userAgent)) return "iPhone";
  if (/iPad/.test(userAgent)) return "iPad";
  // o iPadOS se apresenta como Mac, mas tem tela de toque
  if (/Macintosh/.test(userAgent)) return touchPoints > 1 ? "iPad" : "Mac";
  if (/Android/.test(userAgent)) return "Android";
  if (/Windows/.test(userAgent)) return "Windows";
  if (/Linux|X11/.test(userAgent)) return "Linux";
  return "dispositivo";
}

export interface PairingDeps {
  pair: (code: string) => Promise<Paired>;
  onPaired: (paired: Paired) => void;
}

export interface Pairing {
  show: () => void;
  hide: () => void;
}

const CODE_LENGTH = 6;

/** Tela de pareamento: um campo de seis dígitos e o botão que o envia. */
export function mountPairing(root: HTMLElement, deps: PairingDeps): Pairing {
  const form = root.querySelector<HTMLFormElement>("form")!;
  const input = root.querySelector<HTMLInputElement>("input")!;
  const submit = root.querySelector<HTMLButtonElement>("button[type=submit]")!;
  const alertEl = root.querySelector<HTMLElement>("[role=alert]")!;

  let busy = false;
  let blocked = false;

  const sync = () => {
    submit.disabled = busy || blocked || input.value.length !== CODE_LENGTH;
  };

  input.addEventListener("input", () => {
    input.value = input.value.replace(/\D/g, "").slice(0, CODE_LENGTH);
    sync();
  });

  form.addEventListener("submit", async (event) => {
    event.preventDefault();
    if (busy || blocked || input.value.length !== CODE_LENGTH) return;
    busy = true;
    // com o código certo o pedido vai para o terminal do PC e a resposta só sai depois do "sim"
    alertEl.textContent = t("pairing.waiting");
    sync();
    try {
      const paired = await deps.pair(input.value);
      alertEl.textContent = "";
      deps.onPaired(paired);
    } catch (error) {
      alertEl.textContent = error instanceof Error ? error.message : t("pairing.error.fallback");
      input.value = "";
      const wait = (error as PairingError).retryAfter;
      if (wait) {
        blocked = true;
        setTimeout(() => {
          blocked = false;
          sync();
        }, wait * 1000);
      }
    } finally {
      busy = false;
      sync();
    }
  });

  sync();
  return {
    show() {
      root.hidden = false;
      input.focus();
    },
    hide() {
      root.hidden = true;
    },
  };
}
