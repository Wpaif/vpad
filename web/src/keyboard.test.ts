import { setLanguage } from "./i18n";
import { CATALOGS } from "./locales";
import { describe, expect, it, vi } from "vitest";
import { loadKeyboard, saveKeyboard, type KeyboardConfig } from "./keyboard";

const CONFIG: KeyboardConfig = {
  mapa: { A: "KeyK" },
  padrao: { A: "KeyK", B: "KeyL" },
  controles: ["A", "B"],
  teclas: [{ id: "KeyK", rotulo: "K", grupo: "Letras" }],
};

function resposta(status: number, corpo: unknown): Response {
  return new Response(JSON.stringify(corpo), { status, headers: { "Content-Type": "application/json" } });
}

describe("loadKeyboard", () => {
  it("busca a configuração em /api/teclado", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(200, CONFIG));
    expect(await loadKeyboard("tok", fetchFn)).toEqual(CONFIG);
    expect(fetchFn).toHaveBeenCalledWith("/api/teclado", { headers: { Authorization: "Bearer tok" } });
  });

  it("401 pede para parear o aparelho de novo", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(401, { erro: "não autorizado" }));
    await expect(loadKeyboard("errado", fetchFn)).rejects.toThrow(/parear/i);
  });

  it("403 explica que o aparelho não tem permissão para o modo teclado", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(403, { erro: "x" }));
    await expect(loadKeyboard("tok", fetchFn)).rejects.toThrow(/permissão/i);
  });

  it("falha com mensagem clara se o servidor não responde ok", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(500, {}));
    await expect(loadKeyboard("tok", fetchFn)).rejects.toThrow(/500/);
  });
});

describe("saveKeyboard", () => {
  it("envia o mapa por PUT em JSON e devolve o que o servidor guardou", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(200, { mapa: { A: "Space" } }));
    expect(await saveKeyboard({ A: "Space" }, "tok", fetchFn)).toEqual({ A: "Space" });

    const [url, init] = fetchFn.mock.calls[0]!;
    expect(url).toBe("/api/teclado");
    expect(init.method).toBe("PUT");
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(init.headers.Authorization).toBe("Bearer tok");
    expect(JSON.parse(init.body)).toEqual({ mapa: { A: "Space" } });
  });

  it("mapa recusado pelo PC mostra uma mensagem traduzida, não o texto que o servidor mandou", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(400, { codigo: "mapa_invalido", erro: "tecla desconhecida: Hyper" }));
    const erro = await saveKeyboard({ A: "Hyper" }, "tok", fetchFn).catch((e: Error) => e);
    expect((erro as Error).message).toBe("o PC recusou este mapa de teclas");
    expect((erro as Error).message).not.toContain("Hyper");
  });

  it("o PC não conseguir gravar também vira mensagem própria", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(500, { codigo: "gravacao_falhou", erro: "não consegui gravar: x" }));
    await expect(saveKeyboard({}, "tok", fetchFn)).rejects.toThrow("o PC não conseguiu gravar as teclas");
  });

  it("erro sem código conhecido mostra só o status", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(502, { erro: "texto do servidor" }));
    await expect(saveKeyboard({}, "tok", fetchFn)).rejects.toThrow("não consegui salvar (502)");
  });

  it("403 ao salvar também explica a falta de permissão", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(403, { erro: "x" }));
    await expect(saveKeyboard({ A: "Space" }, "tok", fetchFn)).rejects.toThrow(/permissão/i);
  });

  it("falha de rede vira erro legível", async () => {
    const fetchFn = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(saveKeyboard({}, "tok", fetchFn)).rejects.toThrow(/conex/i);
  });

  it("401 vira mensagem de acesso negado", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(401, { erro: "não autorizado" }));
    await expect(saveKeyboard({}, "errado", fetchFn)).rejects.toThrow(/parear/i);
  });
});

describe("erros do teclado em cada idioma", () => {
  const resposta2 = (status: number, corpo: unknown) =>
    vi.fn().mockResolvedValue(new Response(JSON.stringify(corpo), { status }));

  it.each(["en", "es", "zh-CN", "ja", "ko"] as const)("%s", async (idioma) => {
    setLanguage(idioma);
    const c = CATALOGS[idioma] as Record<string, string>;
    await expect(loadKeyboard("t", resposta2(401, {}))).rejects.toThrow(c["keyboard.error.denied"]!);
    await expect(loadKeyboard("t", resposta2(403, {}))).rejects.toThrow(c["keyboard.error.noPermission"]!);
    await expect(saveKeyboard({}, "t", resposta2(400, { codigo: "mapa_invalido" }))).rejects.toThrow(c["keyboard.error.invalid"]!);
    await expect(saveKeyboard({}, "t", resposta2(500, { codigo: "gravacao_falhou" }))).rejects.toThrow(c["keyboard.error.writeFailed"]!);
    setLanguage("pt-BR");
  });
});
