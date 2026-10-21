//! Consome os eventos do celular e os entrega ao controle virtual, trocando o
//! dispositivo quando o celular pede outro perfil.

use std::{io, sync::Arc};

use tokio::sync::{mpsc::UnboundedReceiver, watch};

use crate::{
    mapa::Mapa,
    perfil::{PERFIS, Perfil},
    protocolo::Evento,
};

/// Destino dos eventos: o gamepad virtual de verdade (uinput) ou um falso nos testes.
pub trait Dispositivo: Send {
    fn aplicar(&mut self, evento: Evento) -> io::Result<()>;

    /// O mapa de teclas mudou. Só o teclado usa; os gamepads ignoram.
    fn definir_mapa(&mut self, _mapa: &Mapa) {}
}

/// Cria o dispositivo de um perfil.
pub type Fabrica = Box<dyn Fn(&'static Perfil) -> io::Result<Box<dyn Dispositivo>> + Send>;

/// Roda até o canal fechar. `perfil` carrega o índice do perfil ativo e é publicado a
/// cada pedido de troca (inclusive os que falham), para o celular nunca ficar no escuro.
/// `mapa` é o mapa de teclas, entregue ao dispositivo ativo ao começar, a cada mudança e
/// a cada dispositivo novo.
pub async fn processar(
    mut eventos: UnboundedReceiver<Evento>,
    perfil: watch::Sender<u8>,
    mut mapa: watch::Receiver<Arc<Mapa>>,
    fabrica: Fabrica,
    mut dispositivo: Box<dyn Dispositivo>,
) {
    let atual = mapa.borrow_and_update().clone();
    dispositivo.definir_mapa(&atual);

    loop {
        tokio::select! {
            // o mapa novo vale antes dos eventos que já estavam na fila
            biased;
            Ok(()) = mapa.changed() => {
                let novo = mapa.borrow_and_update().clone();
                dispositivo.definir_mapa(&novo);
            }
            evento = eventos.recv() => {
                let Some(evento) = evento else { break };
                let Evento::TrocarPerfil(pedido) = evento else {
                    if let Err(e) = dispositivo.aplicar(evento) {
                        eprintln!("erro ao enviar {evento:?} ao controle virtual: {e}");
                    }
                    continue;
                };

                let atual = *perfil.borrow();
                let mut escolhido = atual;
                if pedido != atual {
                    match PERFIS.get(pedido as usize).map(|p| fabrica(p)) {
                        Some(Ok(mut novo)) => {
                            // O dispositivo antigo some junto com os botões que estivessem pressionados.
                            let _ = dispositivo.aplicar(Evento::SoltarTudo);
                            let mapa_atual = mapa.borrow().clone();
                            novo.definir_mapa(&mapa_atual);
                            dispositivo = novo;
                            escolhido = pedido;
                        }
                        Some(Err(e)) => eprintln!("não consegui trocar de perfil: {e}"),
                        None => {}
                    }
                }
                perfil.send_replace(escolhido);
            }
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::perfil::PERFIS;
    use std::sync::{Arc, Mutex};
    use tokio::sync::{mpsc, watch};

    /// Dispositivo falso: registra o que recebeu, rotulado com o perfil.
    type Registro = Arc<Mutex<Vec<(&'static str, Evento)>>>;

    type Mapas = Arc<Mutex<Vec<&'static str>>>;

    struct Falso {
        perfil: &'static str,
        registro: Registro,
        mapas: Mapas,
    }

    impl Dispositivo for Falso {
        fn aplicar(&mut self, evento: Evento) -> std::io::Result<()> {
            self.registro.lock().unwrap().push((self.perfil, evento));
            Ok(())
        }

        fn definir_mapa(&mut self, _mapa: &Mapa) {
            self.mapas.lock().unwrap().push(self.perfil);
        }
    }

    fn fabrica(registro: Registro, mapas: Mapas, falhar_em: Option<&'static str>) -> Fabrica {
        Box::new(move |p| {
            if Some(p.id) == falhar_em {
                return Err(std::io::Error::other("sem permissão"));
            }
            Ok(Box::new(Falso { perfil: p.id, registro: registro.clone(), mapas: mapas.clone() }))
        })
    }

    struct Resultado {
        eventos: Vec<(&'static str, Evento)>,
        perfis: Vec<u8>,
        mapas: Vec<&'static str>,
    }

    async fn rodar(eventos: Vec<Evento>, inicial: u8, falhar_em: Option<&'static str>) -> (Vec<(&'static str, Evento)>, Vec<u8>) {
        let r = rodar_completo(eventos, inicial, falhar_em, false).await;
        (r.eventos, r.perfis)
    }

    /// Roda o serviço sobre `eventos`; com `mudar_mapa`, um mapa novo já está à espera no início.
    async fn rodar_completo(
        eventos: Vec<Evento>,
        inicial: u8,
        falhar_em: Option<&'static str>,
        mudar_mapa: bool,
    ) -> Resultado {
        let registro: Registro = Default::default();
        let mapas: Mapas = Default::default();
        let (tx, rx) = mpsc::unbounded_channel();
        let (perfil_tx, mut perfil_rx) = watch::channel(inicial);
        let (mapa_tx, mapa_rx) = watch::channel(Arc::new(Mapa::padrao()));
        let fab = fabrica(registro.clone(), mapas.clone(), falhar_em);
        let inicial_dev = fab(PERFIS[inicial as usize]).unwrap();

        let publicados = Arc::new(Mutex::new(vec![]));
        let p2 = publicados.clone();
        let ouvinte = tokio::spawn(async move {
            while perfil_rx.changed().await.is_ok() {
                p2.lock().unwrap().push(*perfil_rx.borrow_and_update());
            }
        });

        for e in eventos {
            tx.send(e).unwrap();
        }
        let servico = tokio::spawn(processar(rx, perfil_tx, mapa_rx, fab, inicial_dev));
        if mudar_mapa {
            // deixa o serviço começar e esperar, e só então publica o mapa novo
            for _ in 0..3 {
                tokio::task::yield_now().await;
            }
            mapa_tx.send_replace(Arc::new(Mapa::default()));
            for _ in 0..3 {
                tokio::task::yield_now().await;
            }
        }
        drop(tx);
        servico.await.unwrap();
        ouvinte.await.unwrap();

        let eventos = registro.lock().unwrap().clone();
        let perfis = publicados.lock().unwrap().clone();
        let mapas = mapas.lock().unwrap().clone();
        Resultado { eventos, perfis, mapas }
    }

    fn botao(pressionado: bool) -> Evento {
        Evento::Botao { id: 0, pressionado }
    }

    #[tokio::test]
    async fn eventos_vao_para_o_dispositivo_do_perfil_inicial() {
        let (regs, pubs) = rodar(vec![botao(true)], 0, None).await;
        assert_eq!(regs, vec![("xbox360", botao(true))]);
        assert!(pubs.is_empty());
    }

    #[tokio::test]
    async fn trocar_perfil_cria_novo_dispositivo_e_avisa_o_celular() {
        let (regs, pubs) = rodar(
            vec![botao(true), Evento::TrocarPerfil(1), botao(false)],
            0,
            None,
        )
        .await;
        assert_eq!(regs.last(), Some(&("ds4", botao(false))));
        assert_eq!(pubs, vec![1]);
    }

    #[tokio::test]
    async fn antes_de_trocar_solta_tudo_no_dispositivo_antigo() {
        let (regs, _) = rodar(vec![Evento::TrocarPerfil(2)], 0, None).await;
        assert!(regs.contains(&("xbox360", Evento::SoltarTudo)));
    }

    #[tokio::test]
    async fn falha_ao_criar_mantem_o_perfil_antigo_e_reafirma_ao_celular() {
        let (regs, pubs) = rodar(
            vec![Evento::TrocarPerfil(1), botao(true)],
            0,
            Some("ds4"),
        )
        .await;
        assert_eq!(regs, vec![("xbox360", botao(true))]);
        assert_eq!(pubs, vec![0]);
    }

    #[tokio::test]
    async fn trocar_para_o_mesmo_perfil_so_confirma() {
        let (regs, pubs) = rodar(vec![Evento::TrocarPerfil(0)], 0, None).await;
        assert!(regs.is_empty());
        assert_eq!(pubs, vec![0]);
    }

    #[tokio::test]
    async fn o_dispositivo_inicial_recebe_o_mapa_atual_ao_comecar() {
        let r = rodar_completo(vec![], 3, None, false).await;
        assert_eq!(r.mapas, vec!["teclado"]);
    }

    #[tokio::test]
    async fn mapa_novo_vai_para_o_dispositivo_ativo() {
        let r = rodar_completo(vec![], 3, None, true).await;
        assert_eq!(r.mapas, vec!["teclado", "teclado"]);
    }

    #[tokio::test]
    async fn dispositivo_criado_na_troca_recebe_o_mapa_atual() {
        let r = rodar_completo(vec![Evento::TrocarPerfil(3)], 0, None, false).await;
        assert_eq!(r.mapas, vec!["xbox360", "teclado"]);
    }
}
