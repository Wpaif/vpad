//! Gamepad virtual no Linux (uinput), com a identidade de um controle Xbox 360
//! para que Steam, emuladores e jogos o reconheçam sem configuração.

use std::{io, path::PathBuf};

use evdev::{
    AbsInfo, AbsoluteAxisCode as Abs, AttributeSet, BusType, InputEvent, InputId, UinputAbsSetup,
    uinput::VirtualDevice,
};

use crate::{
    mapa::Mapa,
    mapeamento::{EIXOS, Tradutor, eixos_do_perfil, teclas_do_perfil},
    perfil::{Faixa, Perfil, Tipo, XBOX_360},
    protocolo::Evento,
    servico::Dispositivo,
    teclado::TradutorTeclado,
    teclas::TECLAS,
};

pub struct Gamepad {
    dispositivo: VirtualDevice,
    tradutor: Tradutor,
}

/// Mesmos parâmetros que o driver real do kernel declara, para o jogo calibrar igual.
fn info_do_eixo(perfil: &Perfil, eixo: Abs, faixa: Faixa) -> AbsInfo {
    let analogico = EIXOS[..4].contains(&eixo);
    match (analogico, perfil.id) {
        (false, _) => AbsInfo::new(0, faixa.min, faixa.max, 0, 0, 0),
        (true, "ds4") => AbsInfo::new(faixa.centro(), faixa.min, faixa.max, 0, 0, 0),
        (true, "switch-pro") => AbsInfo::new(0, faixa.min, faixa.max, 250, 500, 0),
        (true, _) => AbsInfo::new(0, faixa.min, faixa.max, 16, 128, 0),
    }
}

impl Gamepad {
    pub fn novo(perfil: &'static Perfil) -> io::Result<Self> {
        if perfil.tipo != Tipo::Gamepad {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "o perfil não é de gamepad"));
        }
        let mut teclas = AttributeSet::new();
        teclas_do_perfil(perfil).into_iter().for_each(|k| teclas.insert(k));

        let mut builder = VirtualDevice::builder()?
            .name(perfil.nome)
            .input_id(InputId::new(
                BusType::BUS_USB,
                perfil.fabricante,
                perfil.produto,
                perfil.versao,
            ))
            .with_keys(&teclas)?;

        for (eixo, faixa) in eixos_do_perfil(perfil) {
            let info = info_do_eixo(perfil, eixo, faixa);
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(eixo, info))?;
        }
        let hat = AbsInfo::new(0, -1, 1, 0, 0, 0);
        for eixo in [Abs::ABS_HAT0X, Abs::ABS_HAT0Y] {
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(eixo, hat))?;
        }

        Ok(Self { dispositivo: builder.build()?, tradutor: Tradutor::new(perfil) })
    }

    /// Controle Xbox 360, o perfil padrão.
    pub fn xbox_360() -> io::Result<Self> {
        Self::novo(&XBOX_360)
    }

    /// Caminhos /dev/input/eventN do dispositivo (o udev os cria de forma assíncrona).
    pub fn caminhos(&mut self) -> io::Result<Vec<PathBuf>> {
        self.dispositivo.enumerate_dev_nodes_blocking()?.collect()
    }

    /// Aplica o evento como um único lote, para o jogo ver o estado consistente.
    pub fn aplicar(&mut self, evento: Evento) -> io::Result<()> {
        let eventos: Vec<InputEvent> = self
            .tradutor
            .traduzir(evento)
            .into_iter()
            .map(|e| InputEvent::new(e.tipo, e.codigo, e.valor))
            .collect();
        if eventos.is_empty() {
            return Ok(());
        }
        self.dispositivo.emit(&eventos)
    }
}

impl Dispositivo for Gamepad {
    fn aplicar(&mut self, evento: Evento) -> io::Result<()> {
        Gamepad::aplicar(self, evento)
    }
}

/// Teclado virtual: declara todas as teclas do catálogo, para o mapa poder mudar sem
/// recriar o dispositivo.
pub struct Teclado {
    dispositivo: VirtualDevice,
    tradutor: TradutorTeclado,
}

impl Teclado {
    pub fn novo(perfil: &'static Perfil, mapa: Mapa) -> io::Result<Self> {
        if perfil.tipo != Tipo::Teclado {
            return Err(io::Error::new(io::ErrorKind::InvalidInput, "o perfil não é de teclado"));
        }
        let mut teclas = AttributeSet::new();
        TECLAS.iter().for_each(|t| teclas.insert(t.codigo));

        let dispositivo = VirtualDevice::builder()?
            .name(perfil.nome)
            .input_id(InputId::new(
                BusType::BUS_USB,
                perfil.fabricante,
                perfil.produto,
                perfil.versao,
            ))
            .with_keys(&teclas)?
            .build()?;
        Ok(Self { dispositivo, tradutor: TradutorTeclado::novo(mapa) })
    }

    pub fn caminhos(&mut self) -> io::Result<Vec<PathBuf>> {
        self.dispositivo.enumerate_dev_nodes_blocking()?.collect()
    }

    fn emitir(&mut self, eventos: Vec<crate::mapeamento::EventoLinux>) -> io::Result<()> {
        if eventos.is_empty() {
            return Ok(());
        }
        let eventos: Vec<InputEvent> =
            eventos.into_iter().map(|e| InputEvent::new(e.tipo, e.codigo, e.valor)).collect();
        self.dispositivo.emit(&eventos)
    }

    pub fn aplicar(&mut self, evento: Evento) -> io::Result<()> {
        let eventos = self.tradutor.traduzir(evento);
        self.emitir(eventos)
    }

    pub fn definir_mapa(&mut self, mapa: &Mapa) -> io::Result<()> {
        let soltas = self.tradutor.definir_mapa(mapa.clone());
        self.emitir(soltas)
    }
}

impl Dispositivo for Teclado {
    fn aplicar(&mut self, evento: Evento) -> io::Result<()> {
        Teclado::aplicar(self, evento)
    }

    fn definir_mapa(&mut self, mapa: &Mapa) {
        if let Err(e) = Teclado::definir_mapa(self, mapa) {
            eprintln!("erro ao aplicar o mapa de teclas: {e}");
        }
    }
}

#[cfg(test)]
mod testes {
    use super::*;
    use crate::perfil::TECLADO;

    #[test]
    fn gamepad_recusa_o_perfil_de_teclado() {
        match Gamepad::novo(&TECLADO) {
            Err(e) => assert_eq!(e.kind(), io::ErrorKind::InvalidInput),
            Ok(_) => panic!("um perfil de teclado não pode virar gamepad"),
        }
    }
}
