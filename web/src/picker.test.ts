import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { PROFILES } from "./profiles";
import { mountProfilePicker } from "./picker";

let picked: number[];
let picker: ReturnType<typeof mountProfilePicker>;

const buttons = () => [...document.querySelectorAll<HTMLButtonElement>("#picker button")];
const button = (i: number) => buttons()[i]!;

beforeEach(() => {
  vi.useFakeTimers();
  document.body.innerHTML = `<div id="picker"></div>`;
  picked = [];
  picker = mountProfilePicker(document.getElementById("picker")!, (i) => picked.push(i));
});
afterEach(() => vi.useRealTimers());

describe("mountProfilePicker", () => {
  it("cria um botão por perfil, com rótulo curto e nome completo acessível", () => {
    expect(buttons().map((b) => b.textContent)).toEqual(PROFILES.map((p) => p.short));
    expect(buttons().map((b) => b.getAttribute("aria-label"))).toEqual(PROFILES.map((p) => p.label));
  });

  it("começa desabilitado, sem perfil ativo, até o servidor informar", () => {
    expect(buttons().every((b) => b.disabled)).toBe(true);
    expect(buttons().every((b) => b.getAttribute("aria-pressed") === "false")).toBe(true);
  });

  it("setCurrent marca o perfil ativo e habilita a escolha", () => {
    picker.setCurrent(1);
    expect(buttons().map((b) => b.getAttribute("aria-pressed"))).toEqual(
      PROFILES.map((_, i) => String(i === 1)),
    );
    expect(buttons().every((b) => !b.disabled)).toBe(true);
  });

  it("escolher outro perfil pede a troca e fica pendente até o servidor confirmar", () => {
    picker.setCurrent(0);
    button(2).click();
    expect(picked).toEqual([2]);
    expect(button(2).hasAttribute("data-pending")).toBe(true);
    expect(button(0).getAttribute("aria-pressed")).toBe("true"); // ainda o antigo

    picker.setCurrent(2);
    expect(button(2).hasAttribute("data-pending")).toBe(false);
    expect(button(2).getAttribute("aria-pressed")).toBe("true");
    expect(button(0).getAttribute("aria-pressed")).toBe("false");
  });

  it("tocar no perfil já ativo não faz nada", () => {
    picker.setCurrent(1);
    button(1).click();
    expect(picked).toEqual([]);
  });

  it("se a troca falha, o servidor reafirma o perfil antigo e o pendente some", () => {
    picker.setCurrent(0);
    button(1).click();
    picker.setCurrent(0);
    expect(button(1).hasAttribute("data-pending")).toBe(false);
    expect(button(0).getAttribute("aria-pressed")).toBe("true");
  });

  it("sem resposta do servidor o pendente expira", () => {
    picker.setCurrent(0);
    button(1).click();
    vi.advanceTimersByTime(4000);
    expect(button(1).hasAttribute("data-pending")).toBe(false);
  });

  it("sem conexão (null) desabilita tudo e esquece o pendente", () => {
    picker.setCurrent(0);
    button(1).click();
    picker.setCurrent(null);
    expect(buttons().every((b) => b.disabled)).toBe(true);
    expect(buttons().some((b) => b.hasAttribute("data-pending"))).toBe(false);
    button(2).click();
    expect(picked).toEqual([1]);
  });

  it("ignora índice desconhecido vindo do servidor", () => {
    picker.setCurrent(99);
    expect(buttons().every((b) => b.getAttribute("aria-pressed") === "false")).toBe(true);
  });
});
