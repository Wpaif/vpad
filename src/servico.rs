//! Consome os eventos do celular e os entrega ao controle virtual, trocando o
//! dispositivo quando o celular pede outro perfil.

use std::io;

use tokio::sync::{mpsc::UnboundedReceiver, watch};

use crate::{perfil::{PERFIS, Perfil}, protocolo::Evento};

/// Destino dos eventos: o gamepad virtual de verdade (uinput) ou um falso nos testes.
pub trait Dispositivo: Send {
    fn aplicar(&mut self, evento: Evento) -> io::Result<()>;
}

/// Cria o dispositivo de um perfil.
pub type Fabrica = Box<dyn Fn(&'static Perfil) -> io::Result<Box<dyn Dispositivo>> + Send>;

/// Roda até o canal fechar. `perfil` carrega o índice do perfil ativo e é publicado a
/// cada pedido de troca (inclusive os que falham), para o celular nunca ficar no escuro.
pub async fn processar(
    mut eventos: UnboundedReceiver<Evento>,
    perfil: watch::Sender<u8>,
    fabrica: Fabrica,
    mut dispositivo: Box<dyn Dispositivo>,
) {
    while let Some(evento) = eventos.recv().await {
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
                Some(Ok(novo)) => {
                    // O dispositivo antigo some junto com os botões que estivessem pressionados.
                    let _ = dispositivo.aplicar(Evento::SoltarTudo);
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

#[cfg(test)]
mod testes {
    use super::*;
    use crate::perfil::PERFIS;
    use std::sync::{Arc, Mutex};
    use tokio::sync::{mpsc, watch};

    /// Dispositivo falso: registra o que recebeu, rotulado com o perfil.
    type Registro = Arc<Mutex<Vec<(&'static str, Evento)>>>;

    struct Falso {
        perfil: &'static str,
        registro: Registro,
    }

    impl Dispositivo for Falso {
        fn aplicar(&mut self, evento: Evento) -> std::io::Result<()> {
            self.registro.lock().unwrap().push((self.perfil, evento));
            Ok(())
        }
    }

    fn fabrica(registro: Registro, falhar_em: Option<&'static str>) -> Fabrica {
        Box::new(move |p| {
            if Some(p.id) == falhar_em {
                return Err(std::io::Error::other("sem permissão"));
            }
            Ok(Box::new(Falso { perfil: p.id, registro: registro.clone() }))
        })
    }

    /// Roda o serviço sobre `eventos` e devolve o registro e o que foi publicado ao celular.
    async fn rodar(
        eventos: Vec<Evento>,
        inicial: u8,
        falhar_em: Option<&'static str>,
    ) -> (Vec<(&'static str, Evento)>, Vec<u8>) {
        let registro: Registro = Default::default();
        let (tx, rx) = mpsc::unbounded_channel();
        let (perfil_tx, mut perfil_rx) = watch::channel(inicial);
        let fab = fabrica(registro.clone(), falhar_em);
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
        drop(tx);
        processar(rx, perfil_tx, fab, inicial_dev).await;
        ouvinte.await.unwrap();

        let regs = registro.lock().unwrap().clone();
        let pubs = publicados.lock().unwrap().clone();
        (regs, pubs)
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
}
