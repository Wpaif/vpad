import { beforeEach, describe, expect, it, vi } from "vitest";
import { mountPingToggle } from "./pingToggle";

let button: HTMLButtonElement;
beforeEach(() => {
  document.body.innerHTML = `<button id="t"></button>`;
  button = document.getElementById("t") as HTMLButtonElement;
});

describe("mountPingToggle", () => {
  it("mostra o estado inicial sem chamar o aviso", () => {
    const on = vi.fn();
    mountPingToggle(button, true, on);
    expect(button.getAttribute("aria-pressed")).toBe("true");
    expect(on).not.toHaveBeenCalled();
  });

  it("cada clique alterna, atualiza aria-pressed e avisa", () => {
    const on = vi.fn();
    mountPingToggle(button, false, on);
    expect(button.getAttribute("aria-pressed")).toBe("false");
    button.click();
    expect(button.getAttribute("aria-pressed")).toBe("true");
    button.click();
    expect(button.getAttribute("aria-pressed")).toBe("false");
    expect(on.mock.calls).toEqual([[true], [false]]);
  });

  it("o texto diz o estado", () => {
    mountPingToggle(button, false, () => {});
    expect(button.textContent).toContain("desligado");
    button.click();
    expect(button.textContent).toContain("ligado");
    expect(button.textContent).not.toContain("desligado");
  });
});
