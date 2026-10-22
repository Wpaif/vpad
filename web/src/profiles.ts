import { t } from "./i18n";

/**
 * Perfis de controle, na mesma ordem de PERFIS em src/perfil.rs: o índice é o que
 * trafega no protocolo. `short` cabe no seletor compacto; `label` é o nome completo.
 * Só o teclado tem nome traduzido (os outros são marcas), e por isso é calculado na hora.
 */
export interface Profile {
  readonly id: string;
  readonly label: string;
  readonly short: string;
}

export const PROFILES: readonly Profile[] = [
  { id: "xbox360", label: "Xbox 360", short: "Xbox" },
  { id: "ds4", label: "DualShock 4", short: "PS4" },
  { id: "switch-pro", label: "Switch Pro", short: "Switch" },
  {
    id: "teclado",
    get label() {
      return t("profiles.keyboard");
    },
    get short() {
      return t("profiles.keyboard");
    },
  },
];
