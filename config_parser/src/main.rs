use std::fs;

#[derive(Debug, thiserror::Error)]
enum ConfigError {
    #[error(
        "unexpected character at line {line}, column {col}: expected {expected}, found '{found}'"
    )]
    UnexpectedCharacter {
        line: usize,
        col: usize,
        expected: String,
        found: String,
    },
    #[error("unterminated string starting at line {line}, column {col}")]
    UnterminatedString { line: usize, col: usize },
    #[error("invalid number at line {line}, column {col}: {detail}")]
    InvalidNumber {
        line: usize,
        col: usize,
        detail: String,
    },
    #[error("expected {expected} at line {line}, column {col}, found {found}")]
    ExpectedToken {
        line: usize,
        col: usize,
        expected: String,
        found: String,
    },
    #[error("missing value for key '{key}' at line {line}, column {col}")]
    MissingValue {
        line: usize,
        col: usize,
        key: String,
    },
    #[error("schema violation at line {line}: key '{key}' expected type {expected}, found {found}")]
    SchemaViolation {
        line: usize,
        key: String,
        expected: String,
        found: String,
    },
    #[error("missing required key '{key}'")]
    MissingRequiredKey { key: String },
    #[error("key '{key}' has value {value} which is outside range {min} to {max}")]
    ValueOutOfRange {
        key: String,
        value: String,
        min: i64,
        max: i64,
    },
    #[error("unknown key '{key}' at line {line}")]
    UnknownKey { key: String, line: usize },
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
}

#[derive(Debug, Clone, PartialEq)]
enum TokenKind {
    Equal,
    StringLit(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Comment(String),
    Identifier(String),
    TableHeader(String),
    Newline,
}

#[derive(Debug, Clone)]
struct SpannedToken {
    kind: TokenKind,
    line: usize,
    col: usize,
}

#[derive(Debug, Clone)]
struct Lexer {
    input: Vec<u8>,
    pos: usize,
    line: usize,
    col: usize,
}

impl Lexer {
    fn new(input: &str) -> Lexer {
        Lexer {
            input: input.as_bytes().to_vec(),
            pos: 0,
            line: 1,
            col: 1,
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
        if self.pos < self.input.len() {
            if self.input[self.pos] == b'\n' {
                self.line += 1;
                self.col = 1;
            } else {
                self.col += 1;
            }
            self.pos += 1;
        }
    }

    fn peek(&self) -> Option<u8> {
        if self.pos + 1 < self.input.len() {
            Some(self.input[self.pos + 1])
        } else {
            None
        }
    }

    fn skip_whitespace_inline(&mut self) {
        while let Some(c) = self.current() {
            if c == b' ' || c == b'\t' {
                self.advance();
            } else {
                break;
            }
        }
    }

    fn tokenize(&mut self) -> Result<Vec<SpannedToken>, ConfigError> {
        let mut tokens = Vec::new();

        while let Some(c) = self.current() {
            match c {
                b' ' | b'\t' | b'\r' => {
                    self.advance();
                }
                b'\n' => {
                    tokens.push(SpannedToken {
                        kind: TokenKind::Newline,
                        line: self.line,
                        col: self.col,
                    });
                    self.advance();
                }
                b'#' => {
                    let comment = self.read_comment()?;
                    tokens.push(SpannedToken {
                        kind: TokenKind::Comment(comment),
                        line: self.line,
                        col: self.col,
                    });
                }
                b'[' => {
                    let (key, line, col) = self.read_table_header()?;
                    tokens.push(SpannedToken {
                        kind: TokenKind::TableHeader(key),
                        line,
                        col,
                    });
                }
                b'=' => {
                    tokens.push(SpannedToken {
                        kind: TokenKind::Equal,
                        line: self.line,
                        col: self.col,
                    });
                    self.advance();
                }
                b'"' => {
                    let (s, line, col) = self.read_string()?;
                    tokens.push(SpannedToken {
                        kind: TokenKind::StringLit(s),
                        line,
                        col,
                    });
                }
                b'0'..=b'9' | b'-' if !is_alpha(self.peek().unwrap_or(b'\0')) => {
                    let (tok, line, col) = self.read_number()?;
                    tokens.push(SpannedToken {
                        kind: tok,
                        line,
                        col,
                    });
                }
                b'a'..=b'z' | b'A'..=b'Z' | b'_' => {
                    let (word, line, col) = self.read_identifier();
                    tokens.push(SpannedToken {
                        kind: match word.as_str() {
                            "true" => TokenKind::Boolean(true),
                            "false" => TokenKind::Boolean(false),
                            other => TokenKind::Identifier(other.to_string()),
                        },
                        line,
                        col,
                    });
                }
                _ => {
                    let found = (c as char).to_string();
                    return Err(ConfigError::UnexpectedCharacter {
                        line: self.line,
                        col: self.col,
                        expected: "valid TOML syntax".to_string(),
                        found,
                    });
                }
            }
        }

        Ok(tokens)
    }

    fn read_comment(&mut self) -> Result<String, ConfigError> {
        self.advance();
        let mut result = String::new();
        while let Some(c) = self.current() {
            if c == b'\n' {
                break;
            }
            result.push(c as char);
            self.advance();
        }
        Ok(result)
    }

    fn read_table_header(&mut self) -> Result<(String, usize, usize), ConfigError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance();

        let mut dotted = String::new();
        self.skip_whitespace_inline();

        while let Some(c) = self.current() {
            match c {
                b']' => {
                    self.advance();
                    return Ok((dotted, start_line, start_col));
                }
                b'.' => {
                    dotted.push('.');
                    self.advance();
                    self.skip_whitespace_inline();
                }
                b'\n' => {
                    return Err(ConfigError::UnterminatedString {
                        line: start_line,
                        col: start_col,
                    });
                }
                b' ' | b'\t' => {
                    self.advance();
                }
                _ => {
                    if is_alpha(c) || is_digit(c) || c == b'_' || c == b'-' {
                        dotted.push(c as char);
                        self.advance();
                    } else {
                        return Err(ConfigError::UnexpectedCharacter {
                            line: self.line,
                            col: self.col,
                            expected: "table key or ']'".to_string(),
                            found: (c as char).to_string(),
                        });
                    }
                }
            }
        }

        Err(ConfigError::UnterminatedString {
            line: start_line,
            col: start_col,
        })
    }

