use log::{error, warn};

pub fn parse(input: String) -> Option<Vec<Statement>> {
    Parser::new(&Lexer::new(input).run()).parse()
}

pub struct Parser<'a> {
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
            other => {
                error!("Unexpected value (number or key): {other:?}");
                None
            }
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
            _ => None,
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
        let key = match self.eat() {
            Some(Token::Space) => ' ',
            Some(Token::Return) => '\n',
            Some(Token::Tab) => '\t',
            Some(Token::Escape) => '\x1b',
            Some(Token::Char(c)) => c,
            Some(Token::Number(c)) => c.to_string().chars().nth(0).unwrap(),
            k => {
                error!("Expected a non-special key here: {k:?}");
                return None;
            }
        };
        Some(KeyCombo {
            prefixes: mods,
            key,
        })
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

#[derive(Debug, Clone)]
pub struct KeyCombo {
    pub prefixes: Vec<SpecialKey>,
    pub key: char,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Variable {
    Gap,
    MasterKey,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Value {
    Num(usize),
    Key(SpecialKey),
}

#[derive(Debug, Clone)]
pub enum Statement {
    Set { var: Variable, value: Value },
    Do { action: Action, on: KeyCombo },
}

#[derive(Debug, Clone)]
pub struct Lexer {
    input: String,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Char(char),
    Number(usize),

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
                c if c.is_ascii_alphabetic() => {
                    let begin = self.pos;
                    while !self.done() && self.peek().unwrap().is_ascii_alphabetic() {
                        self.eat();
                    }
                    let end = self.pos;
                    match &self.input[begin..end] {
                        "Do" => {
                            ts.push(Token::Do);
                        }
                        "Set" => {
                            ts.push(Token::Set);
                        }
                        "on" => {
                            ts.push(Token::On);
                        }
                        "MasterKey" => {
                            ts.push(Token::MasterKey);
                        }
                        "Gap" => {
                            ts.push(Token::Gap);
                        }
                        "Alt" => {
                            ts.push(Token::Alt);
                        }
                        "Control" => {
                            ts.push(Token::Control);
                        }
                        "Shift" => {
                            ts.push(Token::Shift);
                        }
                        "Space" => {
                            ts.push(Token::Space);
                        }
                        "Return" => {
                            ts.push(Token::Return);
                        }
                        "Escape" => {
                            ts.push(Token::Escape);
                        }
                        "Super" => {
                            ts.push(Token::Super);
                        }
                        "Tab" => {
                            ts.push(Token::Tab);
                        }
                        "FocusLeft" => {
                            ts.push(Token::FocusLeft);
                        }
                        "FocusRight" => {
                            ts.push(Token::FocusRight);
                        }
                        "Launcher" => {
                            ts.push(Token::Launcher);
                        }
                        "Terminal" => {
                            ts.push(Token::Terminal);
                        }
                        "CloseWindow" => {
                            ts.push(Token::CloseWindow);
                        }
                        "PrevWs" => {
                            ts.push(Token::PrevWs);
                        }
                        "NextWs" => {
                            ts.push(Token::NextWs);
                        }
                        x if x.len() == 1
                            && x.chars()
                                .nth(0)
                                .is_some_and(|c| c.is_lowercase() || c.is_ascii_digit()) =>
                        {
                            ts.push(Token::Char(x.chars().nth(0).unwrap()));
                        }
                        o => todo!("invalid ident {o}"),
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
                _ => todo!(),
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
