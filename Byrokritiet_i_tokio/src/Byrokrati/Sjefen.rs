//! sjefen handterer opretting av master / backup
//! 

use std::env;

#[derive(Clone, Debug)]
pub struct AnsattPakke {
    pub addr: String,
    pub num_floors: u8,
    pub name: String,
}

/// Master er for den serveren med ansvar, og slaven kopierer masteren. Kvar master og slave lager sin eigen backup.
#[derive(Clone, PartialEq, Debug)]
pub enum Rolle {
    MASTER,
    SLAVE,
    BACKUP,
}

#[derive(Clone, Debug)]
pub struct SjefPakke {
    pub id: u8,
    pub rolle: Rolle,
    // TODO: Lage IP
}

/// Hentar og analyserer argument frå kommandolinja for å returnere ein `SjefPakke`
pub fn hent_sjefpakke() -> Result<SjefPakke, &'static str> {
    let args: Vec<String> = env::args().collect();

    if args.len() > 2 {
        let command = args[1].to_lowercase();
        let id: u8 = match args[2].parse() {
            Ok(num) => num,
            Err(_) => return Err("Andre argument må være et positivt heltall. (u8)"),
        };

        let rolle = match command.as_str() {
            "backup" => Rolle::BACKUP,
            "master" => Rolle::MASTER,
            _ => return Err("Ugyldig kommando. Bruk 'backup' eller 'master'."),
        };

        Ok(SjefPakke { id, rolle })
    } else {
        Err("Bruk: <program> <kommando> <ID>")
    }
}




