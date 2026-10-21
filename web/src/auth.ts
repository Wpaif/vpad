/**
 * Acesso ao servidor: cada aparelho recebe, ao parear com o código do terminal, um token só
 * dele, que fica guardado aqui. O app da Tela de Início do iOS tem armazenamento próprio,
 * separado do Safari, então ele é pareado por conta própria, dentro do app.
 */

type Store = Pick<Storage, "getItem" | "setItem" | "removeItem">;

const TOKEN_KEY = "vpad:token";
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

/** Esquece o token (por exemplo, quando o servidor o revogou). */
export function clearToken(storage: Store): void {
  try {
    storage.removeItem(TOKEN_KEY);
  } catch {
    // idem
  }
}

export function wsUrl(loc: Location, token: string): string {
  const scheme = loc.protocol === "https:" ? "wss" : "ws";
  return `${scheme}://${loc.host}/ws?t=${encodeURIComponent(token)}`;
}
