import { describe, expect, it } from "vitest";
import {
  CONTROL_IDS,
  GROUP_IDS,
  GROUP_OF,
  LAYOUT_VERSION,
  MAX_SCALE,
  MIN_SCALE,
  adjustOf,
  controlsOf,
  defaultLayout,
  isDefault,
  isHidden,
  normalizeLayout,
  parseLayout,
  resetGroup,
  serializeLayout,
  setHidden,
  withGroup,
  type Layout,
  controlOffset,
  withControl,
  MIN_OPACITY,
  opacityOf,
  withOpacity,
} from "./layoutModel";

const layout = (parcial: Partial<Layout> = {}): Layout => ({ ...defaultLayout(), ...parcial });

describe("catálogo de grupos e controles", () => {
  it("são sete grupos, na ordem do controle", () => {
    expect([...GROUP_IDS]).toEqual(["ombros-esq", "ombros-dir", "analogico-esq", "analogico-dir", "direcional", "face", "menu"]);
  });

  it("são 16 controles que podem ser ocultados um a um", () => {
    expect([...CONTROL_IDS]).toEqual([
      "A", "B", "X", "Y", "LB", "RB", "LT", "RT", "Select", "Start", "Up", "Down", "Left", "Right", "StickLeft", "StickRight",
    ]);
  });

  it("cada controle pertence a um grupo conhecido", () => {
    for (const c of CONTROL_IDS) expect(GROUP_IDS).toContain(GROUP_OF[c]);
  });

  it("os grupos têm os controles certos", () => {
    expect(controlsOf("face")).toEqual(["A", "B", "X", "Y"]);
    expect(controlsOf("direcional")).toEqual(["Up", "Down", "Left", "Right"]);
    expect(controlsOf("ombros-esq")).toEqual(["LB", "LT"]);
    expect(controlsOf("ombros-dir")).toEqual(["RB", "RT"]);
    expect(controlsOf("menu")).toEqual(["Select", "Start"]);
    expect(controlsOf("analogico-esq")).toEqual(["StickLeft"]);
    expect(controlsOf("analogico-dir")).toEqual(["StickRight"]);
  });

  it("todo controle aparece em exatamente um grupo", () => {
    const todos = GROUP_IDS.flatMap((g) => controlsOf(g));
    expect([...todos].sort()).toEqual([...CONTROL_IDS].sort());
  });
});

describe("layout padrão", () => {
  it("não ajusta nenhum grupo e não oculta nada", () => {
    const l = defaultLayout();
    expect(l).toEqual({ version: LAYOUT_VERSION, groups: {}, hidden: [] });
    expect(isDefault(l)).toBe(true);
  });

  it("um grupo sem ajuste vale como deslocamento zero e escala 1", () => {
    expect(adjustOf(defaultLayout(), "face")).toEqual({ dx: 0, dy: 0, scale: 1 });
  });

  it("cada chamada devolve um objeto novo (ninguém altera o padrão sem querer)", () => {
    const a = defaultLayout();
    a.hidden.push("A");
    expect(defaultLayout().hidden).toEqual([]);
  });
});

