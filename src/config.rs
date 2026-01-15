use log::{error, warn};

pub fn parse_config(content: String) -> Option<Vec<Statement>> {
    Parser::new(&Lexer::new(content).run()).parse()
}

fn validate_config(cfg: &[Statement]) -> bool {
    for s in cfg {
        match s {
            Statement::Do { action, on } => {
                let action_alt_count = action.alt_count();
                let on_alt_count = on.alt_count();
                if action_alt_count != on_alt_count {
                    error!("Invalid config found: alternative count missmatched");
                    return false;
                }
            }
            Statement::Set {..} => {}
        }
    }
    true
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
            match self.expect_word().unwrap() {
                v if v.as_str() == "Set" => {
                    ss.push(self.parse_set()?);
                }
                v if v.as_str() == "Do" => {
                    ss.push(self.parse_do()?);
                }
                x => {
                    warn!("{x:?}");
                }
            }
        }
        if !validate_config(&ss) {
            None
        } else {
            Some(ss)
        }
    }

    fn parse_do(&mut self) -> Option<Statement> {
        let what = self.parse_action()?;
        match self.eat() {
            None => {
                error!("Missing `on` keyword");
                return None;
            }
            Some(Token::Word(x)) if x.as_str() == "on" => {}
            Some(other) => {
                error!("Expected `on` keyword found: {other:?}");
                return None;
            }
        }
        let combo = self.parse_bind()?;
        Some(Statement::Do {
            action: what,
            on: combo,
        })
    }

    fn parse_set(&mut self) -> Option<Statement> {
        let var = self.parse_variable()?;
        let value = match self.eat() {
            Some(Token::Word(w)) => w,
            Some(Token::Number(w)) => w.to_string(),
            Some(other) => {
                error!("Unexpected config value found {other:?}");
                return None;
            }
            None => {
                error!("Expected config value");
                return None;
            }
        };
        Some(Statement::Set { var, value: value })
    }

    fn parse_variable(&mut self) -> Option<Variable> {
        match self.expect_word() {
            Some(x) if x.as_str() == "Gap" => Some(Variable::Gap),
            Some(x) if x.as_str() == "MasterKey" => Some(Variable::MasterKey),
            _ => {
                error!("Expected a variable name to be here");
                None
            }
        }
    }

    fn expect_word(&mut self) -> Option<String> {
        match self.eat() {
            Some(Token::Word(w)) => Some(w),
            _ => None,
        }
    }

    fn parse_action(&mut self) -> Option<Action> {
        match self.eat() {
            Some(Token::Word(w)) if w.as_str() == "FocusLeft" => Some(Action::FocusLeft),
            Some(Token::Word(w)) if w.as_str() == "FocusRight" => Some(Action::FocusRight),
            Some(Token::OpenCurly) => {
                let mut alts = vec![];
                while !matches!(self.peek(0), Some(Token::CloseCurly)) {
                    alts.push(self.parse_action()?);
                }
                self.eat();
                Some(Action::Alt(alts))
            }
            Some(other) => {
                error!("Unexpected config action found {other:?}");
                return None;
            }
            None => {
                error!("Expected config action name");
                return None;
            }
        }
    }

    fn parse_bind(&mut self) -> Option<KeyCombo> {
        let mut prefixes = vec![];
        let mut has_alt = false;
        loop {
            let prefix = self.parse_bind_atom(&mut has_alt)?;
            if matches!(self.peek(0), Some(Token::Hyphen)) {
                self.eat();
                prefixes.push(prefix);
            } else {
                return Some(if prefixes.is_empty() {
                    prefix
                } else {
                    KeyCombo::Prefixed {
                        prefixes,
                        key: Box::new(prefix),
                    }
                });
            }
        }
    }

    fn parse_bind_atom(&mut self, has_alt: &mut bool) -> Option<KeyCombo> {
        match self.eat() {
            None => {
                return None;
            }
            Some(Token::Number(n)) => {
                return Some(KeyCombo::Char(n.to_string().chars().nth(0).unwrap()));
            }
            Some(Token::Word(x)) if x.len() == 1 && x.chars().nth(0).unwrap().is_lowercase() => {
                return Some(KeyCombo::Char(x.chars().nth(0).unwrap()));
            }
            Some(Token::Word(x)) if x.len() == 1 && x.chars().nth(0).unwrap().is_uppercase() => {
                match x.chars().nth(0).unwrap() {
                    'M' => Some(KeyCombo::Master),
                    'A' => Some(KeyCombo::Alt),
                    'S' => Some(KeyCombo::Space),
                    'C' => Some(KeyCombo::Control),
                    'H' => Some(KeyCombo::Shift),
                    _ => {
                        todo!()
                    }
                }
            }
            Some(Token::OpenCurly) => {
                if *has_alt {
                    error!("Found a second alternative within a pattern");
                    return None;
                }
                *has_alt = true;
                let mut alts = vec![];
                while !matches!(self.peek(0), Some(Token::CloseCurly)) {
                    alts.push(self.parse_bind_atom(has_alt)?);
                }
                self.eat();
                Some(KeyCombo::Alternative(alts))
            }
            _ => todo!(),
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

#[derive(Debug, Clone)]
pub enum Action {
    Alt(Vec<Action>),
    FocusLeft,
    FocusRight,
}

impl Action {
    pub fn alt_count(&self) -> Option<usize> {
        match self {
            Self::Alt(n) => Some(n.len()),
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
pub enum KeyCombo {
    Char(char),
    Alternative(Vec<KeyCombo>),
    Master,
    Super,
    Shift,
    Alt,
    Control,
    Space,
    Prefixed {
        prefixes: Vec<KeyCombo>,
        key: Box<KeyCombo>,
    },
}
impl KeyCombo {
    pub fn alt_count(&self) -> Option<usize> {
        match self {
            Self::Alternative(s) => Some(s.len()),
            Self::Prefixed { prefixes, key } => {
                let mut count = None;
                for p in prefixes {
                    if let Some(c) = p.alt_count() {
                        count = Some(c);
                    }
                }
                if count.is_none() {
                    count = key.alt_count();
                }
                count
            }
            _ => None
        }
    }
}

#[derive(Debug, Clone)]
pub enum Variable {
    Gap,
    MasterKey,
}

#[derive(Debug, Clone)]
pub enum Statement {
    Set { var: Variable, value: String },
    Do { action: Action, on: KeyCombo },
}

#[derive(Debug, Clone)]
pub struct Lexer {
    input: String,
    pos: usize,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Token {
    Word(String),
    Number(usize),
    Hyphen,
    OpenCurly,
    CloseCurly,
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
                    ts.push(Token::Word(self.input[begin..end].to_string()));
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
                '{' => {
                    self.eat();
                    ts.push(Token::OpenCurly);
                }
                '}' => {
                    self.eat();
                    ts.push(Token::CloseCurly);
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
