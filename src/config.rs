use log::error;
use once_cell::sync::Lazy;
use std::io::Write;

use crate::better_x11;

// TODO: Fix the fuckery in the config

pub const DEFAULT_CONFIG: Lazy<Vec<Statement>> = Lazy::new(|| {
    vec![
        Statement::Set {
            var: Variable::MasterKey,
            value: Value::Key(SpecialKey::Alt),
        },
        Statement::Set {
            var: Variable::Gap,
            value: Value::Num(8),
        },
        Statement::Set {
            var: Variable::Terminal,
            value: Value::String("alacritty".to_string()),
        },
        Statement::Set {
            var: Variable::Launcher,
            value: Value::String("dmenu_run".to_string()),
        },
        Statement::Do {
            action: Action::FocusLeft,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Char('h'),
            },
        },
        Statement::Do {
            action: Action::FocusRight,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Char('l'),
            },
        },
        Statement::Do {
            action: Action::MoveLeft,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt, SpecialKey::Shift],
                key: Key::Char('h'),
            },
        },
        Statement::Do {
            action: Action::MoveRight,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt, SpecialKey::Shift],
                key: Key::Char('l'),
            },
        },
        Statement::Do {
            action: Action::Launcher,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Space,
            },
        },
        Statement::Do {
            action: Action::Terminal,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Return,
            },
        },
        Statement::Do {
            action: Action::CloseWindow,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Char('w'),
            },
        },
        Statement::Do {
            action: Action::NextWs,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Char('2'),
            },
        },
        Statement::Do {
            action: Action::PrevWs,
            on: KeyCombo {
                prefixes: vec![SpecialKey::Alt],
                key: Key::Char('1'),
            },
        },
    ]
});

pub struct Config(pub Vec<Statement>);

impl Config {
    pub fn parse(input: String) -> Option<Self> {
        let tokens = Lexer::new(input).run();
        Some(Self(Parser::new(&tokens).parse()?))
    }

    pub fn default() -> Self {
        Self(DEFAULT_CONFIG.clone())
    }
}

impl ToString for Config {
    fn to_string(&self) -> String {
        let mut o = std::io::BufWriter::new(vec![]);
        for s in &self.0 {
            match s {
                Statement::Do { action, on } => {
                    let _ = write!(o, "Do ");
                    let _ = match action {
                        Action::FocusLeft => write!(o, "FocusLeft "),
                        Action::FocusRight => write!(o, "FocusRight "),
                        Action::MoveLeft => write!(o, "MoveLeft "),
                        Action::MoveRight => write!(o, "MoveRight "),
                        Action::Launcher => write!(o, "Launcher "),
                        Action::Terminal => write!(o, "Terminal "),
                        Action::CloseWindow => write!(o, "CloseWindow "),
                        Action::NextWs => write!(o, "NextWs "),
                        Action::PrevWs => write!(o, "PrevWs "),
                    };
                    let _ = write!(o, "on ");
                    for p in on.prefixes.iter().skip(1) {
                        let _ = write!(o, "{}-", p.to_string());
                    }
                    let key_str = match &on.key {
                        Key::Char(c) => c.to_string(),
                        Key::Space => "Space".to_string(),
                        Key::Return => "Return".to_string(),
                        Key::Tab => "Tab".to_string(),
                        Key::Escape => "Escape".to_string(),
                    };
                    let _ = writeln!(o, "{}", key_str);
                }
                Statement::Set { var, value } => {
                    let _ = write!(o, "Set ");
                    let _ = match var {
                        Variable::MasterKey => write!(o, "MasterKey "),
                        Variable::Gap => write!(o, "Gap "),
                        Variable::Terminal => write!(o, "Terminal "),
                        Variable::Launcher => write!(o, "Launcher "),
                    };
                    let _ = match value {
                        Value::Key(s) => writeln!(o, "{}", s.to_string()),
                        Value::Num(n) => writeln!(o, "{}", n),
                        Value::String(n) => writeln!(o, "{}", n),
                    };
                }
            }
        }
        String::from_utf8(o.buffer().to_vec()).unwrap()
    }
}

struct Parser<'a> {
    input: &'a [Token],
}

impl<'a> Parser<'a> {
    pub fn new(input: &'a [Token]) -> Self {
        Self { input }
    }

    pub fn parse(mut self) -> Option<Vec<Statement>> {
        let mut ss = vec![];
        while !self.done() {
            ss.push(self.parse_stmt()?);
        }
        Some(ss)
    }

    fn parse_stmt(&mut self) -> Option<Statement> {
        match self.eat() {
            Some(Token::Do) => self.parse_do(),
            Some(Token::Set) => self.parse_set(),
            _ => None,
        }
    }