describe("ajustes de grupo", () => {
  it("withGroup guarda deslocamento e escala sem mexer no original", () => {
    const base = defaultLayout();
    const novo = withGroup(base, "face", { dx: 0.1, dy: -0.2, scale: 1.5 });
    expect(adjustOf(novo, "face")).toEqual({ dx: 0.1, dy: -0.2, scale: 1.5 });
    expect(base.groups).toEqual({});
    expect(isDefault(novo)).toBe(false);
  });

  it("withGroup mescla com o que já havia", () => {
    const a = withGroup(defaultLayout(), "face", { dx: 0.1, scale: 1.2 });
    const b = withGroup(a, "face", { dy: 0.3 });
    expect(adjustOf(b, "face")).toEqual({ dx: 0.1, dy: 0.3, scale: 1.2 });
  });

  it("voltar um grupo ao padrão tira o registro dele", () => {
    const a = withGroup(defaultLayout(), "face", { dx: 0.1 });
    const b = withGroup(a, "face", { dx: 0 });
    expect(b.groups).toEqual({});
    expect(isDefault(b)).toBe(true);
  });

  it("resetGroup devolve só aquele grupo ao padrão", () => {
    let l = withGroup(defaultLayout(), "face", { dx: 0.1 });
    l = withGroup(l, "menu", { scale: 1.4 });
    const r = resetGroup(l, "face");
    expect(r.groups).toEqual({ menu: { dx: 0, dy: 0, scale: 1.4 } });
  });

  it("os valores são limitados: deslocamento a ±1 da área e escala de 0,5× a 2×", () => {
    const l = withGroup(defaultLayout(), "face", { dx: 5, dy: -9, scale: 10 });
    expect(adjustOf(l, "face")).toEqual({ dx: 1, dy: -1, scale: MAX_SCALE });
    const p = withGroup(defaultLayout(), "face", { scale: 0.01 });
    expect(adjustOf(p, "face").scale).toBe(MIN_SCALE);
  });

  it("valores arredondados para o armazenamento não ter ruído de ponto flutuante", () => {
    const l = withGroup(defaultLayout(), "face", { dx: 0.123456789, dy: 0.1 + 0.2, scale: 1.23456789 });
    expect(adjustOf(l, "face")).toEqual({ dx: 0.1235, dy: 0.3, scale: 1.235 });
  });

  it("número inválido volta ao valor neutro daquele campo", () => {
    const l = withGroup(defaultLayout(), "face", { dx: Number.NaN, dy: Infinity, scale: Number.NaN });
    expect(adjustOf(l, "face")).toEqual({ dx: 0, dy: 0, scale: 1 });
    expect(l.groups).toEqual({});
  });
});

describe("ocultar controles específicos", () => {
  it("setHidden oculta e mostra um controle", () => {
    const a = setHidden(defaultLayout(), "A", true);
    expect(isHidden(a, "A")).toBe(true);
    expect(isHidden(a, "B")).toBe(false);
    const b = setHidden(a, "A", false);
    expect(isHidden(b, "A")).toBe(false);
    expect(isDefault(b)).toBe(true);
  });

  it("é idempotente e não duplica", () => {
    const a = setHidden(setHidden(defaultLayout(), "A", true), "A", true);
    expect(a.hidden).toEqual(["A"]);
    expect(setHidden(defaultLayout(), "A", false).hidden).toEqual([]);
  });

  it("a lista fica sempre na ordem do catálogo", () => {
    let l = defaultLayout();
    for (const c of ["Start", "A", "StickRight", "LB"] as const) l = setHidden(l, c, true);
    expect(l.hidden).toEqual(["A", "LB", "Start", "StickRight"]);
  });

  it("ocultar um botão não mexe no resto do grupo nem nos ajustes", () => {
    const l = setHidden(withGroup(defaultLayout(), "face", { dx: 0.1 }), "A", true);
    expect(isHidden(l, "B") || isHidden(l, "X") || isHidden(l, "Y")).toBe(false);
    expect(adjustOf(l, "face").dx).toBe(0.1);
  });

  it("não altera o layout recebido", () => {
    const base = defaultLayout();
    setHidden(base, "A", true);
    expect(base.hidden).toEqual([]);
  });
});

describe("normalizeLayout", () => {
  it("tira controles desconhecidos, duplicados e ordena", () => {
    const l = normalizeLayout(layout({ hidden: ["Start", "A", "A", "Turbo" as never, "B"] }));
    expect(l.hidden).toEqual(["A", "B", "Start"]);
  });

  it("tira grupos desconhecidos e ajustes neutros", () => {
    const sujo = layout({
      groups: {
        face: { dx: 0, dy: 0, scale: 1 },
        menu: { dx: 0.2, dy: 0, scale: 1 },
        fantasma: { dx: 1, dy: 1, scale: 1 },
      } as never,
    });
    expect(normalizeLayout(sujo).groups).toEqual({ menu: { dx: 0.2, dy: 0, scale: 1 } });
  });

  it("é idempotente", () => {
    const l = withGroup(setHidden(defaultLayout(), "X", true), "menu", { dx: 0.2, scale: 1.3 });
    expect(normalizeLayout(l)).toEqual(l);
  });
});

