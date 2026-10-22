import { beforeEach, describe, expect, it, vi } from "vitest";
import { defaultLayout, opacityOf } from "./layoutModel";
import { createLayoutStore, type LayoutStore } from "./layoutStore.svelte";
import { mountOpacitySlider } from "./layoutOpacity";

let store: LayoutStore;
let input: HTMLInputElement;
let output: HTMLOutputElement;
let onChange: ReturnType<typeof vi.fn<() => void>>;
let slider: { sync: () => void };

const mover = (valor: number) => {
  input.value = String(valor);
  input.dispatchEvent(new Event("input", { bubbles: true }));
};

beforeEach(() => {
  localStorage.clear();
  document.body.innerHTML = `<input id="op" type="range" min="20" max="100" step="5" value="100"><output id="out"></output>`;
  input = document.getElementById("op") as HTMLInputElement;
  output = document.getElementById("out") as HTMLOutputElement;
  store = createLayoutStore();
  onChange = vi.fn<() => void>();
  slider = mountOpacitySlider({ input, output, store, onChange });
});

describe("mountOpacitySlider", () => {
  it("mostra a opacidade atual no começo", () => {
    expect(input.value).toBe("100");
    expect(output.textContent).toBe("100%");
  });

  it("mexer no controle muda a opacidade do rascunho", () => {
    store.edit();
    mover(60);
    expect(opacityOf(store.current)).toBe(0.6);
    expect(output.textContent).toBe("60%");
    expect(onChange).toHaveBeenCalled();
  });

  it("de volta a 100% o layout volta a ser o padrão", () => {
    store.edit();
    mover(60);
    mover(100);
    expect(store.current).toEqual(defaultLayout());
  });

  it("fora do modo edição o controle não muda nada e volta ao valor salvo", () => {
    mover(40);
    expect(opacityOf(store.current)).toBe(1);
    expect(input.value).toBe("100");
    expect(output.textContent).toBe("100%");
  });

  it("sync reflete mudanças de fora, como restaurar o padrão", () => {
    store.edit();
    mover(50);
    store.restoreDefault();
    slider.sync();
    expect(input.value).toBe("100");
    expect(output.textContent).toBe("100%");
  });

  it("sync mostra a opacidade salva ao abrir uma nova edição", () => {
    store.edit();
    mover(70);
    store.save();
    store.edit();
    slider.sync();
    expect(input.value).toBe("70");
  });

  it("sync depois de cancelar volta ao salvo", () => {
    store.edit();
    mover(30);
    store.cancel();
    slider.sync();
    expect(input.value).toBe("100");
  });

  it("o texto é só o percentual (sem HTML)", () => {
    store.edit();
    mover(85);
    expect(output.children.length).toBe(0);
    expect(output.textContent).toBe("85%");
  });
});