    fn parse_do(&mut self) -> Option<Statement> {
        let action = self.expect_action()?;
        self.expect_on()?;
        let on = self.expect_keycombo()?;
        Some(Statement::Do { action, on })
    }
    fn parse_set(&mut self) -> Option<Statement> {
        let var = self.expect_var()?;
        let value = self.expect_value()?;
        Some(Statement::Set { var, value })
    }

    fn expect_action(&mut self) -> Option<Action> {
        match self.eat() {
            Some(Token::FocusLeft) => Some(Action::FocusLeft),
            Some(Token::FocusRight) => Some(Action::FocusRight),
            Some(Token::MoveLeft) => Some(Action::MoveLeft),
            Some(Token::MoveRight) => Some(Action::MoveRight),
            Some(Token::Launcher) => Some(Action::Launcher),
            Some(Token::Terminal) => Some(Action::Terminal),
            Some(Token::CloseWindow) => Some(Action::CloseWindow),
            Some(Token::NextWs) => Some(Action::NextWs),
            Some(Token::PrevWs) => Some(Action::PrevWs),
            other => {
                error!("Expected a variable name to be here: {other:?}");
                None
            }
        }
    }

    fn expect_on(&mut self) -> Option<()> {
        match self.eat() {
            Some(Token::On) => Some(()),
            other => {
                error!("Expected a `on` to be here: {other:?}");
                None
            }
        }
    }

    fn expect_var(&mut self) -> Option<Variable> {
        match self.eat() {
            Some(Token::Gap) => Some(Variable::Gap),
            Some(Token::MasterKey) => Some(Variable::MasterKey),
            Some(Token::Launcher) => Some(Variable::Launcher),
            Some(Token::Terminal) => Some(Variable::Terminal),
            other => {
                error!("Expected a variable name to be here: {other:?}");
                None
            }
        }
    }

    fn expect_value(&mut self) -> Option<Value> {
        match self.peek(0).cloned() {
            Some(Token::Number(n)) => {
                self.eat();
                Some(Value::Num(n))
            }
            Some(Token::Shift) => {
                self.eat();
                Some(Value::Key(SpecialKey::Shift))
            }
            Some(Token::Super) => {
                self.eat();
                Some(Value::Key(SpecialKey::Super))
            }
            Some(Token::Alt) => {
                self.eat();
                Some(Value::Key(SpecialKey::Alt))
            }
            Some(Token::Control) => {
                self.eat();
                Some(Value::Key(SpecialKey::Control))
            }
            Some(Token::Word(w)) => {
                self.eat();
                Some(Value::String(w))
            }
            _ => todo!(),
        }
    }

    fn expect_mod(&mut self) -> Option<SpecialKey> {
        match self.peek(0) {
            Some(Token::Alt) => {
                self.eat();
                Some(SpecialKey::Alt)
            }
            Some(Token::Super) => {
                self.eat();
                Some(SpecialKey::Super)
            }
            Some(Token::Shift) => {
                self.eat();
                Some(SpecialKey::Shift)
            }
            Some(Token::Control) => {
                self.eat();
                Some(SpecialKey::Control)
            }
            _ => None
        }
    }

    fn expect_dash(&mut self) -> Option<()> {
        match self.eat() {
            Some(Token::Hyphen) => Some(()),
            _ => {
                error!("Expected a hyphen here");
                None
            }
        }
    }

    fn expect_keycombo(&mut self) -> Option<KeyCombo> {
        let mut mods = vec![];
        'outer: loop {
            let modif = match self.expect_mod() {
                None => {
                    break 'outer;
                }
                Some(t) => t,
            };
            mods.push(modif);
            self.expect_dash()?;
        }
        let key_str = self.expect_key()?;

        let key = match key_str.as_str() {
            "Space" => Key::Space,
            "Return" => Key::Return,
            "Tab" => Key::Tab,
            "Escape" => Key::Escape,
            k if k.len() == 1 => Key::Char(k.chars().next().unwrap()),
            k if k.parse::<u32>().is_ok() => Key::Char(k.chars().next().unwrap()),
            _ => {
                error!("Unknown key: {}", key_str);
                return None;
            }
        };
        Some(KeyCombo {
            prefixes: mods,
            key,
        })
    }

    fn expect_key(&mut self) -> Option<String> {
        match self.eat() {
            Some(Token::Char(c)) => Some(c.to_string()),
            Some(Token::Space) => Some("Space".to_string()),
            Some(Token::Escape) => Some("Escape".to_string()),
            Some(Token::Return) => Some("Return".to_string()),
            Some(Token::Number(n)) => Some(n.to_string()),
            other => {
                error!("Unexpected token in place of a regular key: {other:?}");
                None
            }
        }
    }

    fn done(&self) -> bool {
        self.input.is_empty()
    }

    fn peek(&self, offset: usize) -> Option<&Token> {
        self.input.get(offset)
    }
    fn eat(&mut self) -> Option<Token> {
        if self.done() {
            return None;
        }
        let t = self.input[0].clone();
        if self.input.len() >= 1 {
            self.input = &self.input[1..];
        }
        Some(t)
    }
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

