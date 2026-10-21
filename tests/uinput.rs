//! Testes com o uinput de verdade. Sem acesso a /dev/uinput (CI, container), eles se pulam.

use std::time::{Duration, Instant};

use evdev::{AbsoluteAxisCode as Abs, Device, EventType, InputEvent, KeyCode};
use vpad::{
    dispositivo::Gamepad,
    perfil::{DUALSHOCK_4, Perfil, SWITCH_PRO, XBOX_360},
    protocolo::Evento,
};

fn gamepad() -> Option<Gamepad> {
    gamepad_com(&XBOX_360)
}

fn gamepad_com(perfil: &'static Perfil) -> Option<Gamepad> {
    match Gamepad::novo(perfil) {
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

    assert_eq!(dev.name(), Some("vpad Xbox 360 Controller"));
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

/// (mínimo, máximo, valor atual) do eixo.
fn estado(dev: &Device, a: Abs) -> (i32, i32, i32) {
    let i = dev.get_abs_state().unwrap()[a.0 as usize];
    (i.minimum, i.maximum, i.value)
}

#[test]
fn dualshock_4_declara_identidade_faixas_e_gatilhos_como_botao() {
    let Some(mut g) = gamepad_com(&DUALSHOCK_4) else { return };
    let dev = abrir(&mut g);

    assert_eq!(dev.name(), Some("vpad DualShock 4"));
    let id = dev.input_id();
    assert_eq!((id.vendor(), id.product()), (0x054c, 0x09cc));

    assert_eq!(estado(&dev, Abs::ABS_X), (0, 255, 128));
    assert!(dev.supported_absolute_axes().unwrap().contains(Abs::ABS_Z));
    let teclas = dev.supported_keys().unwrap();
    assert!(teclas.contains(KeyCode::BTN_TL2) && teclas.contains(KeyCode::BTN_TR2));
}

#[test]
fn dualshock_4_recebe_analogico_centrado_e_gatilho_nos_dois_formatos() {
    let Some(mut g) = gamepad_com(&DUALSHOCK_4) else { return };
    let mut dev = abrir(&mut g);

    // O DS4 já repousa em 128, e o kernel descarta eventos sem mudança de valor:
    // por isso o teste vai até o fim do curso e volta ao centro.
    g.aplicar(Evento::Eixo { id: 0, valor: 32767 }).unwrap();
    g.aplicar(Evento::Eixo { id: 0, valor: 0 }).unwrap();
    g.aplicar(Evento::Eixo { id: 4, valor: 255 }).unwrap();

    let lidos: Vec<_> = ler(&mut dev, 4).iter().map(|e| (e.event_type(), e.code(), e.value())).collect();
    assert_eq!(
        lidos,
        vec![
            (EventType::ABSOLUTE, Abs::ABS_X.0, 255),
            (EventType::ABSOLUTE, Abs::ABS_X.0, 128),
            (EventType::ABSOLUTE, Abs::ABS_Z.0, 255),
            (EventType::KEY, KeyCode::BTN_TL2.0, 1),
        ]
    );
}

#[test]
fn switch_pro_nao_tem_eixo_de_gatilho_so_botao() {
    let Some(mut g) = gamepad_com(&SWITCH_PRO) else { return };
    let mut dev = abrir(&mut g);

    assert_eq!(dev.name(), Some("vpad Switch Pro Controller"));
    let id = dev.input_id();
    assert_eq!((id.vendor(), id.product()), (0x057e, 0x2009));

    let eixos = dev.supported_absolute_axes().unwrap();
    assert!(!eixos.contains(Abs::ABS_Z) && !eixos.contains(Abs::ABS_RZ));
    let (min, max, _) = estado(&dev, Abs::ABS_X);
    assert_eq!((min, max), (-32767, 32767));

    g.aplicar(Evento::Eixo { id: 5, valor: 255 }).unwrap();
    let lidos: Vec<_> = ler(&mut dev, 1).iter().map(|e| (e.code(), e.value())).collect();
    assert_eq!(lidos, vec![(KeyCode::BTN_TR2.0, 1)]);
}

mod teclado {
    //! Estes testes criam um teclado de verdade no sistema. Os que emitem eventos usam só
    //! modificadoras isoladas (Shift/Ctrl/Alt, que não digitam nada) e ficam atrás de
    //! `#[ignore]`, pois mexem na entrada da sessão em uso: `cargo test -- --ignored`.

    use super::*;
    use std::collections::BTreeMap;
    use vpad::{dispositivo::Teclado, mapa::Mapa, perfil::TECLADO};

    fn mapa_inofensivo(pares: &[(&str, &str)]) -> Mapa {
        let json: BTreeMap<_, _> = pares.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect();
        Mapa::de_json(&json).unwrap()
    }

    fn teclado(mapa: Mapa) -> Option<Teclado> {
        match Teclado::novo(&TECLADO, mapa) {
            Ok(t) => Some(t),
            Err(e) => {
                eprintln!("uinput indisponível ({e}); teste ignorado");
                None
            }
        }
    }

    fn abrir_teclado(t: &mut Teclado) -> Device {
        let fim = Instant::now() + Duration::from_secs(3);
        loop {
            let aberto = t.caminhos().unwrap().into_iter().find_map(|p| Device::open(p).ok());
            if let Some(d) = aberto {
                d.set_nonblocking(true).unwrap();
                return d;
            }
            assert!(Instant::now() < fim, "dispositivo não apareceu em /dev/input");
            std::thread::sleep(Duration::from_millis(50));
        }
    }

    #[test]
    fn aparece_como_teclado_com_todas_as_teclas_do_catalogo() {
        let Some(mut t) = teclado(Mapa::padrao()) else { return };
        let dev = abrir_teclado(&mut t);

        assert_eq!(dev.name(), Some("vpad Keyboard"));
        let id = dev.input_id();
        assert_eq!((id.vendor(), id.product()), (0x1209, 0x0001));

        let teclas = dev.supported_keys().unwrap();
        for k in [KeyCode::KEY_A, KeyCode::KEY_SPACE, KeyCode::KEY_LEFTSHIFT, KeyCode::KEY_F12, KeyCode::KEY_UP] {
            assert!(teclas.contains(k), "{k:?}");
        }
        assert!(dev.supported_absolute_axes().is_none(), "teclado não tem eixos");
    }

    #[test]
    #[ignore = "injeta modificadoras na sessão em uso; rode com --ignored"]
    fn botao_do_celular_vira_tecla_no_sistema() {
        let mapa = mapa_inofensivo(&[("A", "ShiftRight"), ("LS_Right", "ControlRight")]);
        let Some(mut t) = teclado(mapa) else { return };
        let mut dev = abrir_teclado(&mut t);

        t.aplicar(Evento::Botao { id: 0, pressionado: true }).unwrap();
        t.aplicar(Evento::Eixo { id: 0, valor: 30000 }).unwrap();
        t.aplicar(Evento::Botao { id: 0, pressionado: false }).unwrap();
        t.aplicar(Evento::SoltarTudo).unwrap();

        let lidos: Vec<_> = ler(&mut dev, 4).iter().map(|e| (e.code(), e.value())).collect();
        assert_eq!(
            lidos,
            vec![
                (KeyCode::KEY_RIGHTSHIFT.0, 1),
                (KeyCode::KEY_RIGHTCTRL.0, 1),
                (KeyCode::KEY_RIGHTSHIFT.0, 0),
                (KeyCode::KEY_RIGHTCTRL.0, 0),
            ]
        );
    }

    #[test]
    #[ignore = "injeta modificadoras na sessão em uso; rode com --ignored"]
    fn mapa_novo_solta_o_que_estava_pressionado_e_vale_no_proximo_toque() {
        let Some(mut t) = teclado(mapa_inofensivo(&[("A", "ShiftRight")])) else { return };
        let mut dev = abrir_teclado(&mut t);

        t.aplicar(Evento::Botao { id: 0, pressionado: true }).unwrap();
        ler(&mut dev, 1);

        t.definir_mapa(&mapa_inofensivo(&[("A", "ControlRight")])).unwrap();
        t.aplicar(Evento::Botao { id: 0, pressionado: true }).unwrap();
        t.aplicar(Evento::SoltarTudo).unwrap();

        let lidos: Vec<_> = ler(&mut dev, 3).iter().map(|e| (e.code(), e.value())).collect();
        assert_eq!(
            lidos,
            vec![
                (KeyCode::KEY_RIGHTSHIFT.0, 0),
                (KeyCode::KEY_RIGHTCTRL.0, 1),
                (KeyCode::KEY_RIGHTCTRL.0, 0),
            ]
        );
    }
}
