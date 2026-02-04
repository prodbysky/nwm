use log::error;
use std::{
    collections::HashMap, env::home_dir, hash::Hash, sync::{Arc, Mutex}
};

use mlua::Lua;
use mlua::FromLua;

/// Sets up the tables for the users configuration
/// `nwm.first_boot` will be equal to `!reload`
pub fn load_config(reload: bool) -> Result<Config, ()> {
    let lua = Lua::new();

    let nwm_table = lua.create_table().map_err(|e| {
        error!("Failed to create base configuration table: {e}");
    })?;

    let config = Arc::new(Mutex::new(Config::default()));

    nwm_table
        .set(
            "set",
            create_set_api(&lua, config.clone()).map_err(|e| {
                error!("Failed to create `set` api table: {e}");
            })?,
        )
        .map_err(|e| {
            error!("Failed to put `set` api table in the `nwm` table: {e}");
        })?;

    nwm_table
        .set(
            "action",
            create_action_data(&lua).map_err(|e| {
                error!("Failed to create `action` data table: {e}");
            })?,
        )
        .map_err(|e| {
            error!("Failed to put `action` data table in the `nwm` table: {e}");
        })?;

    nwm_table
        .set(
            "bind",
            create_bind_api(&lua, config.clone()).map_err(|e| {
                error!("Failed to create `bind` function: {e}");
            })?,
        )
        .map_err(|e| {
            error!("Failed to put `bind` function in the `nwm` table: {e}");
        })?;

    nwm_table
        .set(
            "key",
            create_key_consts(&lua).map_err(|e| {
                error!("Failed to create `key` table: {e}");
            })?,
        )
        .map_err(|e| {
            error!("Failed to put `key` table in the `nwm` table: {e}");
        })?;

    nwm_table
        .set(
            "modifier",
            create_mod_consts(&lua).map_err(|e| {
                error!("Failed to create `modifiers` table: {e}");
            })?,
        )
        .map_err(|e| {
            error!("Failed to put `modifiers` table in the `nwm` table: {e}");
        })?;




    // Add runtime info table
    nwm_table
        .set(
            "info",
            create_info_table(&lua).map_err(|e| {
                error!("Failed to create `info` table: {e}");
            })?,
        )
        .map_err(|e| {
            error!("Failed to put `info` table in the `nwm` table: {e}");
        })?;

    nwm_table.set("first_boot", !reload).map_err(|e| {
        error!("Failed to set first_boot global var: {e}");
    })?;

    nwm_table.set("on", create_hook_register_fn(&lua, config.clone()).map_err(|e| {
        error!("Failed to create `nwm.on` hook registering function: {e}");
    })?).unwrap();

    nwm_table.set("hook", create_hook_constant_table(&lua).map_err(|e| {
        error!("Failed to create `nwm.hook` hook constants table: {e}");
    })?).unwrap();

    lua.globals().set("nwm", nwm_table).map_err(|e| {
        error!("Failed to put table `nwm` in the globals table: {e}");
    })?;

    let mut home_dir = home_dir().unwrap();
    home_dir.push(".config/nwm/config.lua");

    let code = std::fs::read_to_string(&home_dir).map_err(|e| {
        error!("Failed to read lua config file: {e}");
    })?;

    lua.load(&code).set_name("config.lua").exec().map_err(|e| {
        error!("Failed to execute lua config file: {e}");
    })?;
    let mut config = config.lock().unwrap().clone();

    {
        let m_key = config.settings.master_key;
        for b in &mut config.binds {
            b.combo.prefixes.insert(0, m_key);
        }
    }

    // store lua for callbacks
    config.lua = Some(lua);

    Ok(config)
}

fn create_info_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let info_table = lua.create_table()?;
    
    info_table.set("version", env!("CARGO_PKG_VERSION"))?;
    info_table.set("name", env!("CARGO_PKG_NAME"))?;
    
    if let Ok(hostname) = std::env::var("HOSTNAME") {
        info_table.set("hostname", hostname)?;
    }
    if let Ok(user) = std::env::var("USER") {
        info_table.set("user", user)?;
    }
    if let Ok(display) = std::env::var("DISPLAY") {
        info_table.set("display", display)?;
    }
    
    info_table.set("workspace_count", 10)?;
    
    Ok(info_table)
}

