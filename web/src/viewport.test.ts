import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { bindAppHeight, formatViewportInfo } from "./viewport";

describe("bindAppHeight", () => {
  let stop: () => void;

  beforeEach(() => {
    vi.useFakeTimers();
    Object.defineProperty(window, "innerHeight", { value: 390, configurable: true, writable: true });
    document.documentElement.style.removeProperty("--app-h");
  });
  afterEach(() => {
    stop();
    vi.useRealTimers();
  });

  const height = () => document.documentElement.style.getPropertyValue("--app-h");

  it("define --app-h com a altura real da janela logo de início", () => {
    stop = bindAppHeight(window, document.documentElement);
    expect(height()).toBe("390px");
  });

  it("define --app-w com a largura real da janela e a atualiza", () => {
    Object.defineProperty(window, "innerWidth", { value: 844, configurable: true, writable: true });
    stop = bindAppHeight(window, document.documentElement);
    const width = () => document.documentElement.style.getPropertyValue("--app-w");
    expect(width()).toBe("844px");
    (window as { innerWidth: number }).innerWidth = 667;
    window.dispatchEvent(new Event("resize"));
    expect(width()).toBe("667px");
  });

  it("atualiza ao redimensionar", () => {
    stop = bindAppHeight(window, document.documentElement);
    (window as { innerHeight: number }).innerHeight = 369;
    window.dispatchEvent(new Event("resize"));
    expect(height()).toBe("369px");
  });

  it("depois de girar relê a altura, pois o iOS reporta o valor antigo no evento", () => {
    stop = bindAppHeight(window, document.documentElement);
    window.dispatchEvent(new Event("orientationchange"));
    (window as { innerHeight: number }).innerHeight = 390;
    vi.advanceTimersByTime(400);
    expect(height()).toBe("390px");

    (window as { innerHeight: number }).innerHeight = 844;
    window.dispatchEvent(new Event("orientationchange"));
    expect(height()).toBe("390px"); // ainda o valor velho
    vi.advanceTimersByTime(400);
    expect(height()).toBe("844px");
  });

  it("depois de desligado não atualiza mais", () => {
    stop = bindAppHeight(window, document.documentElement);
    stop();
    (window as { innerHeight: number }).innerHeight = 100;
    window.dispatchEvent(new Event("resize"));
    expect(height()).toBe("390px");
  });
});

describe("formatViewportInfo", () => {
  it("resume janela, tela, áreas seguras e modo app", () => {
    expect(
      formatViewportInfo({
        inner: [844, 369],
        screen: [844, 390],
        safe: { top: 0, right: 47, bottom: 21, left: 47 },
        standalone: true,
      }),
    ).toBe("janela 844×369\ntela 844×390\nseguro 0/47/21/47\napp sim");
  });

  it("indica quando não está no modo app", () => {
    expect(
      formatViewportInfo({
        inner: [1, 2],
        screen: [3, 4],
        safe: { top: 0, right: 0, bottom: 0, left: 0 },
        standalone: false,
      }),
    ).toContain("app não");
  });
});
