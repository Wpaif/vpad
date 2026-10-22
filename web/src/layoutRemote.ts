import { parseLayout, serializeLayout, type Layout } from "./layoutModel";

/** Cliente da API do layout guardado no PC (GET/PUT /api/layout). Nunca lança. */

type FetchFn = typeof fetch;

const URL = "/api/layout";

export type RemoteLayout =
  | { status: "ok"; layout: Layout | null }
  | { status: "unauthorized" | "offline" | "error" };

export type PushResult = "ok" | "unauthorized" | "rejected" | "offline" | "error";

/** O layout do servidor, ou null se não há (ou se o que há é inválido ou de uma versão futura). */
export async function fetchRemoteLayout(token: string, fetchFn: FetchFn = fetch): Promise<RemoteLayout> {
  let res: Response;
  try {
    res = await fetchFn(URL, { headers: { Authorization: `Bearer ${token}` } });
  } catch {
    return { status: "offline" };
  }
  if (res.status === 401) return { status: "unauthorized" };
  if (!res.ok) return { status: "error" };
  let body: { layout?: unknown };
  try {
    body = (await res.json()) as { layout?: unknown };
  } catch {
    return { status: "error" };
  }
  if (body.layout === null || body.layout === undefined) return { status: "ok", layout: null };
  const parsed = parseLayout(body.layout);
  return { status: "ok", layout: parsed.status === "ok" ? parsed.layout : null };
}

export async function pushRemoteLayout(layout: Layout, token: string, fetchFn: FetchFn = fetch): Promise<PushResult> {
  let res: Response;
  try {
    res = await fetchFn(URL, {
      method: "PUT",
      headers: { "Content-Type": "application/json", Authorization: `Bearer ${token}` },
      body: `{"layout":${serializeLayout(layout)}}`,
    });
  } catch {
    return "offline";
  }
  if (res.ok) return "ok";
  if (res.status === 401) return "unauthorized";
  if (res.status === 400 || res.status === 413 || res.status === 422) return "rejected";
  return "error";
}
