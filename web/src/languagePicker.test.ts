import { beforeEach, describe, expect, it, vi } from "vitest";
import { LANGUAGES, LANGUAGE_KEY, getLanguage, setLanguage } from "./i18n";
import { mountLanguagePicker } from "./languagePicker";

const memoria = () => {
  const m = new Map<string, string>();
  return {
    getItem: (k: string) => m.get(k) ?? null,
    setItem: (k: string, v: string) => void m.set(k, v),
    removeItem: (k: string) => void m.delete(k),
  };
};

let select: HTMLSelectElement;
beforeEach(() => {
  setLanguage("pt-BR");
  document.body.innerHTML = `<select id="s"></select>`;
  select = document.getElementById("s") as HTMLSelectElement;
});

describe("mountLanguagePicker", () => {
  it("lista os seis idiomas com o nome na própria língua", () => {
    mountLanguagePicker(select, memoria(), vi.fn());
    expect([...select.options].map((o) => [o.value, o.textContent])).toEqual(
      LANGUAGES.map((l) => [l.code, l.name]),
    );
  });

  it("começa no idioma em uso", () => {
    setLanguage("ko");
    mountLanguagePicker(select, memoria(), vi.fn());
    expect(select.value).toBe("ko");
    expect(getLanguage()).toBe("ko");
  });

  it("escolher outro guarda a escolha e recarrega a página", () => {
    const s = memoria();
    const recarregar = vi.fn();
    mountLanguagePicker(select, s, recarregar);
    select.value = "ja";
    select.dispatchEvent(new Event("change", { bubbles: true }));
    expect(s.getItem(LANGUAGE_KEY)).toBe("ja");
    expect(recarregar).toHaveBeenCalledTimes(1);
  });

  it("os nomes não viram HTML", () => {
    mountLanguagePicker(select, memoria(), vi.fn());
    expect(select.querySelector("*:not(option)")).toBeNull();
  });
});