#[derive(Debug, Clone, Copy)]
pub enum Key {
    Char(char),
    Space,
    Return,
    Tab,
    Escape,
}

impl Into<better_x11::Key> for Key {
    fn into(self) -> better_x11::Key {
        match self {
            Self::Space => better_x11::Key::Space,
            Self::Return => better_x11::Key::Return,
            Self::Char('w') => better_x11::Key::W,
            Self::Char('h') => better_x11::Key::H,
            Self::Char('l') => better_x11::Key::L,
            Self::Char('1') => better_x11::Key::One,
            Self::Char('2') => better_x11::Key::Two,
            other => todo!("{other:?}"),
        }
    }
}

#[derive(Debug, Clone)]
pub struct KeyCombo {
    pub prefixes: Vec<SpecialKey>,
    pub key: Key,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variable {
    Gap,
    MasterKey,
    Terminal,
    Launcher,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Value {
    Num(usize),
    Key(SpecialKey),
    String(String),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Set { var: Variable, value: Value },
    Do { action: Action, on: KeyCombo },
}

#[derive(Debug, Clone)]
struct Lexer {
    input: String,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
enum Token {
    Char(char),
    Number(usize),
    Word(String),

    // Keywords
    Do,
    Set,
    On,

    MasterKey,
    Gap,

    Shift,
    Super,
    Space,
    Control,
    Alt,
    Return,
    Tab,
    Escape,

    // actions
    FocusLeft,
    FocusRight,
    MoveLeft,
    MoveRight,
    Launcher,
    Terminal,
    CloseWindow,
    NextWs,
    PrevWs,

    Hyphen,
}

impl Lexer {
    pub fn new(s: String) -> Self {
        Self { input: s, pos: 0 }
    }

    pub fn run(mut self) -> Vec<Token> {
        let mut ts: Vec<Token> = vec![];
        while !self.done() {
            match self.peek().unwrap() {
                c if c.is_whitespace() => {
                    self.eat();
                }
                c if c.is_ascii_alphabetic() || (c == '_') => {
                    let begin = self.pos;
                    while !self.done() && self.peek().unwrap().is_ascii_alphabetic()
                        || self.peek().unwrap() == '_'
                    {
                        self.eat();
                    }
                    let end = self.pos;
                    match &self.input[begin..end] {
                        "Do" => ts.push(Token::Do),
                        "Set" => ts.push(Token::Set),
                        "on" => ts.push(Token::On),
                        "MasterKey" => ts.push(Token::MasterKey),
                        "Gap" => ts.push(Token::Gap),
                        "Alt" => ts.push(Token::Alt),
                        "Control" => ts.push(Token::Control),
                        "Shift" => ts.push(Token::Shift),
                        "Space" => ts.push(Token::Space),
                        "Return" => ts.push(Token::Return),
                        "Escape" => ts.push(Token::Escape),
                        "Super" => ts.push(Token::Super),
                        "Tab" => ts.push(Token::Tab),
                        "FocusLeft" => ts.push(Token::FocusLeft),
                        "FocusRight" => ts.push(Token::FocusRight),
                        "MoveLeft" => ts.push(Token::MoveLeft),
                        "MoveRight" => ts.push(Token::MoveRight),
                        "Launcher" => ts.push(Token::Launcher),
                        "Terminal" => ts.push(Token::Terminal),
                        "CloseWindow" => ts.push(Token::CloseWindow),
                        "PrevWs" => ts.push(Token::PrevWs),
                        "NextWs" => ts.push(Token::NextWs),
                        x if x.len() == 1
                            && x.chars()
                                .nth(0)
                                .is_some_and(|c| c.is_lowercase() || c.is_ascii_digit()) =>
                        {
                            ts.push(Token::Char(x.chars().nth(0).unwrap()));
                        }
                        o => {
                            ts.push(Token::Word(o.to_string()));
                        }
                    }
                }
                c if c.is_ascii_digit() => {
                    let begin = self.pos;
                    while !self.done() && self.peek().unwrap().is_ascii_digit() {
                        self.eat();
                    }
                    let end = self.pos;
                    ts.push(Token::Number(self.input[begin..end].parse().unwrap()));
                }
                '-' => {
                    self.eat();
                    ts.push(Token::Hyphen);
                }
                '*' => {
                    while !self.done() && self.peek().unwrap() != '\n' {
                        self.eat();
                    }
                    self.eat();
                }
                x => todo!("{}", x),
            }
        }
        ts
    }

    fn done(&self) -> bool {
        self.input.len() <= self.pos
    }

    fn peek(&self) -> Option<char> {
        self.input.chars().nth(self.pos)
    }

    fn eat(&mut self) -> Option<char> {
        self.pos += 1;
        self.input.chars().nth(self.pos - 1)
    }
}
