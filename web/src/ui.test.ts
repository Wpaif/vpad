import { beforeEach, describe, expect, it } from "vitest";
import { bindVisibilityToggle, renderStatus } from "./ui";

beforeEach(() => {
  document.body.innerHTML = `
    <div id="pad"></div>
    <button id="toggle"></button>`;
});

describe("ocultar controles", () => {
  it("alterna o atributo data-hidden no controle", () => {
    const pad = document.getElementById("pad")!;
    const toggle = document.getElementById("toggle")!;
    bindVisibilityToggle(toggle, pad);

    toggle.click();
    expect(pad.hasAttribute("data-hidden")).toBe(true);
    expect(toggle.getAttribute("aria-pressed")).toBe("true");

    toggle.click();
    expect(pad.hasAttribute("data-hidden")).toBe(false);
    expect(toggle.getAttribute("aria-pressed")).toBe("false");
  });
});

describe("indicador de conexão", () => {
  it("mostra o estado e a latência", () => {
    document.body.innerHTML = `<div id="status"></div>`;
    const el = document.getElementById("status")!;
    renderStatus(el, "open", 12);
    expect(el.dataset.status).toBe("open");
    expect(el.textContent).toBe("12 ms");
  });

  it("sem latência mostra o rótulo do estado", () => {
    document.body.innerHTML = `<div id="status"></div>`;
    const el = document.getElementById("status")!;
    renderStatus(el, "reconnecting", null);
    expect(el.dataset.status).toBe("reconnecting");
    expect(el.textContent).toBe("reconectando…");
    renderStatus(el, "connecting", null);
    expect(el.textContent).toBe("conectando…");
  });

  it("offline some com a latência antiga", () => {
    document.body.innerHTML = `<div id="status"></div>`;
    const el = document.getElementById("status")!;
    renderStatus(el, "open", 8);
    renderStatus(el, "closed", null);
    expect(el.textContent).toBe("offline");
  });
});