describe("parseLayout (dados vindos do armazenamento)", () => {
  it("aceita um layout válido", () => {
    const original = withGroup(setHidden(defaultLayout(), "A", true), "face", { dx: 0.1, dy: -0.05, scale: 1.2 });
    const r = parseLayout(JSON.parse(serializeLayout(original)));
    expect(r.status).toBe("ok");
    expect(r.layout).toEqual(original);
  });

  it("ida e volta pelo texto não perde nada", () => {
    const l = withGroup(withGroup(defaultLayout(), "face", { dx: 0.3 }), "analogico-esq", { scale: 0.7 });
    expect(parseLayout(JSON.parse(serializeLayout(l))).layout).toEqual(l);
  });

  it("qualquer coisa que não seja um objeto com versão é inválida e vira o padrão", () => {
    for (const lixo of [null, undefined, 3, "x", [], true, {}, { groups: {} }, { version: "1" }, { version: 0 }, { version: 1.5 }, { version: -1 }]) {
      const r = parseLayout(lixo);
      expect(r.status, JSON.stringify(lixo)).toBe("invalid");
      expect(r.layout).toEqual(defaultLayout());
    }
  });

  it("versão mais nova que a conhecida não é interpretada: vira o padrão e avisa", () => {
    const r = parseLayout({ version: LAYOUT_VERSION + 1, groups: { face: { dx: 0.5, dy: 0, scale: 1 } }, hidden: ["A"] });
    expect(r.status).toBe("future-version");
    expect(r.layout).toEqual(defaultLayout());
  });

  it("entradas ruins são descartadas uma a uma, sem perder as boas", () => {
    const r = parseLayout({
      version: 1,
      groups: {
        face: { dx: "muito", dy: 0, scale: 1 },
        menu: { dx: 0.2, dy: 0.1, scale: 1.5 },
        direcional: 42,
        fantasma: { dx: 1, dy: 1, scale: 1 },
      },
      hidden: ["A", 7, null, "Turbo", "Start"],
    });
    expect(r.status).toBe("ok");
    expect(r.layout.groups).toEqual({ menu: { dx: 0.2, dy: 0.1, scale: 1.5 } });
    expect(r.layout.hidden).toEqual(["A", "Start"]);
  });

  it("campos faltando num grupo usam o valor neutro", () => {
    const r = parseLayout({ version: 1, groups: { face: { dx: 0.4 } }, hidden: [] });
    expect(r.layout.groups).toEqual({ face: { dx: 0.4, dy: 0, scale: 1 } });
  });

  it("valores fora da faixa são limitados, não rejeitados", () => {
    const r = parseLayout({ version: 1, groups: { face: { dx: 99, dy: -99, scale: 99 } }, hidden: [] });
    expect(r.layout.groups.face).toEqual({ dx: 1, dy: -1, scale: MAX_SCALE });
  });

  it("hidden e groups ausentes ou do tipo errado viram vazios", () => {
    expect(parseLayout({ version: 1 }).layout).toEqual(defaultLayout());
    expect(parseLayout({ version: 1, groups: "x", hidden: 5 }).layout).toEqual(defaultLayout());
  });

  it("não deixa uma chave perigosa contaminar objetos (__proto__)", () => {
    const r = parseLayout(JSON.parse('{"version":1,"groups":{"__proto__":{"dx":1,"dy":1,"scale":2}},"hidden":[]}'));
    expect(r.layout.groups).toEqual({});
    expect(({} as Record<string, unknown>).dx).toBeUndefined();
  });
});

describe("serializeLayout", () => {
  it("gera sempre o mesmo texto para o mesmo layout (ordem estável)", () => {
    const a = setHidden(setHidden(defaultLayout(), "Start", true), "A", true);
    const b = setHidden(setHidden(defaultLayout(), "A", true), "Start", true);
    expect(serializeLayout(a)).toBe(serializeLayout(b));
  });

  it("o layout padrão é pequeno", () => {
    expect(serializeLayout(defaultLayout())).toBe('{"version":1,"groups":{},"hidden":[]}');
  });
});

