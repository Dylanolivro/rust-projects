enum Token {
    LeftBrace,
    RightBrace,
    LeftBracket,
    RightBracket,
    Colon,
    Comma,
    StringToken(String),
    NumberToken(f64),
    True,
    False,
    Null,
}

enum JsonValue {
    Object(Vec<(String, JsonValue)>),
    Array(Vec<JsonValue>),
    Str(String),
    Number(f64),
    Bool(bool),
    Null,
}

struct Lexer {
    input: Vec<u8>,
    pos: usize,
}

impl Lexer {
    fn new(input: &str) -> Lexer {
        Lexer {
            input: input.as_bytes().to_vec(),
            pos: 0,
        }
    }

    fn current(&self) -> Option<u8> {
        if self.pos < self.input.len() {
            Some(self.input[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn read_keyword(&mut self, keyword: &str) {
        for expected in keyword.as_bytes() {
            match self.current() {
                Some(c) if c == *expected => self.advance(),
                Some(c) => panic!("Unexpected '{}' reading keyword '{}'", c as char, keyword),
                None => panic!("Unexpected end of input reading keyword '{}'", keyword),
            }
        }
    }

    fn read_string(&mut self) -> String {
        self.advance();
        let mut result = String::new();
        loop {
            match self.current() {
                None => panic!("Unterminated string"),
                Some(b'"') => {
                    self.advance();
                    return result;
                }
                Some(b'\\') => {
                    self.advance();
                    match self.current() {
                        Some(b'"') => {
                            result.push('"');
                            self.advance();
                        }
                        Some(b'\\') => {
                            result.push('\\');
                            self.advance();
                        }
                        Some(b'/') => {
                            result.push('/');
                            self.advance();
                        }
                        Some(b'n') => {
                            result.push('\n');
                            self.advance();
                        }
                        Some(b't') => {
                            result.push('\t');
                            self.advance();
                        }
                        Some(b'r') => {
                            result.push('\r');
                            self.advance();
                        }
                        Some(b'b') => {
                            result.push('\x08');
                            self.advance();
                        }
                        Some(b'f') => {
                            result.push('\x0C');
                            self.advance();
                        }
                        Some(b'u') => {
                            self.advance();
                            let cp = self.read_unicode_escape();
                            let ch = char::from_u32(cp)
                                .unwrap_or_else(|| panic!("Invalid unicode codepoint: {}", cp));
                            result.push(ch);
                        }
                        Some(c) => panic!("Invalid escape: \\{}", c as char),
                        None => panic!("Unterminated escape"),
                    }
                }
                Some(c) => {
                    result.push(c as char);
                    self.advance();
                }
            }
        }
    }

    fn read_unicode_escape(&mut self) -> u32 {
        let mut value: u32 = 0;
        for _ in 0..4 {
            match self.current() {
                Some(c) => {
                    let digit = match c {
                        b'0'..=b'9' => (c - b'0') as u32,
                        b'a'..=b'f' => (c - b'a' + 10) as u32,
                        b'A'..=b'F' => (c - b'A' + 10) as u32,
                        _ => panic!("Invalid hex digit: {}", c as char),
                    };
                    value = value * 16 + digit;
                    self.advance();
                }
                None => panic!("Unterminated unicode escape"),
            }
        }
        value
    }

    fn read_number(&mut self) -> f64 {
        let mut s = String::new();
        if let Some(b'-') = self.current() {
            s.push('-');
            self.advance();
        }
        loop {
            match self.current() {
                Some(c @ b'0'..=b'9') => {
                    s.push(c as char);
                    self.advance();
                }
                _ => break,
            }
        }
        if let Some(b'.') = self.current() {
            s.push('.');
            self.advance();
            loop {
                match self.current() {
                    Some(c @ b'0'..=b'9') => {
                        s.push(c as char);
                        self.advance();
                    }
                    _ => break,
                }
            }
        }
        s.parse::<f64>()
            .unwrap_or_else(|_| panic!("Invalid number: {}", s))
    }
}

fn tokenize(input: &str) -> Vec<Token> {
    let mut lexer = Lexer::new(input);
    let mut tokens = Vec::new();
    loop {
        match lexer.current() {
            None => break,
            Some(b' ') | Some(b'\t') | Some(b'\n') | Some(b'\r') => {
                lexer.advance();
            }
            Some(b'{') => {
                tokens.push(Token::LeftBrace);
                lexer.advance();
            }
            Some(b'}') => {
                tokens.push(Token::RightBrace);
                lexer.advance();
            }
            Some(b'[') => {
                tokens.push(Token::LeftBracket);
                lexer.advance();
            }
            Some(b']') => {
                tokens.push(Token::RightBracket);
                lexer.advance();
            }
            Some(b':') => {
                tokens.push(Token::Colon);
                lexer.advance();
            }
            Some(b',') => {
                tokens.push(Token::Comma);
                lexer.advance();
            }
            Some(b'"') => {
                let s = lexer.read_string();
                tokens.push(Token::StringToken(s));
            }
            Some(b't') => {
                lexer.read_keyword("true");
                tokens.push(Token::True);
            }
            Some(b'f') => {
                lexer.read_keyword("false");
                tokens.push(Token::False);
            }
            Some(b'n') => {
                lexer.read_keyword("null");
                tokens.push(Token::Null);
            }
            Some(b'-') | Some(b'0'..=b'9') => {
                let n = lexer.read_number();
                tokens.push(Token::NumberToken(n));
            }
            Some(c) => panic!("Unexpected character: {}", c as char),
        }
    }
    tokens
}

struct Parser {
    tokens: Vec<Token>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<Token>) -> Parser {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> &Token {
        if self.pos < self.tokens.len() {
            &self.tokens[self.pos]
        } else {
            panic!("Unexpected end of token stream");
        }
    }

    fn advance(&mut self) {
        self.pos += 1;
    }

    fn expect(&mut self, description: &str) -> &Token {
        if self.pos < self.tokens.len() {
            let token = &self.tokens[self.pos];
            self.pos += 1;
            token
        } else {
            panic!("Expected {} but reached end of input", description);
        }
    }

    fn parse_value(&mut self) -> JsonValue {
        match self.current() {
            Token::LeftBrace => self.parse_object(),
            Token::LeftBracket => self.parse_array(),
            Token::True => {
                self.advance();
                JsonValue::Bool(true)
            }
            Token::False => {
                self.advance();
                JsonValue::Bool(false)
            }
            Token::Null => {
                self.advance();
                JsonValue::Null
            }
            Token::NumberToken(n) => {
                let v = *n;
                self.advance();
                JsonValue::Number(v)
            }
            Token::StringToken(s) => {
                let v = s.clone();
                self.advance();
                JsonValue::Str(v)
            }
            Token::RightBrace => panic!("Unexpected '}}'"),
            Token::RightBracket => panic!("Unexpected ']'"),
            Token::Colon => panic!("Unexpected ':'"),
            Token::Comma => panic!("Unexpected ','"),
        }
    }

    fn parse_object(&mut self) -> JsonValue {
        self.advance();
        let mut pairs: Vec<(String, JsonValue)> = Vec::new();

        if let Token::RightBrace = self.current() {
            self.advance();
            return JsonValue::Object(pairs);
        }

        loop {
            let key = match self.expect("object key") {
                Token::StringToken(s) => s.clone(),
                _ => panic!("Expected string key in object"),
            };

            match self.expect("colon") {
                Token::Colon => {}
                _ => panic!("Expected ':' after object key"),
            }

            let value = self.parse_value();
            pairs.push((key, value));

            match self.current() {
                Token::Comma => {
                    self.advance();
                }
                Token::RightBrace => {
                    self.advance();
                    break;
                }
                _ => panic!("Expected ',' or '}}' in object"),
            }
        }

        JsonValue::Object(pairs)
    }

    fn parse_array(&mut self) -> JsonValue {
        self.advance();
        let mut elements: Vec<JsonValue> = Vec::new();

        if let Token::RightBracket = self.current() {
            self.advance();
            return JsonValue::Array(elements);
        }

        loop {
            let element = self.parse_value();
            elements.push(element);

            match self.current() {
                Token::Comma => {
                    self.advance();
                }
                Token::RightBracket => {
                    self.advance();
                    break;
                }
                _ => panic!("Expected ',' or ']' in array"),
            }
        }

        JsonValue::Array(elements)
    }
}

fn parse(tokens: Vec<Token>) -> JsonValue {
    let mut parser = Parser::new(tokens);
    parser.parse_value()
}

fn display(value: &JsonValue) -> String {
    match value {
        JsonValue::Null => String::from("null"),
        JsonValue::Bool(true) => String::from("true"),
        JsonValue::Bool(false) => String::from("false"),
        JsonValue::Number(n) => format!("{}", n),
        JsonValue::Str(s) => format!("\"{}\"", s),
        JsonValue::Array(elements) => {
            let mut result = String::from("[");
            let mut first = true;
            for element in elements {
                if !first {
                    result.push_str(", ");
                }
                result.push_str(&display(element));
                first = false;
            }
            result.push(']');
            result
        }
        JsonValue::Object(pairs) => {
            let mut result = String::from("{");
            let mut first = true;
            for (key, val) in pairs {
                if !first {
                    result.push_str(", ");
                }
                result.push_str(&format!("\"{}\": {}", key, display(val)));
                first = false;
            }
            result.push('}');
            result
        }
    }
}

fn main() {
    let tests = vec![
        r#"null"#,
        r#"true"#,
        r#"42"#,
        r#"-3.14"#,
        r#""hello world""#,
        r#"[]"#,
        r#"{}"#,
        r#"[1, 2, 3]"#,
        r#"{"name": "alice", "age": 30, "active": true}"#,
        r#"{"scores": [95, 87, 100], "info": {"city": "delhi", "zip": null}}"#,
        r#"["hello\nworld", "tab\there", "\u0041\u0042\u0043"]"#,
    ];

    for input in &tests {
        let tokens = tokenize(input);
        let value = parse(tokens);
        println!("Input:  {}", input);
        println!("Output: {}", display(&value));
        println!();
    }
}
