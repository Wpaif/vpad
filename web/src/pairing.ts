import { TOKEN_FORMAT } from "./auth";

type FetchFn = typeof fetch;

export class PairingError extends Error {
  /** segundos até poder tentar de novo, quando o servidor bloqueou por excesso de erros */
  retryAfter?: number;
}

/** Troca o código de 6 dígitos do terminal por um token exclusivo deste aparelho. */
export async function pairDevice(code: string, name: string, fetchFn: FetchFn = fetch): Promise<string> {
  let res: Response;
  try {
    res = await fetchFn("/api/parear", {
      method: "POST",
      headers: { "Content-Type": "application/json" },
      body: JSON.stringify({ codigo: code, nome: name }),
    });
  } catch {
    throw new PairingError("Sem conexão com o PC.");
  }
  const body = (await res.json().catch(() => ({}))) as { token?: unknown; segundos?: unknown };

  if (res.status === 401) {
    throw new PairingError("Código incorreto ou expirado. Confira o código que aparece no terminal do PC.");
  }
  if (res.status === 429) {
    const seconds = Number(body.segundos ?? res.headers.get("Retry-After")) || 60;
    const error = new PairingError(`Muitas tentativas erradas. Espere ${seconds} s.`);
    error.retryAfter = seconds;
    throw error;
  }
  if (!res.ok) throw new PairingError(`Não consegui parear (${res.status}).`);
  if (typeof body.token !== "string" || !TOKEN_FORMAT.test(body.token)) {
    throw new PairingError("Resposta inesperada do servidor.");
  }
  return body.token;
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
  pair: (code: string) => Promise<string>;
  onPaired: (token: string) => void;
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
    alertEl.textContent = "";
    sync();
    try {
      deps.onPaired(await deps.pair(input.value));
    } catch (error) {
      alertEl.textContent = error instanceof Error ? error.message : "Não consegui parear.";
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