describe("posição individual dos controles (grupo destravado)", () => {
  it("o padrão não tem deslocamento de controle e continua o mesmo de antes", () => {
    expect(controlOffset(defaultLayout(), "A")).toEqual({ dx: 0, dy: 0 });
    expect(defaultLayout().controls).toBeUndefined();
    expect(serializeLayout(defaultLayout())).toBe('{"version":1,"groups":{},"hidden":[]}');
  });

  it("withControl muda só o controle pedido, sem mexer no original", () => {
    const base = defaultLayout();
    const l = withControl(base, "A", { dx: 0.1, dy: -0.05 });
    expect(controlOffset(l, "A")).toEqual({ dx: 0.1, dy: -0.05 });
    expect(controlOffset(l, "B")).toEqual({ dx: 0, dy: 0 });
    expect(controlOffset(base, "A")).toEqual({ dx: 0, dy: 0 });
  });

  it("aceita mudar só um dos eixos e mantém o outro", () => {
    const l = withControl(withControl(defaultLayout(), "X", { dx: 0.2, dy: 0.1 }), "X", { dx: -0.3 });
    expect(controlOffset(l, "X")).toEqual({ dx: -0.3, dy: 0.1 });
  });

  it("limita a ±1 e arredonda a 4 casas", () => {
    const l = withControl(defaultLayout(), "A", { dx: 5, dy: 0.123456789 });
    expect(controlOffset(l, "A")).toEqual({ dx: 1, dy: 0.1235 });
  });

  it("deslocamento zero é neutro e some do layout", () => {
    const l = withControl(withControl(defaultLayout(), "A", { dx: 0.1 }), "A", { dx: 0, dy: 0 });
    expect(l.controls).toBeUndefined();
    expect(isDefault(l)).toBe(true);
  });

  it("um deslocamento de controle tira o layout de padrão", () => {
    expect(isDefault(withControl(defaultLayout(), "Up", { dy: 0.1 }))).toBe(false);
  });

  it("normalize descarta controle desconhecido, NaN e infinito", () => {
    const sujo = {
      ...defaultLayout(),
      controls: { A: { dx: NaN, dy: Infinity }, Z: { dx: 0.1, dy: 0 }, B: { dx: 0.2, dy: 0 } },
    } as unknown as Layout;
    const l = normalizeLayout(sujo);
    expect(l.controls).toEqual({ B: { dx: 0.2, dy: 0 } });
  });

  it("serialize e parse devolvem o mesmo layout, em ordem estável", () => {
    const l = withControl(withControl(defaultLayout(), "Y", { dx: 0.1 }), "A", { dy: 0.2 });
    const json = serializeLayout(l);
    expect(json.indexOf('"A"')).toBeLessThan(json.indexOf('"Y"')); // ordem do catálogo
    expect(parseLayout(JSON.parse(json)).layout).toEqual(l);
  });

  it("parse descarta entradas ruins de controle uma a uma", () => {
    const r = parseLayout({
      version: 1,
      groups: {},
      hidden: [],
      controls: { A: { dx: 0.1, dy: 0 }, B: "x", X: { dx: "1", dy: 0 }, Y: null, __proto__: { Start: { dx: 1, dy: 1 } } },
    });
    expect(r.status).toBe("ok");
    expect(r.layout.controls).toEqual({ A: { dx: 0.1, dy: 0 } });
  });

  it("controls que não é objeto é ignorado", () => {
    expect(parseLayout({ version: 1, groups: {}, hidden: [], controls: [1, 2] }).layout).toEqual(defaultLayout());
    expect(parseLayout({ version: 1, groups: {}, hidden: [], controls: 7 }).layout).toEqual(defaultLayout());
  });

  it("resetGroup também desfaz o deslocamento individual dos controles daquele grupo", () => {
    let l = withControl(defaultLayout(), "A", { dx: 0.1 });
    l = withControl(l, "Up", { dy: 0.1 });
    l = resetGroup(withGroup(l, "face", { dx: 0.2 }), "face");
    expect(controlOffset(l, "A")).toEqual({ dx: 0, dy: 0 });
    expect(controlOffset(l, "Up")).toEqual({ dx: 0, dy: 0.1 });
    expect(l.groups.face).toBeUndefined();
  });

  it("outras operações preservam os deslocamentos de controle", () => {
    const l = withControl(defaultLayout(), "A", { dx: 0.1 });
    expect(controlOffset(withGroup(l, "menu", { dx: 0.1 }), "A").dx).toBe(0.1);
    expect(controlOffset(setHidden(l, "B", true), "A").dx).toBe(0.1);
  });
});