    fn read_string(&mut self) -> Result<(String, usize, usize), ConfigError> {
        let start_line = self.line;
        let start_col = self.col;
        self.advance();

        let mut result = String::new();
        loop {
            match self.current() {
                None => {
                    return Err(ConfigError::UnterminatedString {
                        line: start_line,
                        col: start_col,
                    });
                }
                Some(b'"') => {
                    self.advance();
                    return Ok((result, start_line, start_col));
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
                        _ => {
                            let found = match self.current() {
                                Some(b) => (b as char).to_string(),
                                None => String::from("end of input"),
                            };
                            return Err(ConfigError::UnexpectedCharacter {
                                line: self.line,
                                col: self.col,
                                expected: "valid escape sequence".to_string(),
                                found,
                            });
                        }
                    }
                }
                Some(c) => {
                    result.push(c as char);
                    self.advance();
                }
            }
        }
    }

    fn read_number(&mut self) -> Result<(TokenKind, usize, usize), ConfigError> {
        let start_line = self.line;
        let start_col = self.col;
        let mut s = String::new();
        let mut has_dot = false;

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
                Some(b'.') => {
                    if has_dot {
                        return Err(ConfigError::InvalidNumber {
                            line: start_line,
                            col: start_col,
                            detail: "multiple decimal points".to_string(),
                        });
                    }
                    has_dot = true;
                    s.push('.');
                    self.advance();
                }
                _ => break,
            }
        }

        if has_dot {
            let value: f64 = match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    return Err(ConfigError::InvalidNumber {
                        line: start_line,
                        col: start_col,
                        detail: format!("cannot parse '{}' as float", s),
                    });
                }
            };
            Ok((TokenKind::Float(value), start_line, start_col))
        } else {
            let value: i64 = match s.parse() {
                Ok(v) => v,
                Err(_) => {
                    return Err(ConfigError::InvalidNumber {
                        line: start_line,
                        col: start_col,
                        detail: format!("cannot parse '{}' as integer", s),
                    });
                }
            };
            Ok((TokenKind::Integer(value), start_line, start_col))
        }
    }

    fn read_identifier(&mut self) -> (String, usize, usize) {
        let start_line = self.line;
        let start_col = self.col;
        let mut result = String::new();
        while let Some(c) = self.current() {
            if is_alpha(c) || is_digit(c) || c == b'_' || c == b'-' {
                result.push(c as char);
                self.advance();
            } else {
                break;
            }
        }
        (result, start_line, start_col)
    }
}

