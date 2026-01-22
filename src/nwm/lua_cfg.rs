use std::sync::{Arc, Mutex};

use mlua::Lua;

pub fn load_config(path: &std::path::Path) -> Result<Config, ()> {
    let lua = Lua::new();

    let config = Arc::new(Mutex::new(Config::default()));

    inject_config_api(&lua, config.clone()).unwrap();
    inject_action_data(&lua).unwrap();
    inject_bind_api(&lua, config.clone()).unwrap();
    inject_key_consts(&lua).unwrap();
    inject_mod_consts(&lua).unwrap();



    let code = std::fs::read_to_string(path).unwrap();

    lua.load(&code).set_name("config.lua").exec().unwrap();
    let mut config = config.lock().unwrap().clone();

    {
        let m_key = config.settings.master_key;
        for b in &mut config.binds {
            b.combo.prefixes.insert(0, m_key);
        }
    }

    Ok(config)
}

fn inject_config_api(lua: &Lua, config: Arc<Mutex<Config>>) -> mlua::Result<()> {
    let globals = lua.globals();

    let set_table = lua.create_table()?;

    macro_rules! set_usize {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set($name, lua.create_function(move |_, n: usize| {
                cfg.lock().unwrap().settings.$field = n;
                Ok(())
            })?)?;
        }};
    }

    macro_rules! set_string {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set($name, lua.create_function(move |_, s: String| {
                cfg.lock().unwrap().settings.$field = s;
                Ok(())
            })?)?;
        }};
    }

    macro_rules! set_color {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set($name, lua.create_function(move |_, n: String| {
                let color = u32::from_str_radix(&n[1..], 16).unwrap();
                cfg.lock().unwrap().settings.$field = color;
                Ok(())
            })?)?;
        }};
    }

    set_usize!("gap", gap);
    set_usize!("border_width", border_width);

    set_string!("terminal", terminal);
    set_string!("launcher", launcher);

    set_color!("border_active_color", border_active_color);
    set_color!("border_inactive_color", border_inactive_color);

    {
        let cfg = config.clone();
        set_table.set("master_key", lua.create_function(move |_, k: SpecialKey| {
            cfg.lock().unwrap().settings.master_key = k;
            Ok(())
        })?)?;
    }

    globals.set("set", set_table)?;

    Ok(())
}

fn inject_action_data(lua: &Lua) -> mlua::Result<()> {
    let action_table = lua.create_table()?;

    let focus_table = lua.create_table()?;
    focus_table.set("left", Action::FocusLeft)?;
    focus_table.set("right", Action::FocusRight)?;

    let move_table = lua.create_table()?;
    move_table.set("left", Action::MoveLeft)?;
    move_table.set("right", Action::MoveRight)?;

    action_table.set("focus", focus_table)?;
    action_table.set("move", move_table)?;
    action_table.set("terminal", Action::Terminal)?;
    action_table.set("launcher", Action::Launcher)?;

    action_table.set("close", Action::CloseWindow)?;

    action_table.set("prev_ws", Action::PrevWs)?;
    action_table.set("next_ws", Action::NextWs)?;
    action_table.set("reload", Action::ReloadConfig)?;

    lua.globals().set("action", action_table).unwrap();

    Ok(())
}

fn inject_bind_api<'a>(lua: &'a Lua, config: Arc<Mutex<Config>>) -> mlua::Result<()> {
    let globals = lua.globals();

    let bind = lua.create_function(move |_, (combo, action): (String, Action)| {
        let combo = parse_keycombo(&combo).map_err(|_| {
            mlua::Error::RuntimeError("invalid key combo".into())
        })?;

        config.lock().unwrap().binds.push(Binding {
            combo,
            action,
        });

        Ok(())
    })?;

    globals.set("bind", bind)?;
    Ok(())
}

fn inject_mod_consts(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    g.set("Alt", SpecialKey::Alt)?;
    g.set("Super", SpecialKey::Super)?;
    g.set("Shift", SpecialKey::Shift)?;
    g.set("Control", SpecialKey::Control)?;
    Ok(())
}