describe("opacidade dos botões", () => {
  it("o padrão é opaco e continua serializando como antes", () => {
    expect(opacityOf(defaultLayout())).toBe(1);
    expect(defaultLayout().opacity).toBeUndefined();
    expect(serializeLayout(defaultLayout())).toBe('{"version":1,"groups":{},"hidden":[]}');
    expect(MIN_OPACITY).toBe(0.2);
  });

  it("withOpacity muda só a opacidade, sem mexer no original", () => {
    const base = withGroup(defaultLayout(), "face", { dx: 0.1 });
    const l = withOpacity(base, 0.6);
    expect(opacityOf(l)).toBe(0.6);
    expect(opacityOf(base)).toBe(1);
    expect(l.groups.face?.dx).toBe(0.1);
  });

  it("limita entre o mínimo (20%) e 100%, e arredonda a 2 casas", () => {
    expect(opacityOf(withOpacity(defaultLayout(), 0))).toBe(MIN_OPACITY);
    expect(opacityOf(withOpacity(defaultLayout(), -3))).toBe(MIN_OPACITY);
    expect(opacityOf(withOpacity(defaultLayout(), 7))).toBe(1);
    expect(opacityOf(withOpacity(defaultLayout(), 0.456789))).toBe(0.46);
  });

  it("NaN e infinito voltam ao opaco", () => {
    expect(opacityOf(withOpacity(defaultLayout(), NaN))).toBe(1);
    expect(opacityOf(withOpacity(defaultLayout(), Infinity))).toBe(1);
  });

  it("100% é o padrão: o campo some do layout", () => {
    const l = withOpacity(withOpacity(defaultLayout(), 0.5), 1);
    expect(l.opacity).toBeUndefined();
    expect(isDefault(l)).toBe(true);
  });

  it("uma opacidade menor que 100% tira o layout de padrão", () => {
    expect(isDefault(withOpacity(defaultLayout(), 0.8))).toBe(false);
  });

  it("serialize e parse devolvem o mesmo layout", () => {
    const l = withOpacity(withGroup(defaultLayout(), "menu", { dy: 0.1 }), 0.65);
    expect(JSON.parse(serializeLayout(l)).opacity).toBe(0.65);
    expect(parseLayout(JSON.parse(serializeLayout(l))).layout).toEqual(l);
  });

  it("parse descarta opacidade que não é número e limita a que está fora da faixa", () => {
    const base = { version: 1, groups: {}, hidden: [] };
    expect(parseLayout({ ...base, opacity: "0.5" }).layout.opacity).toBeUndefined();
    expect(parseLayout({ ...base, opacity: null }).layout.opacity).toBeUndefined();
    expect(parseLayout({ ...base, opacity: 0.01 }).layout.opacity).toBe(MIN_OPACITY);
    expect(parseLayout({ ...base, opacity: 5 }).layout.opacity).toBeUndefined(); // vira 1 = padrão
    expect(parseLayout({ ...base, opacity: 0.5 }).status).toBe("ok");
  });

  it("outras operações preservam a opacidade", () => {
    const l = withOpacity(defaultLayout(), 0.5);
    expect(opacityOf(withGroup(l, "face", { dx: 0.1 }))).toBe(0.5);
    expect(opacityOf(setHidden(l, "A", true))).toBe(0.5);
    expect(opacityOf(resetGroup(withGroup(l, "face", { dx: 0.1 }), "face"))).toBe(0.5);
    expect(opacityOf(withControl(l, "A", { dx: 0.1 }))).toBe(0.5);
  });
});