fn is_alpha(b: u8) -> bool {
    b.is_ascii_alphabetic()
}

fn is_digit(b: u8) -> bool {
    b.is_ascii_digit()
}

#[derive(Debug, Clone, PartialEq)]
enum TomlValue {
    String(String),
    Integer(i64),
    Float(f64),
    Boolean(bool),
    Table(Vec<(String, TomlValue)>),
}

fn display(value: &TomlValue) -> String {
    match value {
        TomlValue::String(s) => format!("\"{}\"", s),
        TomlValue::Integer(n) => format!("{}", n),
        TomlValue::Float(n) => format!("{}", n),
        TomlValue::Boolean(b) => format!("{}", b),
        TomlValue::Table(pairs) => {
            let mut result = String::from("{");
            let mut first = true;
            for (k, v) in pairs {
                if !first {
                    result.push_str(", ");
                }
                result.push_str(&format!("{}: {}", k, display(v)));
                first = false;
            }
            result.push('}');
            result
        }
    }
}

fn toml_type_name(value: &TomlValue) -> &'static str {
    match value {
        TomlValue::String(_) => "string",
        TomlValue::Integer(_) => "integer",
        TomlValue::Float(_) => "float",
        TomlValue::Boolean(_) => "boolean",
        TomlValue::Table(_) => "table",
    }
}

#[derive(Debug, Clone)]
struct Parser {
    tokens: Vec<SpannedToken>,
    pos: usize,
}

impl Parser {
    fn new(tokens: Vec<SpannedToken>) -> Parser {
        Parser { tokens, pos: 0 }
    }

    fn current(&self) -> Option<&SpannedToken> {
        if self.pos < self.tokens.len() {
            Some(&self.tokens[self.pos])
        } else {
            None
        }
    }

    fn advance(&mut self) {
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
    }

    fn parse(&mut self) -> Result<TomlValue, ConfigError> {
        let mut pairs: Vec<(String, TomlValue)> = Vec::new();

        while let Some(token) = self.current() {
            match &token.kind {
                TokenKind::Comment(_) | TokenKind::Newline => {
                    self.advance();
                }
                TokenKind::TableHeader(section) => {
                    let section_name = section.clone();
                    self.advance();
                    self.skip_newlines();

                    let mut section_pairs: Vec<(String, TomlValue)> = Vec::new();

                    while let Some(t) = self.current() {
                        match &t.kind {
                            TokenKind::Comment(_) | TokenKind::Newline => {
                                self.advance();
                            }
                            TokenKind::TableHeader(_) => break,
                            TokenKind::Identifier(key) => {
                                let key_str = key.clone();
                                let key_line = t.line;
                                self.advance();

                                let eq_token = self.current();
                                match eq_token {
                                    Some(eq) if eq.kind == TokenKind::Equal => {
                                        self.advance();
                                    }
                                    Some(eq) => {
                                        return Err(ConfigError::ExpectedToken {
                                            line: eq.line,
                                            col: eq.col,
                                            expected: "=".to_string(),
                                            found: format!("{:?}", eq.kind),
                                        });
                                    }
                                    None => {
                                        return Err(ConfigError::MissingValue {
                                            line: key_line,
                                            col: 1,
                                            key: key_str,
                                        });
                                    }
                                }

                                let value = self.parse_value()?;
                                section_pairs.push((key_str, value));
                                self.skip_newlines();
                            }
                            _ => {
                                return Err(ConfigError::UnexpectedCharacter {
                                    line: t.line,
                                    col: t.col,
                                    expected: "key or table header".to_string(),
                                    found: format!("{:?}", t.kind),
                                });
                            }
                        }
                    }

                    pairs.push((section_name, TomlValue::Table(section_pairs)));
                }
                TokenKind::Identifier(key) => {
                    let key_str = key.clone();
                    let key_line = token.line;
                    self.advance();

                    let eq_token = self.current();
                    match eq_token {
                        Some(t) if t.kind == TokenKind::Equal => {
                            self.advance();
                        }
                        Some(t) => {
                            return Err(ConfigError::ExpectedToken {
                                line: t.line,
                                col: t.col,
                                expected: "=".to_string(),
                                found: format!("{:?}", t.kind),
                            });
                        }
                        None => {
                            return Err(ConfigError::MissingValue {
                                line: key_line,
                                col: 1,
                                key: key_str,
                            });
                        }
                    }

                    let value = self.parse_value()?;
                    pairs.push((key_str, value));
                    self.skip_newlines();
                }
                _ => {
                    return Err(ConfigError::UnexpectedCharacter {
                        line: token.line,
                        col: token.col,
                        expected: "key, table header, or comment".to_string(),
                        found: format!("{:?}", token.kind),
                    });
                }
            }
        }

        Ok(TomlValue::Table(pairs))
    }