fn create_hook_constant_table(lua: &Lua) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("add_window", HookEvent::AddWindow)?;
    Ok(table)

}

fn create_hook_register_fn(lua: &Lua, config: Arc<Mutex<Config>>) -> mlua::Result<mlua::Function> {
    let cfg = config.clone();
    lua.create_function(move |_ctx, (event, callback): (HookEvent, mlua::Value)| {
        if let mlua::Value::Function(f) = callback {
            cfg.lock().unwrap().hooks.add_hook(event, Hook {
                func: f
            });
        }
        Ok(())
    })
}

fn create_set_api(lua: &Lua, config: Arc<Mutex<Config>>) -> mlua::Result<mlua::Table> {
    let set_table = lua.create_table()?;

    macro_rules! set_usize {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set(
                $name,
                lua.create_function(move |_, n: usize| {
                    if stringify!($field) == "gap" {
                        if n == 0 {
                            return Ok(());
                        }
                    }
                    cfg.lock().unwrap().settings.$field = n;
                    Ok(())
                })?,
            )?;
        }};
    }

    macro_rules! set_float {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set(
                $name,
                lua.create_function(move |_, n: f32| {
                    cfg.lock().unwrap().settings.$field = n;
                    Ok(())
                })?,
            )?;
        }};
    }

    macro_rules! set_string {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set(
                $name,
                lua.create_function(move |_, s: String| {
                    cfg.lock().unwrap().settings.$field = s;
                    Ok(())
                })?,
            )?;
        }};
    }

    macro_rules! set_color {
        ($name:literal, $field:ident) => {{
            let cfg = config.clone();
            set_table.set(
                $name,
                lua.create_function(move |_, n: String| {
                    let color = u32::from_str_radix(&n[1..], 16).unwrap();
                    cfg.lock().unwrap().settings.$field = color;
                    Ok(())
                })?,
            )?;
        }};
    }

    set_usize!("gap", gap);
    set_usize!("border_width", border_width);

    set_string!("terminal", terminal);
    set_string!("launcher", launcher);

    set_color!("border_active_color", border_active_color);
    set_color!("border_inactive_color", border_inactive_color);

    set_float!("master_ratio", master_ratio);

    {
        let cfg = config.clone();
        set_table.set(
            "master_key",
            lua.create_function(move |_, k: SpecialKey| {
                cfg.lock().unwrap().settings.master_key = k;
                Ok(())
            })?,
        )?;
    }

    Ok(set_table)
}

fn create_action_data(lua: &Lua) -> mlua::Result<mlua::Table> {
    let action_table = lua.create_table()?;

    let focus_table = lua.create_table()?;
    focus_table.set("left", Action::FocusLeft)?;
    focus_table.set("right", Action::FocusRight)?;

    focus_table.set("up", Action::FocusUp)?;
    focus_table.set("down", Action::FocusDown)?;

    let move_table = lua.create_table()?;
    move_table.set("left", Action::MoveLeft)?;
    move_table.set("right", Action::MoveRight)?;

    move_table.set("up", Action::MoveUp)?;
    move_table.set("down", Action::MoveDown)?;

    action_table.set("focus", focus_table)?;
    action_table.set("move", move_table)?;
    action_table.set("terminal", Action::Terminal)?;
    action_table.set("launcher", Action::Launcher)?;

    action_table.set("close", Action::CloseWindow)?;

    action_table.set("prev_ws", Action::PrevWs)?;
    action_table.set("next_ws", Action::NextWs)?;
    action_table.set("reload", Action::ReloadConfig)?;
    action_table.set("quit", Action::Quit)?;
    action_table.set("ws0", Action::Ws0)?;
    action_table.set("ws1", Action::Ws1)?;
    action_table.set("ws2", Action::Ws2)?;
    action_table.set("ws3", Action::Ws3)?;
    action_table.set("ws4", Action::Ws4)?;
    action_table.set("ws5", Action::Ws5)?;
    action_table.set("ws6", Action::Ws6)?;
    action_table.set("ws7", Action::Ws7)?;
    action_table.set("ws8", Action::Ws8)?;
    action_table.set("ws9", Action::Ws9)?;

    action_table.set("move_to_ws0", Action::MoveToWs0)?;
    action_table.set("move_to_ws1", Action::MoveToWs1)?;
    action_table.set("move_to_ws2", Action::MoveToWs2)?;
    action_table.set("move_to_ws3", Action::MoveToWs3)?;
    action_table.set("move_to_ws4", Action::MoveToWs4)?;
    action_table.set("move_to_ws5", Action::MoveToWs5)?;
    action_table.set("move_to_ws6", Action::MoveToWs6)?;
    action_table.set("move_to_ws7", Action::MoveToWs7)?;
    action_table.set("move_to_ws8", Action::MoveToWs8)?;
    action_table.set("move_to_ws9", Action::MoveToWs9)?;

    action_table.set("next_layout", Action::NextLayout)?;
    action_table.set("prev_layout", Action::PrevLayout)?;

    action_table.set("gap_up", Action::GapUp)?;
    action_table.set("gap_down", Action::GapDown)?;

    action_table.set("master_ratio_up", Action::MasterRatioUp)?;
    action_table.set("master_ratio_down", Action::MasterRatioDown)?;

    Ok(action_table)
}

