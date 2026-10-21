//! Identidade do controle virtual: o que o jogo enxerga (VID:PID, nome, faixas dos eixos
//! e tipo de gatilho). Os valores seguem os drivers do kernel (xpad, hid-playstation,
//! hid-nintendo); os botões de face são por posição e iguais em todos os perfis.

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub struct Faixa {
    pub min: i32,
    pub max: i32,
}

impl Faixa {
    pub const fn centro(&self) -> i32 {
        (self.min + self.max + 1) / 2
    }

    /// Converte um eixo do protocolo (i16 centrado em zero) para esta faixa.
    pub fn converter(&self, valor: i16) -> i32 {
        let deslocado = valor as i64 + 32768; // 0..=65535
        let largura = (self.max - self.min) as i64;
        ((deslocado * largura + 32767) / 65535) as i32 + self.min
    }
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Gatilhos {
    /// Só eixo (0..=255), como o Xbox.
    Analogicos,
    /// Só botão, como o ZL/ZR do Switch Pro.
    Digitais,
    /// Eixo e botão ao mesmo tempo, como o L2/R2 do DualShock 4.
    Ambos,
}

#[derive(Debug, PartialEq, Eq, Clone, Copy)]
pub enum Tipo {
    Gamepad,
    Teclado,
}

#[derive(Debug, PartialEq, Eq)]
pub struct Perfil {
    pub id: &'static str,
    pub nome: &'static str,
    pub fabricante: u16,
    pub produto: u16,
    pub versao: u16,
    pub tipo: Tipo,
    /// Só os gamepads usam `analogico` e `gatilhos`.
    pub analogico: Faixa,
    pub gatilhos: Gatilhos,
}

pub const XBOX_360: Perfil = Perfil {
    id: "xbox360",
    tipo: Tipo::Gamepad,
    nome: "vpad Xbox 360 Controller",
    fabricante: 0x045e,
    produto: 0x028e,
    versao: 0x0110,
    analogico: Faixa { min: -32768, max: 32767 },
    gatilhos: Gatilhos::Analogicos,
};

pub const DUALSHOCK_4: Perfil = Perfil {
    id: "ds4",
    tipo: Tipo::Gamepad,
    nome: "vpad DualShock 4",
    fabricante: 0x054c,
    produto: 0x09cc,
    versao: 0x0100,
    analogico: Faixa { min: 0, max: 255 },
    gatilhos: Gatilhos::Ambos,
};

pub const SWITCH_PRO: Perfil = Perfil {
    id: "switch-pro",
    tipo: Tipo::Gamepad,
    nome: "vpad Switch Pro Controller",
    fabricante: 0x057e,
    produto: 0x2009,
    versao: 0x0100,
    analogico: Faixa { min: -32767, max: 32767 },
    gatilhos: Gatilhos::Digitais,
};

/// Teclado virtual: os controles da página viram teclas (ver `mapa`).
pub const TECLADO: Perfil = Perfil {
    id: "teclado",
    tipo: Tipo::Teclado,
    nome: "vpad Keyboard",
    fabricante: 0x1209,
    produto: 0x0001,
    versao: 0x0100,
    analogico: Faixa { min: -32768, max: 32767 },
    gatilhos: Gatilhos::Analogicos,
};

pub const PERFIS: [&Perfil; 4] = [&XBOX_360, &DUALSHOCK_4, &SWITCH_PRO, &TECLADO];

impl Perfil {
    pub fn por_id(id: &str) -> Option<&'static Perfil> {
        PERFIS.into_iter().find(|p| p.id == id)
    }
}

/// Quem revogar.
#[derive(Debug, PartialEq, Eq)]
pub enum Alvo {
    Todos,
    Id(String),
}

/// O que o programa deve fazer.
#[derive(Debug, PartialEq, Eq)]
pub enum Acao {
    /// subir o servidor (o padrão)
    Servir,
    /// mostrar os aparelhos pareados e sair
    ListarDispositivos,
    /// revogar o acesso de aparelhos pareados e sair
    Revogar(Alvo),
}

/// O que a linha de comando pede.
#[derive(Debug, PartialEq, Eq)]
pub struct Opcoes {
    pub perfil: &'static Perfil,
    pub acao: Acao,
}

