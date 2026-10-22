import { isDefault, serializeLayout, type Layout } from "./layoutModel";
import { isPending as storedPending, setPending as storePending } from "./layoutStorage";
import { fetchRemoteLayout, pushRemoteLayout, type PushResult, type RemoteLayout } from "./layoutRemote";

/**
 * Mantém uma cópia do layout no PC, para ele sobreviver a reinstalar o app da Tela de Início
 * (cujo armazenamento é separado do Safari e pode ser apagado). A regra é simples:
 * - o que o usuário salvou é enviado ao PC; sem rede, fica marcado como pendente e vai depois;
 * - ao abrir, o layout do PC é adotado, a menos que haja um envio pendente (o local vence);
 * - o PC sem layout recebe o local, se ele for diferente do padrão.
 */

export type SyncState = "none" | "synced" | "pending" | "rejected";

interface Store {
  readonly saved: Layout;
  readonly editing: boolean;
  adopt: (layout: Layout) => boolean;
}

interface Options {
  store: Store;
  token: string;
  fetchRemote?: (token: string) => Promise<RemoteLayout>;
  pushRemote?: (layout: Layout, token: string) => Promise<PushResult>;
  isPending?: () => boolean;
  setPending?: (pending: boolean) => void;
  onState?: (state: SyncState) => void;
}

export interface LayoutSync {
  /** ao abrir o app */
  start: () => Promise<void>;
  /** depois de o usuário salvar */
  saved: () => Promise<void>;
  /** ao voltar ao app ou à rede */
  refresh: () => Promise<void>;
}

export function createLayoutSync({
  store,
  token,
  fetchRemote = fetchRemoteLayout,
  pushRemote = pushRemoteLayout,
  isPending = storedPending,
  setPending = storePending,
  onState,
}: Options): LayoutSync {
  let state: SyncState = "none";
  const set = (next: SyncState) => {
    state = next;
    onState?.(state);
  };

  // uma coisa de cada vez: salvar e reler não se atropelam
  let chain: Promise<void> = Promise.resolve();
  const enqueue = (job: () => Promise<void>) => {
    const run = chain.then(job);
    chain = run.catch(() => {});
    return run;
  };

  const sendLocal = async () => {
    const result = await pushRemote(store.saved, token);
    if (result === "ok") {
      setPending(false);
      set("synced");
    } else if (result === "rejected") {
      // o PC não aceita este layout: tentar de novo não adianta
      setPending(false);
      set("rejected");
    } else {
      setPending(true);
      set("pending");
    }
  };

  const reconcile = async () => {
    if (isPending()) return sendLocal(); // o que foi salvo aqui e não chegou ao PC vale mais
    const remote = await fetchRemote(token);
    if (remote.status !== "ok") return onState?.(state);
    if (remote.layout === null) {
      if (isDefault(store.saved)) return set("none");
      return sendLocal(); // migração: o PC ainda não tem nenhum
    }
    if (serializeLayout(remote.layout) !== serializeLayout(store.saved)) store.adopt(remote.layout);
    set("synced");
  };

  return {
    start: () => enqueue(reconcile),
    refresh: () => enqueue(reconcile),
    saved() {
      setPending(true); // marcado já: sobrevive a fechar o app antes da resposta
      return enqueue(sendLocal);
    },
  };
}