fn create_bind_api(lua: &Lua, config: Arc<Mutex<Config>>) -> mlua::Result<mlua::Function> {
    let bind = lua.create_function(move |lua_ctx, (combo, action): (String, mlua::Value)| {
        let combo = parse_keycombo(&combo)
            .map_err(|_| mlua::Error::RuntimeError("invalid key combo".into()))?;

        let binding = match action {
            mlua::Value::UserData(_) => {
                let action: Action = Action::from_lua(action, lua_ctx)?;
                Binding {
                    combo,
                    action: BindAction::Native(action),
                }
            }
            mlua::Value::Function(func) => {
                let key = lua_ctx.create_registry_value(func)
                    .map_err(|e| mlua::Error::RuntimeError(format!("Failed to store callback: {}", e)))?;
                
                Binding {
                    combo,
                    action: BindAction::LuaCallback(LuaCallback { key: Arc::new(key)}),
                }
            }
            _ => {
                return Err(mlua::Error::RuntimeError(
                    "action must be either an nwm.action constant or a function".into()
                ));
            }
        };

        config.lock().unwrap().binds.push(binding);
        Ok(())
    })?;

    Ok(bind)
}

fn create_mod_consts(lua: &Lua) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("Alt", SpecialKey::Alt)?;
    table.set("Super", SpecialKey::Super)?;
    table.set("Shift", SpecialKey::Shift)?;
    table.set("Control", SpecialKey::Control)?;
    Ok(table)
}

fn create_key_consts(lua: &Lua) -> mlua::Result<mlua::Table> {
    let table = lua.create_table()?;
    table.set("Space", "Space")?;
    table.set("Return", "Return")?;
    table.set("Tab", "Tab")?;
    table.set("Escape", "Escape")?;
    Ok(table)
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
        _ => return Err(()),
    };
    let parts = &parts[..parts.len() - 1];
    for p in parts {
        match *p {
            "Alt" => {
                combo.prefixes.push(SpecialKey::Alt);
            }
            "Super" => {
                combo.prefixes.push(SpecialKey::Super);
            }
            "Shift" => {
                combo.prefixes.push(SpecialKey::Shift);
            }
            "Control" => {
                combo.prefixes.push(SpecialKey::Control);
            }
            _ => return Err(()),
        }
    }
    Ok(combo)
}

#[derive(Clone)]
pub struct LuaCallback {
    pub key: Arc<mlua::RegistryKey>,
}

#[derive(Clone)]
pub enum BindAction {
    Native(Action),
    LuaCallback(LuaCallback),
}

/// TODO: Expand these
#[derive(Hash, Eq, PartialEq, Clone, Copy, Debug)]
pub enum HookEvent {
    AddWindow,
    RemoveWindow,
}

#[derive(PartialEq, Clone, Debug)]
pub struct Hook {
    pub func: mlua::Function
}

#[derive(Clone)]
pub struct Config {
    pub settings: Settings,
    pub binds: Vec<Binding>,
    pub hooks: Hooks,
    pub lua: Option<Lua>,
}

#[derive(Clone)]
pub struct Hooks(HashMap<HookEvent, Vec<Hook>>);

impl Hooks {
    pub fn new() -> Self {
        Self (HashMap::new())
    }
    pub fn call_hooks(&self, event: HookEvent) {
        if let Some(fun) = &self.0.get(&event) {
            for hook in fun.iter() {
                hook.func.call::<()>(()).unwrap();
            }
        }
    }
    fn add_hook(&mut self, event: HookEvent, callback: Hook) {
        match self.0.get_mut(&event) {
            Some(v) => v.push(callback),
            None => {
                self.0.insert(event, vec![callback]);
            }
        }
    }
}