/// Lê `--perfil <id>`, `--dispositivos` e `--revogar <id|todos>` (sem o nome do programa).
/// O perfil padrão é o Xbox 360.
pub fn opcoes_dos_argumentos(mut args: impl Iterator<Item = String>) -> Result<Opcoes, String> {
    let mut id = XBOX_360.id.to_string();
    let mut acao = Acao::Servir;
    let definir_acao = |nova: Acao, atual: &mut Acao| -> Result<(), String> {
        if *atual != Acao::Servir {
            return Err(format!("use só uma ação por vez\n{}", uso()));
        }
        *atual = nova;
        Ok(())
    };
    let alvo = |valor: &str| -> Result<Acao, String> {
        match valor {
            "" => Err("--revogar precisa de um id (ou 'todos')".to_string()),
            "todos" => Ok(Acao::Revogar(Alvo::Todos)),
            id => Ok(Acao::Revogar(Alvo::Id(id.to_string()))),
        }
    };
    while let Some(arg) = args.next() {
        if arg == "--perfil" {
            id = args.next().ok_or("--perfil precisa de um valor")?;
        } else if let Some(valor) = arg.strip_prefix("--perfil=") {
            id = valor.to_string();
        } else if arg == "--dispositivos" {
            definir_acao(Acao::ListarDispositivos, &mut acao)?;
        } else if arg == "--revogar" {
            let valor = args.next().ok_or("--revogar precisa de um id (ou 'todos')")?;
            definir_acao(alvo(&valor)?, &mut acao)?;
        } else if let Some(valor) = arg.strip_prefix("--revogar=") {
            definir_acao(alvo(valor)?, &mut acao)?;
        } else {
            return Err(format!("argumento desconhecido: {arg}\n{}", uso()));
        }
    }
    let perfil = Perfil::por_id(&id).ok_or_else(|| format!("perfil desconhecido: {id}\n{}", uso()))?;
    Ok(Opcoes { perfil, acao })
}

/// Posição do perfil em `PERFIS`, que é o número trocado com o celular.
pub fn indice_do_perfil(id: &str) -> Option<u8> {
    PERFIS.iter().position(|p| p.id == id).map(|i| i as u8)
}

fn uso() -> String {
    let ids: Vec<_> = PERFIS.iter().map(|p| p.id).collect();
    format!(
        "uso: vpad [--perfil <{}>]\n     vpad --dispositivos\n     vpad --revogar <id|todos>",
        ids.join("|")
    )
}

#[cfg(test)]
mod testes {
    use super::*;

    #[test]
    fn identidades_conferem_com_os_drivers_do_kernel() {
        let id = |p: &Perfil| (p.fabricante, p.produto);
        assert_eq!(id(&XBOX_360), (0x045e, 0x028e));
        assert_eq!(id(&DUALSHOCK_4), (0x054c, 0x09cc));
        assert_eq!(id(&SWITCH_PRO), (0x057e, 0x2009));
        assert_eq!(id(&TECLADO), (0x1209, 0x0001));
    }

    #[test]
    fn tipo_do_perfil() {
        assert_eq!(XBOX_360.tipo, Tipo::Gamepad);
        assert_eq!(DUALSHOCK_4.tipo, Tipo::Gamepad);
        assert_eq!(SWITCH_PRO.tipo, Tipo::Gamepad);
        assert_eq!(TECLADO.tipo, Tipo::Teclado);
    }

    #[test]
    fn ids_sao_unicos_e_encontrados() {
        for p in PERFIS {
            assert_eq!(Perfil::por_id(p.id).map(|x| x.id), Some(p.id));
        }
        assert_eq!(PERFIS.len(), 4);
        assert!(Perfil::por_id("nao-existe").is_none());
    }

    #[test]
    fn faixas_dos_analogicos() {
        assert_eq!(XBOX_360.analogico, Faixa { min: -32768, max: 32767 });
        assert_eq!(DUALSHOCK_4.analogico, Faixa { min: 0, max: 255 });
        assert_eq!(SWITCH_PRO.analogico, Faixa { min: -32767, max: 32767 });
    }

    #[test]
    fn centro_de_cada_faixa() {
        assert_eq!(XBOX_360.analogico.centro(), 0);
        assert_eq!(DUALSHOCK_4.analogico.centro(), 128);
        assert_eq!(SWITCH_PRO.analogico.centro(), 0);
    }

    #[test]
    fn tipo_de_gatilho_por_perfil() {
        assert_eq!(XBOX_360.gatilhos, Gatilhos::Analogicos);
        assert_eq!(DUALSHOCK_4.gatilhos, Gatilhos::Ambos);
        assert_eq!(SWITCH_PRO.gatilhos, Gatilhos::Digitais);
    }