    fn parse_value(&mut self) -> Result<TomlValue, ConfigError> {
        let token = self.current();
        match token {
            Some(t) => match &t.kind {
                TokenKind::StringLit(s) => {
                    let val = TomlValue::String(s.clone());
                    self.advance();
                    Ok(val)
                }
                TokenKind::Integer(n) => {
                    let val = TomlValue::Integer(*n);
                    self.advance();
                    Ok(val)
                }
                TokenKind::Float(n) => {
                    let val = TomlValue::Float(*n);
                    self.advance();
                    Ok(val)
                }
                TokenKind::Boolean(b) => {
                    let val = TomlValue::Boolean(*b);
                    self.advance();
                    Ok(val)
                }
                _ => Err(ConfigError::ExpectedToken {
                    line: t.line,
                    col: t.col,
                    expected: "value (string, integer, float, or boolean)".to_string(),
                    found: format!("{:?}", t.kind),
                }),
            },
            None => Err(ConfigError::ExpectedToken {
                line: 0,
                col: 0,
                expected: "value".to_string(),
                found: "end of input".to_string(),
            }),
        }
    }

    fn skip_newlines(&mut self) {
        while let Some(token) = self.current() {
            match &token.kind {
                TokenKind::Newline | TokenKind::Comment(_) => {
                    self.advance();
                }
                _ => break,
            }
        }
    }
}

#[derive(Debug, Clone)]
enum FieldType {
    String,
    Integer,
    Float,
    Boolean,
}

impl FieldType {
    fn from_str(s: &str) -> Option<FieldType> {
        match s {
            "string" => Some(FieldType::String),
            "integer" => Some(FieldType::Integer),
            "float" => Some(FieldType::Float),
            "boolean" => Some(FieldType::Boolean),
            _ => None,
        }
    }

    fn name(&self) -> &'static str {
        match self {
            FieldType::String => "string",
            FieldType::Integer => "integer",
            FieldType::Float => "float",
            FieldType::Boolean => "boolean",
        }
    }
}

#[derive(Debug, Clone)]
struct FieldSchema {
    field_type: FieldType,
    required: bool,
    default: Option<TomlValue>,
    min: Option<i64>,
    max: Option<i64>,
}

type Schema = Vec<(String, FieldSchema)>;