impl Default for Config {
    fn default() -> Self {
        Self {
            settings: Settings::default(),
            binds: vec![
                Binding {
                    action: BindAction::Native(Action::Terminal),
                    combo: KeyCombo {
                        prefixes: vec![SpecialKey::Alt],
                        key: Key::Return,
                    },
                },
                Binding {
                    action: BindAction::Native(Action::Launcher),
                    combo: KeyCombo {
                        prefixes: vec![SpecialKey::Alt],
                        key: Key::Space,
                    },
                },
                Binding {
                    action: BindAction::Native(Action::CloseWindow),
                    combo: KeyCombo {
                        prefixes: vec![SpecialKey::Alt],
                        key: Key::Char('w'),
                    },
                },
                Binding {
                    action: BindAction::Native(Action::Quit),
                    combo: KeyCombo {
                        prefixes: vec![SpecialKey::Alt, SpecialKey::Shift],
                        key: Key::Char('q'),
                    },
                },
            ],
            lua: None,
            hooks: Hooks::new()
        }
    }
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub master_key: SpecialKey,
    pub gap: usize,
    pub terminal: String,
    pub launcher: String,
    pub border_width: usize,
    pub border_active_color: u32,
    pub border_inactive_color: u32,
    pub master_ratio: f32,
}

impl Default for Settings {
    fn default() -> Self {
        Self {
            master_key: SpecialKey::Alt,
            gap: 2,
            terminal: String::from("xterm"),
            launcher: String::from("dmenu_run"),
            border_width: 2,
            border_active_color: 0xffffffff,
            border_inactive_color: 0xff181818,
            master_ratio: 0.5,
        }
    }
}

#[derive(Clone)]
pub struct Binding {
    pub combo: KeyCombo,
    pub action: BindAction,
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
    FocusUp,
    FocusDown,
    MoveLeft,
    MoveRight,
    MoveUp,
    MoveDown,
    Launcher,
    Terminal,
    CloseWindow,
    NextWs,
    PrevWs,
    ReloadConfig,
    Quit,
    Ws0,
    Ws1,
    Ws2,
    Ws3,
    Ws4,
    Ws5,
    Ws6,
    Ws7,
    Ws8,
    Ws9,
    MoveToWs0,
    MoveToWs1,
    MoveToWs2,
    MoveToWs3,
    MoveToWs4,
    MoveToWs5,
    MoveToWs6,
    MoveToWs7,
    MoveToWs8,
    MoveToWs9,
    NextLayout,
    PrevLayout,
    GapUp,
    GapDown,
    MasterRatioUp,
    MasterRatioDown,
}

impl mlua::UserData for Action {}
impl mlua::FromLua for Action {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow().unwrap()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: "Lua side action constant",
                to: "Rust size action constant".to_string(),
                message: Some("You might have specified a non-action value in config.lua, check if not then pr :)".to_string())
            })
        }
    }
}

impl mlua::UserData for HookEvent {}
impl mlua::FromLua for HookEvent {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow().unwrap()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: "Lua side hook constant",
                to: "Rust size hook constant".to_string(),
                message: Some("You might have specified a non-action value in config.lua, check if not then pr :)".to_string())
            })
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
}

impl mlua::UserData for SpecialKey {}
impl mlua::FromLua for SpecialKey {
    fn from_lua(value: mlua::Value, _lua: &Lua) -> mlua::Result<Self> {
        match value {
            mlua::Value::UserData(ud) => Ok(*ud.borrow().unwrap()),
            _ => Err(mlua::Error::FromLuaConversionError {
                from: "Lua side modifier key constant",
                to: "Rust size modifier key constant".to_string(),
                message: Some("You might have specified a non-modifier value in config.lua, check if not then pr :)".to_string())
            })
        }
    }
}

impl std::fmt::Display for SpecialKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            SpecialKey::Alt => write!(f, "Alt"),
            SpecialKey::Shift => write!(f, "Shift"),
            SpecialKey::Control => write!(f, "Control"),
            SpecialKey::Super => write!(f, "Super"),
        }
    }
}
