import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { deviceName, mountPairing, pairDevice } from "./pairing";

const TOKEN = "0123456789abcdef0123456789abcdef";

function resposta(status: number, corpo: unknown, headers: Record<string, string> = {}): Response {
  return new Response(JSON.stringify(corpo), {
    status,
    headers: { "Content-Type": "application/json", ...headers },
  });
}

describe("pairDevice", () => {
  it("envia o código e o nome do aparelho por POST e devolve o token", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(200, { token: TOKEN, id: "ab12cd34" }));
    expect(await pairDevice("123456", "iPhone", fetchFn)).toBe(TOKEN);

    const [url, init] = fetchFn.mock.calls[0]!;
    expect(url).toBe("/api/parear");
    expect(init.method).toBe("POST");
    expect(init.headers["Content-Type"]).toBe("application/json");
    expect(JSON.parse(init.body)).toEqual({ codigo: "123456", nome: "iPhone" });
  });

  it("código errado: pede para conferir o código do terminal", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(401, { erro: "código incorreto ou expirado" }));
    await expect(pairDevice("000000", "iPhone", fetchFn)).rejects.toThrow(/incorreto ou expirado/i);
  });

  it("muitas tentativas: informa quanto esperar", async () => {
    const fetchFn = vi
      .fn()
      .mockResolvedValue(resposta(429, { erro: "x", segundos: 42 }, { "Retry-After": "42" }));
    const erro = await pairDevice("000000", "iPhone", fetchFn).catch((e) => e);
    expect(erro.message).toMatch(/42 s/);
    expect(erro.retryAfter).toBe(42);
  });

  it("sem rede: mensagem de conexão", async () => {
    const fetchFn = vi.fn().mockRejectedValue(new TypeError("Failed to fetch"));
    await expect(pairDevice("123456", "iPhone", fetchFn)).rejects.toThrow(/conex/i);
  });

  it("outro erro do servidor: mostra o status", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(500, {}));
    await expect(pairDevice("123456", "iPhone", fetchFn)).rejects.toThrow(/500/);
  });

  it("resposta sem token válido é tratada como erro", async () => {
    const fetchFn = vi.fn().mockResolvedValue(resposta(200, { token: "curto" }));
    await expect(pairDevice("123456", "iPhone", fetchFn)).rejects.toThrow();
  });
});

describe("deviceName", () => {
  it.each([
    ["Mozilla/5.0 (iPhone; CPU iPhone OS 17_0 like Mac OS X) AppleWebKit/605.1.15", 5, "iPhone"],
    ["Mozilla/5.0 (iPad; CPU OS 17_0 like Mac OS X) AppleWebKit/605.1.15", 5, "iPad"],
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15", 5, "iPad"], // iPadOS finge ser Mac
    ["Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15", 0, "Mac"],
    ["Mozilla/5.0 (Linux; Android 14; Pixel 8) AppleWebKit/537.36", 5, "Android"],
    ["Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36", 0, "Windows"],
    ["Mozilla/5.0 (X11; Linux x86_64; rv:130.0) Gecko/20100101 Firefox/130.0", 0, "Linux"],
    ["algo desconhecido", 0, "dispositivo"],
  ])("%s", (ua, toques, esperado) => {
    expect(deviceName(ua, toques)).toBe(esperado);
  });
});

describe("mountPairing", () => {
  let root: HTMLElement;
  let pair: ReturnType<typeof vi.fn>;
  let onPaired: ReturnType<typeof vi.fn>;
  let pairing: ReturnType<typeof mountPairing>;

  const input = () => root.querySelector<HTMLInputElement>("input")!;
  const submit = () => root.querySelector<HTMLButtonElement>("button[type=submit]")!;
  const alertEl = () => root.querySelector<HTMLElement>("[role=alert]")!;
  const flush = () => new Promise((r) => setTimeout(r, 0));

  function digitar(texto: string) {
    input().value = texto;
    input().dispatchEvent(new Event("input", { bubbles: true }));
  }
  const enviar = () => root.querySelector("form")!.dispatchEvent(new Event("submit", { cancelable: true, bubbles: true }));

  beforeEach(() => {
    vi.useFakeTimers();
    document.body.innerHTML = `
      <div id="pair" hidden>
        <form>
          <input name="codigo" inputmode="numeric" maxlength="6">
          <button type="submit" disabled>Parear</button>
          <p role="alert"></p>
        </form>
      </div>`;
    root = document.getElementById("pair")!;
    pair = vi.fn();
    onPaired = vi.fn();
    pairing = mountPairing(root, { pair: pair as never, onPaired: onPaired as never });
  });
  afterEach(() => vi.useRealTimers());

  it("show e hide controlam a visibilidade", () => {
    pairing.show();
    expect(root.hidden).toBe(false);
    pairing.hide();
    expect(root.hidden).toBe(true);
  });

  it("só aceita dígitos, no máximo seis", () => {
    digitar("12a 3-4 5678");
    expect(input().value).toBe("123456");
  });

  it("o botão só habilita com os seis dígitos", () => {
    digitar("12345");
    expect(submit().disabled).toBe(true);
    digitar("123456");
    expect(submit().disabled).toBe(false);
    digitar("123");
    expect(submit().disabled).toBe(true);
  });

  it("código completo: pareia e entrega o token", async () => {
    vi.useRealTimers();
    pair.mockResolvedValue(TOKEN);
    digitar("123456");
    enviar();
    await flush();
    expect(pair).toHaveBeenCalledWith("123456");
    expect(onPaired).toHaveBeenCalledWith(TOKEN);
  });

  it("não envia código incompleto", async () => {
    vi.useRealTimers();
    digitar("123");
    enviar();
    await flush();
    expect(pair).not.toHaveBeenCalled();
  });

  it("durante o envio o botão fica travado, para não mandar duas vezes", async () => {
    vi.useRealTimers();
    let resolver!: (t: string) => void;
    pair.mockReturnValue(new Promise<string>((r) => (resolver = r)));
    digitar("123456");
    enviar();
    expect(submit().disabled).toBe(true);
    enviar();
    expect(pair).toHaveBeenCalledTimes(1);
    resolver(TOKEN);
    await flush();
  });

  it("código errado: mostra o erro, limpa o campo e deixa tentar de novo", async () => {
    vi.useRealTimers();
    pair.mockRejectedValue(new Error("Código incorreto ou expirado."));
    digitar("123456");
    enviar();
    await flush();
    expect(alertEl().textContent).toContain("incorreto");
    expect(input().value).toBe("");
    expect(onPaired).not.toHaveBeenCalled();

    pair.mockResolvedValue(TOKEN);
    digitar("654321");
    expect(submit().disabled).toBe(false);
    enviar();
    await flush();
    expect(onPaired).toHaveBeenCalledWith(TOKEN);
    expect(alertEl().textContent).toBe("");
  });

  it("bloqueio por muitas tentativas trava o botão durante a espera e depois libera", async () => {
    const erro = Object.assign(new Error("Muitas tentativas. Espere 3 s."), { retryAfter: 3 });
    pair.mockRejectedValue(erro);
    digitar("123456");
    enviar();
    await vi.advanceTimersByTimeAsync(0);
    expect(alertEl().textContent).toContain("3 s");

    digitar("654321");
    expect(submit().disabled).toBe(true);
    await vi.advanceTimersByTimeAsync(3100);
    expect(submit().disabled).toBe(false);
  });
});
