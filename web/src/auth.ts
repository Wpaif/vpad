/**
 * Acesso ao servidor: o token vem no QR code do terminal (`?t=...`) e é guardado no
 * aparelho; a URL continua com ele (ver `readToken`). Cada aparelho também tem um id próprio, para o servidor
 * reconhecê-lo quando reconecta (ele solta a conexão antiga em vez de achar que é um intruso).
 */

type Store = Pick<Storage, "getItem" | "setItem">;

const TOKEN_KEY = "vpad:token";
const CLIENT_KEY = "vpad:cliente";
const TOKEN_FORMAT = /^[0-9a-f]{32}$/i;
const CLIENT_FORMAT = /^[0-9a-f]{32}$/;

function read(storage: Store, key: string): string | null {
  try {
    return storage.getItem(key);
  } catch {
    return null;
  }
}

function write(storage: Store, key: string, value: string) {
  try {
    storage.setItem(key, value);
  } catch {
    // armazenamento bloqueado: vale só nesta aba
  }
}

/** localStorage, ou uma memória simples se o navegador o bloquear (navegação privada). */
export function safeStorage(): Store {
  try {
    const storage = window.localStorage;
    storage.getItem(TOKEN_KEY);
    return storage;
  } catch {
    const memory = new Map<string, string>();
    return { getItem: (k) => memory.get(k) ?? null, setItem: (k, v) => void memory.set(k, v) };
  }
}

/**
 * Token da URL (que então é guardado) ou o guardado antes. A URL mantém o `?t=...` de
 * propósito: é ela que o iOS grava no atalho da Tela de Início, e o app instalado tem
 * armazenamento próprio, separado do Safari, então só a URL leva o token até ele.
 */
export function readToken(loc: Location, storage: Store): string | null {
  const part = loc.search
    .replace(/^\?/, "")
    .split("&")
    .find((p) => p === "t" || p.startsWith("t="));

  if (part !== undefined) {
    const fromUrl = decodeURIComponent(part.slice(2));
    if (TOKEN_FORMAT.test(fromUrl)) {
      write(storage, TOKEN_KEY, fromUrl);
      return fromUrl;
    }
  }

  const saved = read(storage, TOKEN_KEY);
  return saved !== null && TOKEN_FORMAT.test(saved) ? saved : null;
}

const randomBytes = (n: number) => crypto.getRandomValues(new Uint8Array(n));

/** Id estável deste aparelho (128 bits aleatórios). */
export function getClientId(storage: Store, random: (n: number) => Uint8Array = randomBytes): string {
  const saved = read(storage, CLIENT_KEY);
  if (saved !== null && CLIENT_FORMAT.test(saved)) return saved;
  const id = [...random(16)].map((b) => b.toString(16).padStart(2, "0")).join("");
  write(storage, CLIENT_KEY, id);
  return id;
}

export function wsUrl(loc: Location, token: string, clientId: string): string {
  const scheme = loc.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${loc.host}/ws?t=${encodeURIComponent(token)}&id=${encodeURIComponent(clientId)}`;
}
