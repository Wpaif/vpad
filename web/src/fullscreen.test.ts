import { afterEach, beforeEach, describe, expect, it, vi, type Mock } from "vitest";
import { bindInstallHint, enterFullscreenOnTouch, shouldShowInstallHint } from "./fullscreen";

function setFullscreen(enabled: boolean | undefined, element: Element | null = null) {
  Object.defineProperty(document, "fullscreenEnabled", { value: enabled, configurable: true });
  Object.defineProperty(document, "fullscreenElement", { value: element, configurable: true });
}

function touch() {
  document.dispatchEvent(new PointerEvent("pointerdown", { pointerId: 1, bubbles: true }));
}

describe("enterFullscreenOnTouch", () => {
  let stop: () => void;
  let request: Mock<(o?: FullscreenOptions) => Promise<void>>;

  beforeEach(() => {
    request = vi.fn<(o?: FullscreenOptions) => Promise<void>>().mockResolvedValue(undefined);
    document.documentElement.requestFullscreen = request;
    setFullscreen(true);
  });
  afterEach(() => {
    stop();
    setFullscreen(undefined);
  });

  it("pede tela cheia no primeiro toque, escondendo a barra do navegador", () => {
    stop = enterFullscreenOnTouch(document);
    touch();
    expect(request).toHaveBeenCalledWith({ navigationUI: "hide" });
  });

  it("não faz nada antes do toque (o navegador exige um gesto)", () => {
    stop = enterFullscreenOnTouch(document);
    expect(request).not.toHaveBeenCalled();
  });

  it("não tenta quando o navegador não suporta (Safari do iPhone)", () => {
    setFullscreen(undefined);
    stop = enterFullscreenOnTouch(document);
    touch();
    expect(request).not.toHaveBeenCalled();
  });

  it("não pede de novo se já está em tela cheia", () => {
    setFullscreen(true, document.documentElement);
    stop = enterFullscreenOnTouch(document);
    touch();
    expect(request).not.toHaveBeenCalled();
  });

  it("volta a pedir se o usuário saiu da tela cheia", () => {
    stop = enterFullscreenOnTouch(document);
    touch();
    setFullscreen(true, null);
    touch();
    expect(request).toHaveBeenCalledTimes(2);
  });

  it("depois de desligado não pede mais", () => {
    stop = enterFullscreenOnTouch(document);
    stop();
    touch();
    expect(request).not.toHaveBeenCalled();
  });

  it("recusa do navegador não gera erro", async () => {
    request.mockRejectedValue(new Error("negado"));
    stop = enterFullscreenOnTouch(document);
    expect(() => touch()).not.toThrow();
    await Promise.resolve();
  });
});

describe("shouldShowInstallHint", () => {
  const base = { standalone: false, fullscreenEnabled: false, dismissed: false };

  it("mostra no Safari do iPhone fora do app instalado", () => {
    expect(shouldShowInstallHint(base)).toBe(true);
  });
  it("não mostra se já está instalado na tela inicial", () => {
    expect(shouldShowInstallHint({ ...base, standalone: true })).toBe(false);
  });
  it("não mostra onde a tela cheia nativa funciona", () => {
    expect(shouldShowInstallHint({ ...base, fullscreenEnabled: true })).toBe(false);
  });
  it("não mostra se o usuário dispensou", () => {
    expect(shouldShowInstallHint({ ...base, dismissed: true })).toBe(false);
  });
});

describe("bindInstallHint", () => {
  const store = new Map<string, string>();
  const storage = {
    getItem: (k: string) => store.get(k) ?? null,
    setItem: (k: string, v: string) => void store.set(k, v),
  };

  beforeEach(() => {
    store.clear();
    document.body.innerHTML = `<aside id="hint" hidden><button id="close"></button></aside>`;
  });

  const env = { standalone: false, fullscreenEnabled: false };
  const hint = () => document.getElementById("hint")!;

  it("exibe a dica quando necessário", () => {
    bindInstallHint(hint(), document.getElementById("close")!, env, storage);
    expect(hint().hidden).toBe(false);
  });

  it("fechar esconde e lembra a escolha", () => {
    bindInstallHint(hint(), document.getElementById("close")!, env, storage);
    document.getElementById("close")!.click();
    expect(hint().hidden).toBe(true);
    expect(store.size).toBe(1);
  });

  it("não reaparece depois de dispensada", () => {
    bindInstallHint(hint(), document.getElementById("close")!, env, storage);
    document.getElementById("close")!.click();
    hint().hidden = true;
    bindInstallHint(hint(), document.getElementById("close")!, env, storage);
    expect(hint().hidden).toBe(true);
  });

  it("funciona mesmo se o armazenamento falhar", () => {
    const quebrado = {
      getItem: () => {
        throw new Error("bloqueado");
      },
      setItem: () => {
        throw new Error("bloqueado");
      },
    };
    expect(() => {
      bindInstallHint(hint(), document.getElementById("close")!, env, quebrado);
      document.getElementById("close")!.click();
    }).not.toThrow();
    expect(hint().hidden).toBe(true);
  });
});
