import { beforeEach, describe, expect, it } from "vitest";
import { bindVisibilityToggle } from "./ui";

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
