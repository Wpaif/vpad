/**
 * Perfis de controle, na mesma ordem de PERFIS em src/perfil.rs: o índice é o que
 * trafega no protocolo. `short` cabe no seletor compacto; `label` é o nome completo.
 */
export const PROFILES = [
  { id: "xbox360", label: "Xbox 360", short: "Xbox" },
  { id: "ds4", label: "DualShock 4", short: "PS4" },
  { id: "switch-pro", label: "Switch Pro", short: "Switch" },
] as const;
