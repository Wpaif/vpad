import { describe, expect, it, vi } from "vitest";
import { defaultLayout, setHidden, withGroup } from "./layoutModel";
import { fetchRemoteLayout, pushRemoteLayout } from "./layoutRemote";

const TOKEN = "0123456789abcdef0123456789abcdef";
const resposta = (status: number, corpo: unknown): Response =>
  new Response(JSON.stringify(corpo), { status, headers: { "Content-Type": "application/json" } });
const fetchDe = (r: Response | (() => never)) =>
  vi.fn(async () => (typeof r === "function" ? r() : r)) as unknown as typeof fetch & ReturnType<typeof vi.fn>;

const salvo = setHidden(withGroup(defaultLayout(), "face", { dx: 0.1, scale: 1.2 }), "A", true);

describe("fetchRemoteLayout", () => {
  it("manda o token e devolve o layout do servidor", async () => {
    const f = fetchDe(resposta(200, { layout: JSON.parse(JSON.stringify(salvo)) }));
    const r = await fetchRemoteLayout(TOKEN, f);
    expect(r).toEqual({ status: "ok", layout: salvo });
    const [url, init] = (f as unknown as ReturnType<typeof vi.fn>).mock.calls[0]!;
    expect(url).toBe("/api/layout");
    expect(init.headers.Authorization).toBe(`Bearer ${TOKEN}`);
  });

  it("servidor sem layout guardado devolve layout null", async () => {
    expect(await fetchRemoteLayout(TOKEN, fetchDe(resposta(200, { layout: null })))).toEqual({ status: "ok", layout: null });
  });

  it("dado ruim ou de versão futura no servidor não é adotado", async () => {
    expect(await fetchRemoteLayout(TOKEN, fetchDe(resposta(200, { layout: 5 })))).toEqual({ status: "ok", layout: null });
    expect(await fetchRemoteLayout(TOKEN, fetchDe(resposta(200, { layout: { version: 99, groups: {}, hidden: [] } })))).toEqual({
      status: "ok",
      layout: null,
    });
  });

  it("corpo que não é JSON conta como erro do servidor", async () => {
    const r = new Response("<html>", { status: 200 });
    expect(await fetchRemoteLayout(TOKEN, fetchDe(r))).toEqual({ status: "error" });
  });

  it("401 diz que o aparelho não é mais reconhecido", async () => {
    expect(await fetchRemoteLayout(TOKEN, fetchDe(resposta(401, { erro: "x" })))).toEqual({ status: "unauthorized" });
  });

  it("falha de rede é offline; outros erros HTTP são erro", async () => {
    const queda = fetchDe(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await fetchRemoteLayout(TOKEN, queda)).toEqual({ status: "offline" });
    expect(await fetchRemoteLayout(TOKEN, fetchDe(resposta(500, {})))).toEqual({ status: "error" });
  });
});

describe("pushRemoteLayout", () => {
  it("envia {layout} serializado, com o token, e confirma", async () => {
    const f = fetchDe(resposta(200, { layout: {} }));
    expect(await pushRemoteLayout(salvo, TOKEN, f)).toBe("ok");
    const [url, init] = (f as unknown as ReturnType<typeof vi.fn>).mock.calls[0]!;
    expect(url).toBe("/api/layout");
    expect(init.method).toBe("PUT");
    expect(init.headers.Authorization).toBe(`Bearer ${TOKEN}`);
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(init.body)).toEqual({ layout: JSON.parse(JSON.stringify(salvo)) });
  });

  it("o padrão também é enviado (vale como 'restaurado')", async () => {
    const f = fetchDe(resposta(200, { layout: {} }));
    expect(await pushRemoteLayout(defaultLayout(), TOKEN, f)).toBe("ok");
    expect(JSON.parse((f as unknown as ReturnType<typeof vi.fn>).mock.calls[0]![1].body)).toEqual({
      layout: { version: 1, groups: {}, hidden: [] },
    });
  });

  it("distingue não autorizado, recusado, erro e offline", async () => {
    expect(await pushRemoteLayout(salvo, TOKEN, fetchDe(resposta(401, {})))).toBe("unauthorized");
    expect(await pushRemoteLayout(salvo, TOKEN, fetchDe(resposta(400, { erro: "x" })))).toBe("rejected");
    expect(await pushRemoteLayout(salvo, TOKEN, fetchDe(resposta(413, {})))).toBe("rejected");
    expect(await pushRemoteLayout(salvo, TOKEN, fetchDe(resposta(500, {})))).toBe("error");
    const queda = fetchDe(() => {
      throw new TypeError("Failed to fetch");
    });
    expect(await pushRemoteLayout(salvo, TOKEN, queda)).toBe("offline");
  });
});
