import { CONTROLS, CONTROL_GROUPS, groupName } from "./controls";
import { t } from "./i18n";
import { keyGroupName, keyLabel } from "./keyNames";
import type { KeyboardConfig, KeyInfo, KeyMap } from "./keyboard";

export interface EditorDeps {
  load: () => Promise<KeyboardConfig>;
  save: (mapa: KeyMap) => Promise<KeyMap>;
  onSaved: (mapa: KeyMap, config: KeyboardConfig) => void;
}

export interface KeyEditor {
  open: () => Promise<void>;
  close: () => void;
}

/** Montado na hora de abrir (o idioma só é escolhido depois do carregamento do módulo). */
const template = () => `
  <header class="editor-bar">
    <h2>${t("editor.title")}</h2>
    <button type="button" data-action="reset">${t("editor.reset")}</button>
    <button type="button" data-action="close" aria-label="${t("editor.close")}">×</button>
  </header>
  <p class="editor-status" role="status"></p>
  <div class="editor-body" data-scroll></div>
  <div class="key-picker" hidden>
    <header class="editor-bar">
      <h2 data-role="picker-title"></h2>
      <button type="button" data-action="none">${t("editor.picker.none")}</button>
      <button type="button" data-action="back">${t("editor.picker.back")}</button>
    </header>
    <div class="picker-keys" data-scroll></div>
  </div>`;

const el = <T extends HTMLElement>(tag: string, className?: string, text?: string): T => {
  const node = document.createElement(tag) as T;
  if (className) node.className = className;
  if (text !== undefined) node.textContent = text;
  return node;
};

/** Editor de teclas: lista os controles com a tecla de cada um e deixa trocá-la (salva na hora). */
export function mountKeyEditor(root: HTMLElement, deps: EditorDeps): KeyEditor {
  root.innerHTML = template();
  const body = root.querySelector<HTMLElement>(".editor-body")!;
  const statusEl = root.querySelector<HTMLElement>(".editor-status")!;
  const picker = root.querySelector<HTMLElement>(".key-picker")!;
  const pickerTitle = root.querySelector<HTMLElement>("[data-role=picker-title]")!;
  const pickerKeys = root.querySelector<HTMLElement>(".picker-keys")!;

  let config: KeyboardConfig | null = null;
  let mapa: KeyMap = {};
  let editing: string | null = null;

  const setStatus = (text: string) => {
    statusEl.textContent = text;
  };
  const keyName = (id: string | undefined) =>
    id === undefined ? "–" : (() => {
      const key = config?.teclas.find((k) => k.id === id);
      return key ? keyLabel(key) : id;
    })();
  const controlLabel = (id: string) => CONTROLS.find((c) => c.id === id)?.label ?? id;

  function renderRows() {
    const available = new Set(config?.controles ?? []);
    const sections = CONTROL_GROUPS.map((group) => {
      const section = el("section");
      section.append(el("h3", undefined, groupName(group)));
      for (const control of CONTROLS.filter((c) => c.group === group && available.has(c.id))) {
        const row = el<HTMLButtonElement>("button", "row");
        row.type = "button";
        row.dataset.control = control.id;
        row.append(el("span", undefined, control.label), el("kbd", undefined, keyName(mapa[control.id])));
        section.append(row);
      }
      return section;
    });
    body.replaceChildren(...sections);
  }

  function renderKeys(current: string | undefined) {
    const groups = new Map<string, KeyInfo[]>();
    for (const key of config?.teclas ?? []) groups.set(key.grupo, [...(groups.get(key.grupo) ?? []), key]);
    const sections = [...groups].map(([group, keys]) => {
      const section = el("section");
      section.append(el("h4", undefined, keyGroupName(group)));
      const grid = el("div", "keys");
      for (const key of keys) {
        const b = el<HTMLButtonElement>("button", undefined, keyLabel(key));
        b.type = "button";
        b.dataset.key = key.id;
        b.setAttribute("aria-label", key.id);
        b.setAttribute("aria-pressed", String(key.id === current));
        grid.append(b);
      }
      section.append(grid);
      return section;
    });
    pickerKeys.replaceChildren(...sections);
    pickerKeys.scrollTop = 0;
  }

  /** Grava `next`; em caso de erro mantém o mapa antigo e a lista de teclas aberta. */
  async function commit(next: KeyMap, warning = "") {
    if (!config) return;
    try {
      mapa = await deps.save(next);
    } catch (e) {
      setStatus(e instanceof Error ? e.message : t("editor.error.save"));
      return;
    }
    picker.hidden = true;
    editing = null;
    renderRows();
    setStatus(warning);
    deps.onSaved(mapa, { ...config, mapa });
  }

  function choose(keyId: string | null) {
    if (editing === null) return;
    const next = { ...mapa };
    if (keyId === null) delete next[editing];
    else next[editing] = keyId;

    const others =
      keyId === null
        ? []
        : Object.entries(mapa)
            .filter(([control, key]) => key === keyId && control !== editing)
            .map(([control]) => controlLabel(control));
    void commit(next, others.length ? t("editor.warning.sharedKey", { others: others.join(", ") }) : "");
  }

  body.addEventListener("click", (e) => {
    const row = (e.target as Element).closest<HTMLElement>("[data-control]");
    if (!row?.dataset.control) return;
    editing = row.dataset.control;
    pickerTitle.textContent = t("editor.picker.title", { control: controlLabel(editing) });
    renderKeys(mapa[editing]);
    picker.hidden = false;
  });

  pickerKeys.addEventListener("click", (e) => {
    const key = (e.target as Element).closest<HTMLElement>("[data-key]");
    if (key?.dataset.key) choose(key.dataset.key);
  });

  root.addEventListener("click", (e) => {
    const action = (e.target as Element).closest<HTMLElement>("[data-action]")?.dataset.action;
    if (action === "close") api.close();
    else if (action === "back") picker.hidden = true;
    else if (action === "none") choose(null);
    else if (action === "reset" && config) void commit({ ...config.padrao });
  });

  const api: KeyEditor = {
    async open() {
      root.hidden = false;
      picker.hidden = true;
      setStatus(t("editor.loading"));
      body.replaceChildren();
      try {
        config = await deps.load();
      } catch (e) {
        config = null;
        setStatus(e instanceof Error ? e.message : t("editor.error.load"));
        return;
      }
      mapa = config.mapa;
      setStatus("");
      renderRows();
    },
    close() {
      root.hidden = true;
    },
  };
  return api;
}
