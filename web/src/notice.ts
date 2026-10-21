import type { ProfileNotice } from "./protocol";

/** Texto do aviso sobre o pedido de ativar o modo teclado. O id só entra se tiver o formato esperado. */
export function describeNotice(notice: ProfileNotice, deviceId: string | null): string {
  const id = deviceId !== null && /^[0-9a-f]{8}$/.test(deviceId) ? deviceId : null;
  switch (notice) {
    case "no-permission":
      return id
        ? `Este aparelho não tem permissão para o modo teclado. No PC: vpad --permitir-teclado ${id}`
        : "Este aparelho não tem permissão para o modo teclado. No PC, veja o id dele com vpad --dispositivos e libere com vpad --permitir-teclado <id>";
    case "waiting":
      return "Aguardando a confirmação no PC: olhe o terminal do vpad.";
    case "denied":
      return "O modo teclado foi recusado no PC.";
    case "expired":
      return "Ninguém confirmou no PC a tempo. Tente de novo.";
    case "busy":
      return "O PC está esperando resposta a outro pedido. Tente de novo em instantes.";
  }
}
