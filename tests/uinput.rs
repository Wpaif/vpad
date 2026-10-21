//! Testes com o uinput de verdade. Sem acesso a /dev/uinput (CI, container), eles se pulam.

use std::time::{Duration, Instant};

use evdev::{AbsoluteAxisCode as Abs, Device, EventType, InputEvent, KeyCode};
use vpad::{dispositivo::Gamepad, protocolo::Evento};

fn gamepad() -> Option<Gamepad> {
    match Gamepad::novo() {
        Ok(g) => Some(g),
        Err(e) => {
            eprintln!("uinput indisponível ({e}); teste ignorado");
            None
        }
    }
}

/// Abre o dispositivo virtual como um jogo faria: pelo /dev/input/eventN.
fn abrir(g: &mut Gamepad) -> Device {
    let fim = Instant::now() + Duration::from_secs(3);
    loop {
        // O udev leva um instante para criar o nó e liberar a permissão.
        let aberto = g
            .caminhos()
            .unwrap()
            .into_iter()
            .find_map(|p| Device::open(p).ok());
        if let Some(d) = aberto {
            d.set_nonblocking(true).unwrap();
            return d;
        }
        assert!(Instant::now() < fim, "dispositivo não apareceu em /dev/input");
        std::thread::sleep(Duration::from_millis(50));
    }
}

fn ler(dev: &mut Device, quantos: usize) -> Vec<InputEvent> {
    let fim = Instant::now() + Duration::from_secs(2);
    let mut vistos = vec![];
    while vistos.len() < quantos && Instant::now() < fim {
        match dev.fetch_events() {
            Ok(it) => vistos.extend(it.filter(|e| e.event_type() != EventType::SYNCHRONIZATION)),
            Err(_) => std::thread::sleep(Duration::from_millis(10)),
        }
    }
    vistos
}

#[test]
fn aparece_como_um_controle_xbox_360() {
    let Some(mut g) = gamepad() else { return };
    let dev = abrir(&mut g);

    assert_eq!(dev.name(), Some("vpad virtual gamepad"));
    let id = dev.input_id();
    assert_eq!((id.vendor(), id.product()), (0x045e, 0x028e));

    let teclas = dev.supported_keys().unwrap();
    for k in [KeyCode::BTN_SOUTH, KeyCode::BTN_START, KeyCode::BTN_THUMBR] {
        assert!(teclas.contains(k), "{k:?}");
    }
    let eixos = dev.supported_absolute_axes().unwrap();
    for a in [Abs::ABS_X, Abs::ABS_RY, Abs::ABS_Z, Abs::ABS_RZ, Abs::ABS_HAT0X, Abs::ABS_HAT0Y] {
        assert!(eixos.contains(a), "{a:?}");
    }
}

#[test]
fn faixas_dos_eixos_seguem_o_padrao_xbox() {
    let Some(mut g) = gamepad() else { return };
    let dev = abrir(&mut g);
    let faixa = |a: Abs| {
        let info = dev.get_abs_state().unwrap()[a.0 as usize];
        (info.minimum, info.maximum)
    };
    assert_eq!(faixa(Abs::ABS_X), (-32768, 32767));
    assert_eq!(faixa(Abs::ABS_Z), (0, 255));
    assert_eq!(faixa(Abs::ABS_HAT0X), (-1, 1));
}

#[test]
fn botao_pressionado_chega_ao_dispositivo() {
    let Some(mut g) = gamepad() else { return };
    let mut dev = abrir(&mut g);

    g.aplicar(Evento::Botao { id: 0, pressionado: true }).unwrap();
    g.aplicar(Evento::Botao { id: 0, pressionado: false }).unwrap();

    let eventos = ler(&mut dev, 2);
    let teclas: Vec<_> = eventos
        .iter()
        .map(|e| (e.event_type(), e.code(), e.value()))
        .collect();
    assert_eq!(
        teclas,
        vec![
            (EventType::KEY, KeyCode::BTN_SOUTH.0, 1),
            (EventType::KEY, KeyCode::BTN_SOUTH.0, 0),
        ]
    );
}

#[test]
fn eixo_e_direcional_chegam_ao_dispositivo() {
    let Some(mut g) = gamepad() else { return };
    let mut dev = abrir(&mut g);

    g.aplicar(Evento::Eixo { id: 0, valor: 20000 }).unwrap();
    g.aplicar(Evento::Botao { id: 12, pressionado: true }).unwrap();

    let eventos = ler(&mut dev, 2);
    let lidos: Vec<_> = eventos.iter().map(|e| (e.code(), e.value())).collect();
    assert_eq!(lidos, vec![(Abs::ABS_X.0, 20000), (Abs::ABS_HAT0X.0, -1)]);
}

#[test]
fn soltar_tudo_libera_o_que_estava_pressionado() {
    let Some(mut g) = gamepad() else { return };
    let mut dev = abrir(&mut g);

    g.aplicar(Evento::Botao { id: 1, pressionado: true }).unwrap();
    ler(&mut dev, 1);
    g.aplicar(Evento::SoltarTudo).unwrap();

    let eventos = ler(&mut dev, 1);
    assert!(eventos
        .iter()
        .any(|e| e.code() == KeyCode::BTN_EAST.0 && e.value() == 0));
}
