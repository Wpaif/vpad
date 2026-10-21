/// <reference types="vite/client" />

// Componentes Svelte importados por arquivos .ts (o editor os resolve por conta própria).
declare module "*.svelte" {
  import type { Component } from "svelte";
  const component: Component<any>;
  export default component;
}