    fn args(a: &[&str]) -> impl Iterator<Item = String> {
        a.iter().map(|s| s.to_string()).collect::<Vec<_>>().into_iter()
    }

    #[test]
    fn sem_argumento_usa_xbox_360() {
        assert_eq!(opcoes_dos_argumentos(args(&[])).unwrap().perfil.id, "xbox360");
    }

    #[test]
    fn aceita_perfil_separado_ou_com_igual() {
        assert_eq!(opcoes_dos_argumentos(args(&["--perfil", "ds4"])).unwrap().perfil.id, "ds4");
        assert_eq!(
            opcoes_dos_argumentos(args(&["--perfil=switch-pro"])).unwrap().perfil.id,
            "switch-pro"
        );
    }

    #[test]
    fn perfil_desconhecido_lista_as_opcoes() {
        let erro = opcoes_dos_argumentos(args(&["--perfil", "wii"])).unwrap_err();
        assert!(erro.contains("wii"));
        for p in PERFIS {
            assert!(erro.contains(p.id), "{erro}");
        }
    }

    #[test]
    fn perfil_sem_valor_e_erro() {
        assert!(opcoes_dos_argumentos(args(&["--perfil"])).is_err());
    }

    #[test]
    fn argumento_desconhecido_e_erro() {
        assert!(opcoes_dos_argumentos(args(&["--foo"])).is_err());
    }

    #[test]
    fn ordem_dos_perfis_e_a_mesma_da_pagina() {
        // web/src/profiles.ts usa estes mesmos índices: mudar a ordem exige mudar os dois lados.
        let ids: Vec<_> = PERFIS.iter().map(|p| p.id).collect();
        assert_eq!(ids, ["xbox360", "ds4", "switch-pro", "teclado"]);
    }

    #[test]
    fn indice_do_perfil_pelo_id() {
        assert_eq!(indice_do_perfil("xbox360"), Some(0));
        assert_eq!(indice_do_perfil("ds4"), Some(1));
        assert_eq!(indice_do_perfil("switch-pro"), Some(2));
        assert_eq!(indice_do_perfil("teclado"), Some(3));
        assert_eq!(indice_do_perfil("wii"), None);
    }

    #[test]
    fn por_padrao_sobe_o_servidor() {
        assert_eq!(opcoes_dos_argumentos(args(&[])).unwrap().acao, Acao::Servir);
        assert_eq!(opcoes_dos_argumentos(args(&["--perfil", "ds4"])).unwrap().acao, Acao::Servir);
    }

    #[test]
    fn dispositivos_pede_a_lista() {
        assert_eq!(opcoes_dos_argumentos(args(&["--dispositivos"])).unwrap().acao, Acao::ListarDispositivos);
    }

    #[test]
    fn revogar_aceita_um_id_ou_todos_nas_duas_formas() {
        let alvo = |a: &[&str]| opcoes_dos_argumentos(args(a)).unwrap().acao;
        assert_eq!(alvo(&["--revogar", "ab12cd34"]), Acao::Revogar(Alvo::Id("ab12cd34".into())));
        assert_eq!(alvo(&["--revogar=ab12cd34"]), Acao::Revogar(Alvo::Id("ab12cd34".into())));
        assert_eq!(alvo(&["--revogar", "todos"]), Acao::Revogar(Alvo::Todos));
        assert_eq!(alvo(&["--revogar=todos"]), Acao::Revogar(Alvo::Todos));
    }

    #[test]
    fn revogar_sem_valor_e_erro() {
        assert!(opcoes_dos_argumentos(args(&["--revogar"])).is_err());
        assert!(opcoes_dos_argumentos(args(&["--revogar="])).is_err());
    }

    #[test]
    fn so_uma_acao_por_vez() {
        let erro = opcoes_dos_argumentos(args(&["--dispositivos", "--revogar", "todos"])).unwrap_err();
        assert!(erro.contains("uma ação"), "{erro}");
    }

    #[test]
    fn o_token_unico_antigo_nao_existe_mais() {
        assert!(opcoes_dos_argumentos(args(&["--novo-token"])).is_err());
    }

    #[test]
    fn a_mensagem_de_uso_cita_as_novas_opcoes() {
        let erro = opcoes_dos_argumentos(args(&["--foo"])).unwrap_err();
        assert!(erro.contains("--dispositivos") && erro.contains("--revogar"), "{erro}");
    }
}
