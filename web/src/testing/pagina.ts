import { readFileSync } from "node:fs";
import { mount, unmount } from "svelte";
import Pad from "../Pad.svelte";

let montado: ReturnType<typeof mount> | null = null;

/** Carrega o index.html e monta o pad (que agora é um componente) no lugar dele. */
export function montarPagina(): HTMLElement {
  if (montado) unmount(montado);
  document.documentElement.innerHTML = readFileSync("index.html", "utf8");
  montado = mount(Pad, { target: document.getElementById("app")! });
  return document.getElementById("pad")!;
}
