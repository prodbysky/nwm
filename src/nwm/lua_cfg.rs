#[derive(Debug, Clone)]
pub struct Config {
    pub settings: Settings,
    pub binds: Vec<Binding>
}

#[derive(Debug, Clone)]
pub struct Settings {
    pub master_key: SpecialKey,
    pub gap: usize,
    pub terminal: String,
    pub launcher: String,
    pub border_width: usize,
    pub border_active_color: String,
    pub border_inactive_color: String,
}

#[derive(Debug, Clone)]
pub struct Binding {
    pub combo: KeyCombo,
    pub action: Action
}

#[derive(Debug, Clone)]
pub struct KeyCombo {
    pub prefixes: Vec<SpecialKey>,
    pub key: Key,
}

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Char(char),
    Space,
    Return,
    Tab,
    Escape,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Action {
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

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpecialKey {
    Shift,
    Control,
    Alt,
    Super,
    Space,
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