fn parse_schema(value: &TomlValue) -> Result<Schema, ConfigError> {
    let mut schema = Vec::new();
    let table = match value {
        TomlValue::Table(pairs) => pairs,
        _ => {
            return Err(ConfigError::UnexpectedCharacter {
                line: 0,
                col: 0,
                expected: "table".to_string(),
                found: "non-table value".to_string(),
            });
        }
    };

    for (key, val) in table {
        let inner = match val {
            TomlValue::Table(pairs) => pairs,
            _ => continue,
        };

        let mut field_type = FieldType::String;
        let mut required = false;
        let mut default_val: Option<TomlValue> = None;
        let mut min_val: Option<i64> = None;
        let mut max_val: Option<i64> = None;

        for (field_name, field_val) in inner {
            match field_name.as_str() {
                "type" => {
                    let type_str = match field_val {
                        TomlValue::String(s) => s.clone(),
                        _ => String::new(),
                    };
                    let ft = FieldType::from_str(&type_str);
                    match ft {
                        Some(t) => field_type = t,
                        None => {
                            return Err(ConfigError::SchemaViolation {
                                line: 0,
                                key: key.clone(),
                                expected: "valid type (string, integer, float, boolean)"
                                    .to_string(),
                                found: type_str,
                            });
                        }
                    }
                }
                "required" => {
                    required = match field_val {
                        TomlValue::Boolean(b) => *b,
                        _ => false,
                    };
                }
                "default" => {
                    default_val = Some(field_val.clone());
                }
                "min" => {
                    min_val = match field_val {
                        TomlValue::Integer(n) => Some(*n),
                        _ => None,
                    };
                }
                "max" => {
                    max_val = match field_val {
                        TomlValue::Integer(n) => Some(*n),
                        _ => None,
                    };
                }
                _ => {}
            }
        }

        schema.push((
            key.clone(),
            FieldSchema {
                field_type,
                required,
                default: default_val,
                min: min_val,
                max: max_val,
            },
        ));
    }

    Ok(schema)
}

fn flatten_table(table: &TomlValue, prefix: &str) -> Vec<(String, TomlValue)> {
    let pairs = match table {
        TomlValue::Table(pairs) => pairs,
        _ => return vec![],
    };

    let mut result = Vec::new();
    for (key, value) in pairs {
        let full_key = if prefix.is_empty() {
            key.clone()
        } else {
            format!("{}.{}", prefix, key)
        };

        match value {
            TomlValue::Table(_) => {
                let nested = flatten_table(value, &full_key);
                for item in nested {
                    result.push(item);
                }
            }
            _ => {
                result.push((full_key, value.clone()));
            }
        }
    }
    result
}

fn validate(config: &TomlValue, schema: &Schema, source: &str) -> Result<(), Vec<ConfigError>> {
    let mut errors = Vec::new();

    let flat_config = flatten_table(config, "");

    for (key, field) in schema {
        let mut found_value: Option<&TomlValue> = None;
        for (k, v) in &flat_config {
            if k == key {
                found_value = Some(v);
                break;
            }
        }

        match found_value {
            None => {
                if field.required {
                    match &field.default {
                        Some(_) => {}
                        None => {
                            errors.push(ConfigError::MissingRequiredKey { key: key.clone() });
                        }
                    }
                }
            }
            Some(value) => {
                let type_matches = match (&field.field_type, value) {
                    (FieldType::String, TomlValue::String(_)) => true,
                    (FieldType::Integer, TomlValue::Integer(_)) => true,
                    (FieldType::Float, TomlValue::Float(_)) => true,
                    (FieldType::Float, TomlValue::Integer(_)) => true,
                    (FieldType::Boolean, TomlValue::Boolean(_)) => true,
                    _ => false,
                };

                if !type_matches {
                    let line = find_line_for_key(source, key);
                    errors.push(ConfigError::SchemaViolation {
                        line,
                        key: key.clone(),
                        expected: field.field_type.name().to_string(),
                        found: toml_type_name(value).to_string(),
                    });
                }

                if let (TomlValue::Integer(n), Some(min)) = (value, field.min) {
                    if *n < min {
                        errors.push(ConfigError::ValueOutOfRange {
                            key: key.clone(),
                            value: n.to_string(),
                            min,
                            max: field.max.unwrap_or(0),
                        });
                    }
                }

                if let (TomlValue::Integer(n), Some(max)) = (value, field.max) {
                    if *n > max {
                        errors.push(ConfigError::ValueOutOfRange {
                            key: key.clone(),
                            value: n.to_string(),
                            min: field.min.unwrap_or(0),
                            max,
                        });
                    }
                }
            }
        }
    }

    for (key, _) in &flat_config {
        let mut is_known = false;
        for (k, _) in schema {
            if k == key {
                is_known = true;
                break;
            }
        }
        if !is_known {
            let line = find_line_for_key(source, key);
            errors.push(ConfigError::UnknownKey {
                key: key.clone(),
                line,
            });
        }
    }

    if errors.is_empty() {
        Ok(())
    } else {
        Err(errors)
    }
}

