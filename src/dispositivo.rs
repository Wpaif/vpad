//! Gamepad virtual no Linux (uinput), com a identidade de um controle Xbox 360
//! para que Steam, emuladores e jogos o reconheçam sem configuração.

use std::{io, path::PathBuf};

use evdev::{
    AbsInfo, AbsoluteAxisCode as Abs, AttributeSet, BusType, InputEvent, InputId, UinputAbsSetup,
    uinput::VirtualDevice,
};

use crate::{
    mapeamento::{BOTOES, EIXOS, Tradutor},
    protocolo::Evento,
};

const FABRICANTE_MICROSOFT: u16 = 0x045e;
const PRODUTO_XBOX_360: u16 = 0x028e;
const VERSAO: u16 = 0x0110;

pub struct Gamepad {
    dispositivo: VirtualDevice,
    tradutor: Tradutor,
}

impl Gamepad {
    pub fn novo() -> io::Result<Self> {
        let mut teclas = AttributeSet::new();
        BOTOES.iter().for_each(|k| teclas.insert(*k));

        let analogico = AbsInfo::new(0, -32768, 32767, 16, 128, 0);
        let gatilho = AbsInfo::new(0, 0, 255, 0, 0, 0);
        let hat = AbsInfo::new(0, -1, 1, 0, 0, 0);

        let mut builder = VirtualDevice::builder()?
            .name("vpad virtual gamepad")
            .input_id(InputId::new(
                BusType::BUS_USB,
                FABRICANTE_MICROSOFT,
                PRODUTO_XBOX_360,
                VERSAO,
            ))
            .with_keys(&teclas)?;

        for (i, eixo) in EIXOS.iter().enumerate() {
            let info = if i >= 4 { gatilho } else { analogico };
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(*eixo, info))?;
        }
        for eixo in [Abs::ABS_HAT0X, Abs::ABS_HAT0Y] {
            builder = builder.with_absolute_axis(&UinputAbsSetup::new(eixo, hat))?;
        }

        Ok(Self { dispositivo: builder.build()?, tradutor: Tradutor::default() })
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
