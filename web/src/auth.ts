/**
 * Acesso ao servidor: cada aparelho recebe, ao parear com o código do terminal, um token só
 * dele, que fica guardado aqui. O app da Tela de Início do iOS tem armazenamento próprio,
 * separado do Safari, então ele é pareado por conta própria, dentro do app.
 */

type Store = Pick<Storage, "getItem" | "setItem" | "removeItem">;

const TOKEN_KEY = "vpad:token";
const DEVICE_ID_KEY = "vpad:id";
const DEVICE_ID_FORMAT = /^[0-9a-f]{8}$/;
export const TOKEN_FORMAT = /^[0-9a-f]{32}$/i;

/** localStorage, ou uma memória simples se o navegador o bloquear (navegação privada). */
export function safeStorage(): Store {
  try {
    const storage = window.localStorage;
    storage.getItem(TOKEN_KEY);
    return storage;
  } catch {
    const memory = new Map<string, string>();
    return {
      getItem: (k) => memory.get(k) ?? null,
      setItem: (k, v) => void memory.set(k, v),
      removeItem: (k) => void memory.delete(k),
    };
  }
}

/** Token deste aparelho, ou null se ele ainda não foi pareado. */
export function getToken(storage: Store): string | null {
  try {
    const saved = storage.getItem(TOKEN_KEY);
    return saved !== null && TOKEN_FORMAT.test(saved) ? saved : null;
  } catch {
    return null;
  }
}

export function saveToken(storage: Store, token: string): void {
  try {
    storage.setItem(TOKEN_KEY, token);
  } catch {
    // armazenamento bloqueado: vale só nesta aba
  }
}

/** Esquece o token e o id (por exemplo, quando o servidor revogou este aparelho). */
export function clearToken(storage: Store): void {
  for (const key of [TOKEN_KEY, DEVICE_ID_KEY]) {
    try {
      storage.removeItem(key);
    } catch {
      // armazenamento bloqueado
    }
  }
}

/** Id público deste aparelho na lista do servidor (não é segredo): o dono do PC o usa para liberar o teclado. */
export function getDeviceId(storage: Store): string | null {
  try {
    const saved = storage.getItem(DEVICE_ID_KEY);
    return saved !== null && DEVICE_ID_FORMAT.test(saved) ? saved : null;
  } catch {
    return null;
  }
}

export function saveDeviceId(storage: Store, id: string): void {
  try {
    storage.setItem(DEVICE_ID_KEY, id);
  } catch {
    // vale só nesta aba
  }
}

/** A URL do WebSocket, sem segredo: o token vai no subprotocolo (ver `wsProtocols`). */
export function wsUrl(loc: Location): string {
  const scheme = loc.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${loc.host}/ws`;
}

/**
 * O token viaja como subprotocolo, fora da URL: assim ele não fica no histórico do navegador,
 * no Referer nem nos logs do servidor. O primeiro valor é o protocolo que o servidor escolhe.
 */
export function wsProtocols(token: string): string[] {
  return ["vpad", `vpad.${token}`];
}
