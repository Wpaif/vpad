import { beforeEach, describe, expect, it } from "vitest";
import { bindPanelToggle, bindVisibilityToggle, lockGestures, renderStats, renderStatus } from "./ui";

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

const PANEL = `
  <div id="status"></div>
  <section id="stats" hidden>
    <b data-stat="last"></b><b data-stat="avg"></b><b data-stat="min"></b>
    <b data-stat="max"></b><b data-stat="jitter"></b><b data-stat="loss"></b>
    <svg viewBox="0 0 120 32"><polyline data-stat="spark"></polyline></svg>
  </section>`;

const stat = (name: string) => document.querySelector(`[data-stat=${name}]`)!;

describe("painel de latência", () => {
  beforeEach(() => {
    document.body.innerHTML = PANEL;
  });

  it("preenche as estatísticas formatadas", () => {
    renderStats(document.getElementById("stats")!, {
      last: 12, min: 8, avg: 14.5, max: 31, jitter: 3.2, loss: 0.25, samples: [8, 12, 31],
    });
    expect(stat("last").textContent).toBe("12 ms");
    expect(stat("avg").textContent).toBe("14.5 ms");
    expect(stat("min").textContent).toBe("8 ms");
    expect(stat("max").textContent).toBe("31 ms");
    expect(stat("jitter").textContent).toBe("3.2 ms");
    expect(stat("loss").textContent).toBe("25%");
    expect(stat("spark").getAttribute("points")).not.toBe("");
  });

  it("sem dados mostra traço e gráfico vazio", () => {
    renderStats(document.getElementById("stats")!, {
      last: null, min: null, avg: null, max: null, jitter: null, loss: 0, samples: [],
    });
    expect(stat("last").textContent).toBe("–");
    expect(stat("jitter").textContent).toBe("–");
    expect(stat("loss").textContent).toBe("0%");
    expect(stat("spark").getAttribute("points")).toBe("");
  });

  it("tocar no indicador abre e fecha o painel", () => {
    const status = document.getElementById("status")!;
    const panel = document.getElementById("stats")!;
    bindPanelToggle(status, panel);
    status.click();
    expect(panel.hidden).toBe(false);
    status.click();
    expect(panel.hidden).toBe(true);
  });

  it("avisa quando o painel abre ou fecha", () => {
    const status = document.getElementById("status")!;
    const panel = document.getElementById("stats")!;
    const calls: boolean[] = [];
    bindPanelToggle(status, panel, (open) => calls.push(open));
    status.click();
    status.click();
    expect(calls).toEqual([true, false]);
  });
});

describe("lockGestures", () => {
  function tocar(alvo: Element): TouchEvent {
    const e = new Event("touchmove", { bubbles: true, cancelable: true }) as TouchEvent;
    alvo.dispatchEvent(e);
    return e;
  }

  it("bloqueia o arrasto na página, para ela não rolar nem dar zoom", () => {
    document.body.innerHTML = `<main id="pad"></main>`;
    lockGestures();
    expect(tocar(document.getElementById("pad")!).defaultPrevented).toBe(true);
  });

  it("deixa rolar dentro de uma área marcada com data-scroll (editor de teclas)", () => {
    document.body.innerHTML = `<div data-scroll><p id="dentro"></p></div><p id="fora"></p>`;
    lockGestures();
    expect(tocar(document.getElementById("dentro")!).defaultPrevented).toBe(false);
    expect(tocar(document.getElementById("fora")!).defaultPrevented).toBe(true);
  });
});
