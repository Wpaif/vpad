import { beforeEach, describe, expect, it } from "vitest";
import { montarPagina } from "./testing/pagina";

beforeEach(() => {
  montarPagina();
});

describe("barra superior", () => {
  it("indicador de latência e botão de ocultar ficam juntos na mesma barra", () => {
    const bar = document.querySelector(".topbar");
    expect(bar).not.toBeNull();
    expect(bar!.contains(document.getElementById("status"))).toBe(true);
    expect(bar!.contains(document.getElementById("toggle"))).toBe(true);
  });

  it("a barra fica fora do controle, para continuar visível quando ele é ocultado", () => {
    expect(document.getElementById("pad")!.contains(document.querySelector(".topbar"))).toBe(false);
  });
});

describe("seletor de perfil", () => {
  it("fica dentro do painel que só abre ao tocar no indicador, sem ocupar espaço fixo", () => {
    const picker = document.getElementById("profile-picker");
    expect(picker).not.toBeNull();
    expect(document.getElementById("stats")!.contains(picker)).toBe(true);
  });
});

describe("editor de teclas", () => {
  it("o botão para abri-lo fica dentro do painel, sem ocupar espaço fixo", () => {
    const button = document.getElementById("open-key-editor");
    expect(button).not.toBeNull();
    expect(document.getElementById("stats")!.contains(button)).toBe(true);
  });

  it("o editor começa escondido e fica fora do controle", () => {
    const editor = document.getElementById("key-editor")!;
    expect(editor.hidden).toBe(true);
    expect(editor.getAttribute("role")).toBe("dialog");
    expect(document.getElementById("pad")!.contains(editor)).toBe(false);
  });
});

describe("tela de pareamento no HTML", () => {
  it("existe, começa escondida e fica fora do controle", () => {
    const pair = document.getElementById("pair")!;
    expect(pair.hidden).toBe(true);
    expect(pair.getAttribute("role")).toBe("dialog");
    expect(document.getElementById("pad")!.contains(pair)).toBe(false);
  });

  it("tem um campo numérico de seis dígitos e o botão de parear", () => {
    const input = document.querySelector<HTMLInputElement>("#pair input")!;
    expect(input.getAttribute("inputmode")).toBe("numeric");
    expect(input.getAttribute("maxlength")).toBe("6");
    expect(input.getAttribute("autocomplete")).toBe("one-time-code");
    expect(document.querySelector("#pair button[type=submit]")).not.toBeNull();
    expect(document.querySelector("#pair [role=alert]")).not.toBeNull();
  });

  it("o antigo bloqueio de 'outro aparelho está controlando' não existe mais", () => {
    expect(document.getElementById("gate")).toBeNull();
  });
});

describe("dica de instalação", () => {
  it("avisa que o app instalado precisa ser pareado de novo, com o código do terminal", () => {
    expect(document.getElementById("install-hint")!.textContent).toMatch(/parear|código/i);
  });
});

describe("aviso do modo teclado no painel", () => {
  it("fica dentro do painel, junto do seletor de perfil", () => {
    const nota = document.getElementById("profile-note");
    expect(nota).not.toBeNull();
    expect(document.getElementById("stats")!.contains(nota)).toBe(true);
    expect(nota!.getAttribute("role")).toBe("status");
  });
});

describe("painel de latência", () => {
  it("tem o interruptor do ping curto dentro do painel", () => {
    const toggle = document.getElementById("ping-toggle");
    expect(toggle).not.toBeNull();
    expect(toggle!.tagName).toBe("BUTTON");
    expect(document.getElementById("stats")!.contains(toggle)).toBe(true);
  });
});

describe("editor de layout na página", () => {
  it("o painel tem o botão que abre o modo edição", () => {
    const edit = document.getElementById("edit-layout");
    expect(edit?.tagName).toBe("BUTTON");
    expect(document.getElementById("stats")!.contains(edit)).toBe(true);
  });

  it("a barra de edição começa escondida e tem salvar, cancelar e restaurar", () => {
    const bar = document.getElementById("layout-bar")!;
    expect(bar.hidden).toBe(true);
    for (const act of ["save", "cancel", "reset"]) {
      expect(bar.querySelector(`button[data-act=${act}]`)).not.toBeNull();
    }
  });
});

describe("lista de botões na página", () => {
  it("a barra tem o botão que abre a lista, e a lista começa escondida", () => {
    const bar = document.getElementById("layout-bar")!;
    expect(bar.querySelector("button[data-act=buttons]")).not.toBeNull();
    const list = document.getElementById("hide-list")!;
    expect(list.hidden).toBe(true);
  });
});

describe("estado do layout no PC, na página", () => {
  it("o painel tem o texto de estado, que começa vazio", () => {
    const note = document.getElementById("layout-sync");
    expect(note).not.toBeNull();
    expect(document.getElementById("stats")!.contains(note)).toBe(true);
    expect(note!.textContent).toBe("");
    expect(note!.getAttribute("role")).toBe("status");
  });
});

describe("opacidade na barra de edição", () => {
  it("a barra tem um controle deslizante de 20% a 100%, de 5 em 5, que começa em 100%", () => {
    const input = document.querySelector<HTMLInputElement>("#layout-bar #layout-opacity");
    expect(input).not.toBeNull();
    expect(input!.type).toBe("range");
    expect([input!.min, input!.max, input!.step, input!.value]).toEqual(["20", "100", "5", "100"]);
    expect(input!.getAttribute("aria-label")).toBe("Opacidade dos botões");
  });
});

describe("seletor de idioma na página", () => {
  it("o painel tem o seletor de idioma, com nome acessível", () => {
    const select = document.getElementById("language-select");
    expect(select?.tagName).toBe("SELECT");
    expect(document.getElementById("stats")!.contains(select)).toBe(true);
    expect(select!.getAttribute("aria-label")).toBe("Idioma");
    expect(select!.getAttribute("data-i18n-aria-label")).toBe("language.label");
  });
});

describe("painel de latência rola quando não cabe", () => {
  it("o painel é uma área de rolagem (data-scroll libera o gesto de rolar no iPhone)", () => {
    expect(document.getElementById("stats")!.hasAttribute("data-scroll")).toBe(true);
  });
});
