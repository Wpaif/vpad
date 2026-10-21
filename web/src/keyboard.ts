/** Cliente da API do modo teclado (GET/PUT /api/teclado). */

/** controle -> id da tecla (o mesmo nome de KeyboardEvent.code); controle sem tecla fica de fora */
export type KeyMap = Record<string, string>;

export interface KeyInfo {
  id: string;
  rotulo: string;
  grupo: string;
}

export interface KeyboardConfig {
  mapa: KeyMap;
  padrao: KeyMap;
  controles: string[];
  teclas: KeyInfo[];
}

type FetchFn = typeof fetch;

const URL = "/api/teclado";

const DENIED = "acesso negado: é preciso parear este aparelho de novo";
const NO_PERMISSION = "este aparelho não tem permissão para o modo teclado";

export async function loadKeyboard(token: string, fetchFn: FetchFn = fetch): Promise<KeyboardConfig> {
  const res = await fetchFn(URL, { headers: { Authorization: `Bearer ${token}` } });
  if (res.status === 401) throw new Error(DENIED);
  if (res.status === 403) throw new Error(NO_PERMISSION);
  if (!res.ok) throw new Error(`não consegui carregar as teclas (${res.status})`);
  return (await res.json()) as KeyboardConfig;
}

export async function saveKeyboard(mapa: KeyMap, token: string, fetchFn: FetchFn = fetch): Promise<KeyMap> {
  let res: Response;
  try {
    res = await fetchFn(URL, {
      method: "PUT",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
      body: JSON.stringify({ mapa }),
    });
  } catch {
    throw new Error("sem conexão com o PC");
  }
  const body = (await res.json().catch(() => ({}))) as { mapa?: KeyMap; erro?: string };
  if (res.status === 403) throw new Error(NO_PERMISSION);
  if (!res.ok) throw new Error(body.erro ?? `não consegui salvar (${res.status})`);
  return body.mapa ?? mapa;
}