fn find_line_for_key(source: &str, target_key: &str) -> usize {
    let short_key = if target_key.contains('.') {
        let mut parts = target_key.rsplit('.');
        match parts.next() {
            Some(k) => k,
            None => target_key,
        }
    } else {
        target_key
    };

    for (i, line) in source.lines().enumerate() {
        let trimmed = line.trim();
        if trimmed.starts_with('#') || trimmed.starts_with('[') || trimmed.is_empty() {
            continue;
        }
        if let Some(eq_pos) = trimmed.find('=') {
            let key_part = trimmed[..eq_pos].trim();
            if key_part == short_key {
                return i + 1;
            }
        }
    }
    0
}

fn format_error(err: &ConfigError, source: &str) -> String {
    let line_num = match err {
        ConfigError::UnexpectedCharacter { line, .. } => *line,
        ConfigError::UnterminatedString { line, .. } => *line,
        ConfigError::InvalidNumber { line, .. } => *line,
        ConfigError::ExpectedToken { line, .. } => *line,
        ConfigError::MissingValue { line, .. } => *line,
        ConfigError::SchemaViolation { line, .. } => *line,
        ConfigError::UnknownKey { line, .. } => *line,
        _ => 0,
    };

    if line_num == 0 {
        return format!("Error: {}", err);
    }

    let lines: Vec<&str> = source.lines().collect();
    let line_content = match lines.get(line_num - 1) {
        Some(l) => *l,
        None => "",
    };

    format!(
        "Error: {}\n   |\n{:>2} | {}\n   |",
        err, line_num, line_content
    )
}

fn run(config_path: &str, schema_path: &str) -> Result<(), Vec<ConfigError>> {
    let config_source = match fs::read_to_string(config_path) {
        Ok(s) => s,
        Err(e) => return Err(vec![ConfigError::from(e)]),
    };
    let schema_source = match fs::read_to_string(schema_path) {
        Ok(s) => s,
        Err(e) => return Err(vec![ConfigError::from(e)]),
    };

    let mut config_lexer = Lexer::new(&config_source);
    let config_tokens = match config_lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(e) => return Err(vec![e]),
    };
    let mut config_parser = Parser::new(config_tokens);
    let config_value = match config_parser.parse() {
        Ok(v) => v,
        Err(e) => return Err(vec![e]),
    };

    let mut schema_lexer = Lexer::new(&schema_source);
    let schema_tokens = match schema_lexer.tokenize() {
        Ok(tokens) => tokens,
        Err(e) => return Err(vec![e]),
    };
    let mut schema_parser = Parser::new(schema_tokens);
    let schema_value = match schema_parser.parse() {
        Ok(v) => v,
        Err(e) => return Err(vec![e]),
    };

    let schema = match parse_schema(&schema_value) {
        Ok(s) => s,
        Err(e) => return Err(vec![e]),
    };

    validate(&config_value, &schema, &config_source)?;

    println!("Config is valid!");
    println!();
    println!("Parsed config:");
    for (key, value) in flatten_table(&config_value, "") {
        println!("  {} = {}", key, display(&value));
    }

    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();

    let (config_path, schema_path) = if args.len() >= 3 {
        (args[1].clone(), args[2].clone())
    } else {
        (
            String::from("examples/valid_config.toml"),
            String::from("examples/schema.toml"),
        )
    };

    match run(&config_path, &schema_path) {
        Ok(()) => {}
        Err(errors) => {
            let source = match fs::read_to_string(&config_path) {
                Ok(s) => s,
                Err(_) => String::new(),
            };
            for err in &errors {
                eprintln!("{}", format_error(err, &source));
                eprintln!();
            }
            std::process::exit(1);
        }
    }
}