fn inject_key_consts(lua: &Lua) -> mlua::Result<()> {
    let g = lua.globals();
    g.set("Space", "Space")?;
    g.set("Return", "Return")?;
    g.set("Tab", "Tab")?;
    g.set("Escape", "Escape")?;
    Ok(())
}


fn parse_keycombo(s: &str) -> Result<KeyCombo, ()> {
    let mut combo = KeyCombo::default();

    let parts = s.split('-').collect::<Vec<_>>();
    let parts = &parts;
    let final_key = parts.last().unwrap();
    combo.key = match *final_key {
        "Space" => Key::Space,
        "Return" => Key::Return,
        "Tab" => Key::Tab,
        "Escape" => Key::Escape,
        k if k.len() == 1 => Key::Char(k.chars().next().unwrap()),
        k if k.parse::<u32>().is_ok() => Key::Char(k.chars().next().unwrap()),
        _ => unreachable!()
    };
    let parts = &parts[..parts.len() - 1];
    for p in parts {
        match *p {
            "Alt" => {
                combo.prefixes.push(SpecialKey::Alt);
            },
            "Super" => {
                combo.prefixes.push(SpecialKey::Super);
            },
            "Shift" => {
                combo.prefixes.push(SpecialKey::Shift);
            },
            "Control" => {
                combo.prefixes.push(SpecialKey::Control);
            },
            _ => unreachable!()
        }
    }
    Ok(combo)
}


#[derive(Debug, Clone, Default)]
pub struct Config {
    pub settings: Settings,
    pub binds: Vec<Binding>
}

#[derive(Debug, Clone, Default)]
pub struct Settings {
    pub master_key: SpecialKey,
    pub gap: usize,
    pub terminal: String,
    pub launcher: String,
    pub border_width: usize,
    pub border_active_color: u32,
    pub border_inactive_color: u32,
}

#[derive(Debug, Clone, Default)]
pub struct Binding {
    pub combo: KeyCombo,
    pub action: Action
}

#[derive(Debug, Clone, Default)]
pub struct KeyCombo {
    pub prefixes: Vec<SpecialKey>,
    pub key: Key,
}

#[derive(Debug, Clone, Copy, Default)]
pub enum Key {
    Char(char),
    Space,
    Return,
    Tab,
    #[default]
    Escape,
}

impl Key {
    pub fn into_x11rb(self) -> u32 {
        match self {
            Self::Escape => crate::better_x11rb::XK_ESCAPE,
            Self::Space => ' ' as u32,
            Self::Return => crate::better_x11rb::XK_RETURN,
            Self::Tab => crate::better_x11rb::XK_TAB,
            Self::Char(c) => c as u32,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Action {
    #[default]
    FocusLeft,
    FocusRight,
    MoveLeft,
    MoveRight,
    Launcher,
    Terminal,
    CloseWindow,
    NextWs,
    PrevWs,
    ReloadConfig,
}

impl mlua::UserData for Action {}
impl mlua::FromLua for Action {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow().unwrap()),
            _ => unreachable!()
        }
    }
}


#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum SpecialKey {
    #[default]
    Shift,
    Control,
    Alt,
    Super,
    Space,
}

impl mlua::UserData for SpecialKey {}
impl mlua::FromLua for SpecialKey {
    fn from_lua(value: mlua::Value, lua: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow().unwrap()),
            _ => todo!()
        }
    }
}

impl ToString for SpecialKey {
    fn to_string(&self) -> String {
        match self {
            SpecialKey::Alt => "Alt".to_string(),
            SpecialKey::Shift => "Shift".to_string(),
            SpecialKey::Control => "Control".to_string(),
            SpecialKey::Super => "Super".to_string(),
            SpecialKey::Space => "Space".to_string(),
        }
    }
}

impl From<crate::MasterKey> for SpecialKey {
    fn from(value: crate::MasterKey) -> Self {
        match value {
            crate::MasterKey::Alt => SpecialKey::Alt,
            crate::MasterKey::Super => SpecialKey::Super,
            crate::MasterKey::Shift => SpecialKey::Shift,
            crate::MasterKey::Control => SpecialKey::Control,
        }
    }
}


