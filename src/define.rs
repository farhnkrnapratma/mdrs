use std::collections::BTreeMap;

use serde_json::{Map, Number, Value};

use crate::RenderError;

pub(crate) type Scope = BTreeMap<String, Value>;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum TokenKind {
    Number,
    String,
    Ident,
    True,
    False,
    Plus,
    Minus,
    Star,
    Slash,
    Percent,
    Eq,
    Neq,
    Gt,
    Lt,
    Gte,
    Lte,
    And,
    Or,
    Not,
    Question,
    Colon,
    Dot,
    Comma,
    LParen,
    RParen,
    LBracket,
    RBracket,
    LBrace,
    RBrace,
    Eof,
}

#[derive(Clone, Debug)]
struct Token {
    kind: TokenKind,
    text: String,
}

fn tokenize(source: &str) -> Result<Vec<Token>, String> {
    let chars: Vec<char> = source.chars().collect();
    let mut tokens = Vec::new();
    let mut i = 0;
    while i < chars.len() {
        let c = chars[i];
        if c.is_whitespace() {
            i += 1;
            continue;
        }
        if c == '/' && chars.get(i + 1) == Some(&'/') {
            while i < chars.len() && chars[i] != '\n' {
                i += 1;
            }
            continue;
        }
        if c.is_ascii_digit() || (c == '.' && chars.get(i + 1).is_some_and(char::is_ascii_digit)) {
            let start = i;
            i += 1;
            while chars.get(i).is_some_and(char::is_ascii_digit) {
                i += 1;
            }
            if chars.get(i) == Some(&'.') {
                i += 1;
                while chars.get(i).is_some_and(char::is_ascii_digit) {
                    i += 1;
                }
            }
            tokens.push(Token {
                kind: TokenKind::Number,
                text: chars[start..i].iter().collect(),
            });
            continue;
        }
        if c == '\'' || c == '"' {
            let quote = c;
            i += 1;
            let mut value = String::new();
            let mut closed = false;
            while i < chars.len() {
                if chars[i] == quote {
                    i += 1;
                    closed = true;
                    break;
                }
                if chars[i] == '\\' && i + 1 < chars.len() {
                    i += 1;
                    value.push(match chars[i] {
                        'n' => '\n',
                        'r' => '\r',
                        't' => '\t',
                        other => other,
                    });
                    i += 1;
                } else {
                    value.push(chars[i]);
                    i += 1;
                }
            }
            if !closed {
                return Err("unterminated string literal".to_owned());
            }
            tokens.push(Token {
                kind: TokenKind::String,
                text: value,
            });
            continue;
        }
        if c.is_ascii_alphabetic() || c == '_' {
            let start = i;
            i += 1;
            while chars
                .get(i)
                .is_some_and(|value| value.is_ascii_alphanumeric() || *value == '_')
            {
                i += 1;
            }
            let text: String = chars[start..i].iter().collect();
            let kind = match text.as_str() {
                "true" => TokenKind::True,
                "false" => TokenKind::False,
                _ => TokenKind::Ident,
            };
            tokens.push(Token { kind, text });
            continue;
        }
        let two = chars
            .get(i..i + 2)
            .unwrap_or(&[])
            .iter()
            .collect::<String>();
        let kind = match two.as_str() {
            "==" => Some(TokenKind::Eq),
            "!=" => Some(TokenKind::Neq),
            ">=" => Some(TokenKind::Gte),
            "<=" => Some(TokenKind::Lte),
            "&&" => Some(TokenKind::And),
            "||" => Some(TokenKind::Or),
            _ => None,
        };
        if let Some(kind) = kind {
            tokens.push(Token { kind, text: two });
            i += 2;
            continue;
        }
        let kind = match c {
            '+' => TokenKind::Plus,
            '-' => TokenKind::Minus,
            '*' => TokenKind::Star,
            '/' => TokenKind::Slash,
            '%' => TokenKind::Percent,
            '>' => TokenKind::Gt,
            '<' => TokenKind::Lt,
            '!' => TokenKind::Not,
            '?' => TokenKind::Question,
            ':' => TokenKind::Colon,
            '.' => TokenKind::Dot,
            ',' => TokenKind::Comma,
            '(' => TokenKind::LParen,
            ')' => TokenKind::RParen,
            '[' => TokenKind::LBracket,
            ']' => TokenKind::RBracket,
            '{' => TokenKind::LBrace,
            '}' => TokenKind::RBrace,
            _ => return Err(format!("unexpected character '{c}'")),
        };
        tokens.push(Token {
            kind,
            text: c.to_string(),
        });
        i += 1;
    }
    tokens.push(Token {
        kind: TokenKind::Eof,
        text: String::new(),
    });
    Ok(tokens)
}

struct Parser<'a> {
    tokens: Vec<Token>,
    position: usize,
    scope: &'a Scope,
    data: &'a Value,
    allow_this: bool,
}

impl<'a> Parser<'a> {
    fn new(
        source: &str,
        scope: &'a Scope,
        data: &'a Value,
        allow_this: bool,
    ) -> Result<Self, String> {
        Ok(Self {
            tokens: tokenize(source)?,
            position: 0,
            scope,
            data,
            allow_this,
        })
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.position]
    }

    fn take(&mut self, kind: TokenKind) -> bool {
        if self.peek().kind == kind {
            self.position += 1;
            true
        } else {
            false
        }
    }

    fn expect(&mut self, kind: TokenKind) -> Result<Token, String> {
        if self.peek().kind == kind {
            let token = self.peek().clone();
            self.position += 1;
            Ok(token)
        } else {
            Err(format!("expected {:?}, got '{}'", kind, self.peek().text))
        }
    }

    fn parse(mut self) -> Result<Value, String> {
        let value = self.ternary()?;
        if self.peek().kind != TokenKind::Eof {
            return Err(format!("unexpected token '{}'", self.peek().text));
        }
        Ok(value)
    }

    fn ternary(&mut self) -> Result<Value, String> {
        let test = self.or()?;
        if !self.take(TokenKind::Question) {
            return Ok(test);
        }
        let consequent = self.ternary()?;
        self.expect(TokenKind::Colon)?;
        let alternate = self.ternary()?;
        if test.truthy() {
            Ok(consequent)
        } else {
            Ok(alternate)
        }
    }

    fn or(&mut self) -> Result<Value, String> {
        let mut left = self.and()?;
        while self.take(TokenKind::Or) {
            let right = self.and()?;
            left = Value::Bool(left.truthy() || right.truthy());
        }
        Ok(left)
    }

    fn and(&mut self) -> Result<Value, String> {
        let mut left = self.equality()?;
        while self.take(TokenKind::And) {
            let right = self.equality()?;
            left = Value::Bool(left.truthy() && right.truthy());
        }
        Ok(left)
    }

    fn equality(&mut self) -> Result<Value, String> {
        let mut left = self.relational()?;
        loop {
            let equal = if self.take(TokenKind::Eq) {
                Some(true)
            } else if self.take(TokenKind::Neq) {
                Some(false)
            } else {
                None
            };
            let Some(equal) = equal else { break };
            let right = self.relational()?;
            left = Value::Bool(if equal { left == right } else { left != right });
        }
        Ok(left)
    }

    fn relational(&mut self) -> Result<Value, String> {
        let mut left = self.additive()?;
        loop {
            let compare: fn(f64, f64) -> bool = if self.take(TokenKind::Gt) {
                |a, b| a > b
            } else if self.take(TokenKind::Lt) {
                |a, b| a < b
            } else if self.take(TokenKind::Gte) {
                |a, b| a >= b
            } else if self.take(TokenKind::Lte) {
                |a, b| a <= b
            } else {
                break;
            };
            let right = self.additive()?;
            let a = left.number_value()?;
            let b = right.number_value()?;
            left = Value::Bool(compare(a, b));
        }
        Ok(left)
    }

    fn additive(&mut self) -> Result<Value, String> {
        let mut left = self.multiplicative()?;
        loop {
            let op = if self.take(TokenKind::Plus) {
                Some(TokenKind::Plus)
            } else if self.take(TokenKind::Minus) {
                Some(TokenKind::Minus)
            } else {
                None
            };
            let Some(op) = op else { break };
            let right = self.multiplicative()?;
            if op == TokenKind::Plus
                && (matches!(left, Value::String(_)) || matches!(right, Value::String(_)))
            {
                left = Value::String(format!("{}{}", display(&left), display(&right)));
            } else {
                let result = if op == TokenKind::Plus {
                    left.number_value()? + right.number_value()?
                } else {
                    left.number_value()? - right.number_value()?
                };
                left = number(result);
            }
        }
        Ok(left)
    }

    fn multiplicative(&mut self) -> Result<Value, String> {
        let mut left = self.unary()?;
        loop {
            let apply: fn(f64, f64) -> f64 = if self.take(TokenKind::Star) {
                |a, b| a * b
            } else if self.take(TokenKind::Slash) {
                |a, b| a / b
            } else if self.take(TokenKind::Percent) {
                |a, b| a % b
            } else {
                break;
            };
            let right = self.unary()?;
            let a = left.number_value()?;
            let b = right.number_value()?;
            left = number(apply(a, b));
        }
        Ok(left)
    }

    fn unary(&mut self) -> Result<Value, String> {
        if self.take(TokenKind::Not) {
            return Ok(Value::Bool(!self.unary()?.truthy()));
        }
        if self.take(TokenKind::Minus) {
            return Ok(number(-self.unary()?.number_value()?));
        }
        self.primary()
    }

    fn primary(&mut self) -> Result<Value, String> {
        let token = self.peek().clone();
        match token.kind {
            TokenKind::Number => {
                self.position += 1;
                Ok(token.text.parse::<f64>().map_or(Value::Null, number))
            }
            TokenKind::String => {
                self.position += 1;
                interpolate_string(&token.text, self.scope, self.data, self.allow_this)
            }
            TokenKind::True => {
                self.position += 1;
                Ok(Value::Bool(true))
            }
            TokenKind::False => {
                self.position += 1;
                Ok(Value::Bool(false))
            }
            TokenKind::Ident => self.reference(),
            TokenKind::LParen => {
                self.position += 1;
                let value = self.ternary()?;
                self.expect(TokenKind::RParen)?;
                Ok(value)
            }
            TokenKind::LBracket => self.array(),
            TokenKind::LBrace => self.object(),
            _ => Err(format!("unexpected token '{}'", token.text)),
        }
    }

    fn array(&mut self) -> Result<Value, String> {
        self.expect(TokenKind::LBracket)?;
        let mut values = Vec::new();
        while !self.take(TokenKind::RBracket) {
            values.push(self.ternary()?);
            if !self.take(TokenKind::Comma) {
                self.expect(TokenKind::RBracket)?;
                break;
            }
        }
        Ok(Value::Array(values))
    }

    fn object(&mut self) -> Result<Value, String> {
        self.expect(TokenKind::LBrace)?;
        let mut values = Map::new();
        while !self.take(TokenKind::RBrace) {
            let key = self.expect(TokenKind::Ident)?.text;
            self.expect(TokenKind::Colon)?;
            values.insert(key, self.ternary()?);
            if !self.take(TokenKind::Comma) {
                self.expect(TokenKind::RBrace)?;
                break;
            }
        }
        Ok(Value::Object(values))
    }

    fn reference(&mut self) -> Result<Value, String> {
        let root = self.expect(TokenKind::Ident)?.text;
        let mut value = if root == "this" {
            if !self.allow_this {
                return Err("'this' is only valid inside :::define".to_owned());
            }
            let name = self.expect(TokenKind::Dot)?;
            let _ = name;
            let property = self.expect(TokenKind::Ident)?.text;
            self.scope
                .get(&property)
                .cloned()
                .ok_or_else(|| format!("'{property}' is undefined"))?
        } else if let Some(value) = self.scope.get(&root) {
            value.clone()
        } else {
            walk_path(self.data, std::slice::from_ref(&root))?
        };

        loop {
            if self.take(TokenKind::Dot) {
                let property = self.expect(TokenKind::Ident)?.text;
                if property == "length"
                    && let Value::Array(values) = value
                {
                    value = number(values.len() as f64);
                    continue;
                }
                value = walk_property(&value, &property)?;
            } else if self.take(TokenKind::LBracket) {
                let index = self.ternary()?;
                self.expect(TokenKind::RBracket)?;
                value = walk_index(&value, &index)?;
            } else {
                break;
            }
        }
        Ok(value)
    }
}

fn number(value: f64) -> Value {
    Number::from_f64(value).map_or(Value::Null, Value::Number)
}

fn display(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Number(value) => {
            let number = value.as_f64().unwrap_or(f64::NAN);
            if number.fract() == 0.0 {
                format!("{}", number as i64)
            } else {
                number.to_string()
            }
        }
        Value::Bool(value) => value.to_string(),
        Value::Null => String::new(),
        Value::Array(_) | Value::Object(_) => value.to_string(),
    }
}

trait ValueExt {
    fn truthy(&self) -> bool;
    fn number_value(&self) -> Result<f64, String>;
}

impl ValueExt for Value {
    fn truthy(&self) -> bool {
        match self {
            Value::Bool(value) => *value,
            Value::Null => false,
            Value::Number(value) => value.as_f64().is_some_and(|value| value != 0.0),
            Value::String(value) => !value.is_empty(),
            Value::Array(value) => !value.is_empty(),
            Value::Object(value) => !value.is_empty(),
        }
    }

    fn number_value(&self) -> Result<f64, String> {
        match self {
            Value::Number(value) => Ok(value.as_f64().unwrap_or(f64::NAN)),
            _ => Err(format!("expected number, got {}", display(self))),
        }
    }
}

fn walk_property(value: &Value, property: &str) -> Result<Value, String> {
    value
        .as_object()
        .and_then(|object| object.get(property))
        .cloned()
        .ok_or_else(|| format!("property '{property}' does not exist"))
}

fn walk_index(value: &Value, index: &Value) -> Result<Value, String> {
    if let Some(index) = index.as_u64().or_else(|| {
        index
            .as_f64()
            .filter(|value| value.fract() == 0.0)
            .map(|value| value as u64)
    }) {
        return value
            .as_array()
            .and_then(|values| values.get(index as usize))
            .cloned()
            .ok_or_else(|| format!("index [{index}] is out of range"));
    }
    if let Some(key) = index.as_str() {
        return walk_property(value, key);
    }
    Err("array indexes must be numbers or strings".to_owned())
}

fn walk_path(data: &Value, path: &[String]) -> Result<Value, String> {
    let mut value = data.clone();
    for part in path {
        value = walk_property(&value, part)?;
    }
    Ok(value)
}

fn interpolate_string(
    source: &str,
    scope: &Scope,
    data: &Value,
    allow_this: bool,
) -> Result<Value, String> {
    if !source.contains('$') {
        return Ok(Value::String(source.to_owned()));
    }
    let mut output = String::new();
    let mut rest = source;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let after = &rest[start + 2..];
        let end = after
            .find('}')
            .ok_or_else(|| "unterminated interpolation in string".to_owned())?;
        let expression = &after[..end];
        output.push_str(&display(&eval(expression, scope, data, allow_this)?));
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    Ok(Value::String(output))
}

pub(crate) fn eval(
    expression: &str,
    scope: &Scope,
    data: &Value,
    allow_this: bool,
) -> Result<Value, String> {
    Parser::new(expression, scope, data, allow_this)?.parse()
}

pub(crate) fn value_display(value: &Value) -> String {
    display(value)
}

pub(crate) fn value_as_bool(value: &Value) -> bool {
    value.truthy()
}

pub(crate) fn resolve(source: &str, data: &Value) -> Result<String, RenderError> {
    resolve_with_scope(source, data).map(|(rendered, _)| rendered)
}

pub(crate) fn resolve_with_scope(
    source: &str,
    data: &Value,
) -> Result<(String, Scope), RenderError> {
    let mut definitions = Vec::<(String, String)>::new();
    let mut output = Vec::new();
    let mut in_define = false;
    let mut fence: Option<(char, usize)> = None;
    let mut dropping_leading_blank_lines = false;

    for line in source.lines() {
        if let Some((marker, minimum)) = fence {
            output.push(line.to_owned());
            if crate::markdown::fence_marker(line)
                .is_some_and(|(closing, count)| closing == marker && count >= minimum)
            {
                fence = None;
            }
            continue;
        }
        if let Some(opening) = crate::markdown::fence_marker(line) {
            fence = Some(opening);
            output.push(line.to_owned());
            continue;
        }
        if !in_define && line.trim() == ":::define" {
            in_define = true;
            dropping_leading_blank_lines = true;
            continue;
        }
        if in_define {
            if line.trim() == ":::" {
                in_define = false;
            } else if let Some((name, expression)) = line.split_once(':') {
                definitions.push((name.trim().to_owned(), expression.trim().to_owned()));
            } else if !line.trim().is_empty() {
                return Err(RenderError::UnsupportedDefineValue {
                    name: String::new(),
                    expression: line.trim().to_owned(),
                });
            }
            continue;
        }
        if dropping_leading_blank_lines && line.trim().is_empty() {
            continue;
        }
        dropping_leading_blank_lines = false;
        output.push(line.to_owned());
    }
    if in_define {
        return Err(RenderError::UnclosedDefineBlock);
    }

    let mut scope = Scope::new();
    let mut pending = definitions;
    while !pending.is_empty() {
        let mut next = Vec::new();
        let mut progressed = false;
        for (name, expression) in pending {
            match eval(&expression, &scope, data, true) {
                Ok(value) => {
                    crate::check_rendered_output_size(&value_display(&value))?;
                    scope.insert(name, value);
                    progressed = true;
                }
                Err(error) if error.contains("undefined") || error.contains("does not exist") => {
                    next.push((name, expression));
                }
                Err(_error) => {
                    return Err(RenderError::UnsupportedDefineValue { name, expression });
                }
            }
        }
        if !progressed {
            let (name, expression) = next.into_iter().next().unwrap_or_default();
            return Err(RenderError::UnsupportedDefineValue { name, expression });
        }
        pending = next;
    }

    let mut rendered = String::new();
    let mut in_fence: Option<(char, usize)> = None;
    for line in output {
        if let Some((marker, minimum)) = in_fence {
            rendered.push_str(&line);
            rendered.push('\n');
            if crate::markdown::fence_marker(&line)
                .is_some_and(|(closing, count)| closing == marker && count >= minimum)
            {
                in_fence = None;
            }
            continue;
        }
        if let Some(opening) = crate::markdown::fence_marker(&line) {
            in_fence = Some(opening);
            rendered.push_str(&line);
            rendered.push('\n');
            continue;
        }
        let line = interpolate_line(&line, &scope, data)?;
        crate::check_rendered_output_size(&line)?;
        rendered.push_str(&line);
        rendered.push('\n');
        crate::check_rendered_output_size(&rendered)?;
    }
    if rendered.ends_with('\n') {
        rendered.pop();
    }
    Ok((rendered, scope))
}

fn interpolate_line(line: &str, scope: &Scope, data: &Value) -> Result<String, RenderError> {
    let mut output = String::with_capacity(line.len());
    let mut rest = line;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        if start > 0 && rest.as_bytes()[start - 1] == b'\\' {
            output.push_str("${");
            rest = &rest[start + 2..];
            continue;
        }
        let after = &rest[start + 2..];
        let end = after
            .find('}')
            .ok_or_else(|| RenderError::UndefinedVariable(after.to_owned()))?;
        let expression = &after[..end];
        if expression.trim() == "else"
            || expression.trim() == "endif"
            || expression.trim_start().starts_with("if ")
        {
            output.push_str(&rest[start..start + 2 + end + 1]);
            rest = &after[end + 1..];
            continue;
        }
        let value = eval(expression, scope, data, false)
            .map_err(|error| RenderError::UndefinedVariable(format!("{expression}: {error}")))?;
        output.push_str(&value_display(&value));
        rest = &after[end + 1..];
    }
    output.push_str(rest);
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn data() -> Value {
        json!({
            "a": {"b": [10, 20, 30], "s": "text", "n": null, "z": 0, "e": "", "l": [], "o": {}},
            "f": 1.5,
            "n": null
        })
    }

    fn scope() -> Scope {
        let mut scope = Scope::new();
        scope.insert("x".to_owned(), json!(7));
        scope
    }

    fn run(expression: &str) -> Result<Value, String> {
        eval(expression, &scope(), &data(), false)
    }

    #[test]
    fn evaluates_operators_and_literals() {
        let cases = [
            ("1 + 2", json!(3.0)),
            ("'a' + 1", json!("a1")),
            ("1 + 'a'", json!("1a")),
            ("5 - 3", json!(2.0)),
            ("2 * 3", json!(6.0)),
            ("7 / 2", json!(3.5)),
            ("7 % 4", json!(3.0)),
            ("-3", json!(-3.0)),
            ("!0", json!(true)),
            ("!'x'", json!(false)),
            ("1 > 0", json!(true)),
            ("1 < 0", json!(false)),
            ("1 >= 1", json!(true)),
            ("1 <= 0", json!(false)),
            ("1 == 1", json!(true)),
            ("1 != 1", json!(false)),
            ("true && false", json!(false)),
            ("false || true", json!(true)),
            ("true ? 1 : 2", json!(1.0)),
            ("false ? 1 : 2", json!(2.0)),
            ("(1 + 2) * 3", json!(9.0)),
            ("[1, 2]", json!([1.0, 2.0])),
            ("[1, 2,]", json!([1.0, 2.0])),
            ("{a: 1, b: 'two'}", json!({"a": 1.0, "b": "two"})),
            ("{a: 1,}", json!({"a": 1.0})),
            (".5 + 1.", json!(1.5)),
            ("1 // note", json!(1.0)),
            ("'a\\nb\\rc\\td\\qe'", json!("a\nb\rc\tdqe")),
            ("\"dq\"", json!("dq")),
            ("'${1 + 1}!'", json!("2!")),
            ("0 / 0", Value::Null),
            ("x", json!(7)),
            ("a.b.length", json!(3.0)),
            ("a.b[0]", json!(10)),
            ("a['b'][1]", json!(20)),
            ("a.b[1.0]", json!(20)),
            ("a.s.length", Value::Null),
        ];
        for (expression, expected) in cases {
            let outcome = run(expression);
            if expression == "a.s.length" {
                assert!(outcome.is_err(), "{expression}");
            } else {
                assert_eq!(outcome, Ok(expected), "{expression}");
            }
        }
    }

    #[test]
    fn evaluates_truthiness_of_every_value_kind() {
        for (expression, expected) in [
            ("!n", true),
            ("!a.z", true),
            ("!a.e", true),
            ("!a.l", true),
            ("!a.o", true),
            ("!a.b", false),
            ("!a.s", false),
            ("!f", false),
        ] {
            assert_eq!(run(expression), Ok(json!(expected)), "{expression}");
        }
    }

    #[test]
    fn rejects_malformed_expressions() {
        for expression in [
            "'abc",
            "@",
            "1 +",
            "1 2",
            "this.x",
            "missing",
            "a.zzz",
            "a.b[9]",
            "a.b[true]",
            "1 > 'a'",
            "(1",
            "[1 2]",
            "{a 1}",
            "{1: 2}",
            "'${1'",
            "1 ? 2",
            "- 'a'",
            "7 * 'a'",
        ] {
            assert!(run(expression).is_err(), "{expression}");
        }
    }

    #[test]
    fn resolves_this_only_inside_definitions() {
        let outcome = eval("this.x + 1", &scope(), &data(), true);
        assert_eq!(outcome, Ok(json!(8.0)));
        assert!(eval("this", &scope(), &data(), true).is_err());
        assert!(eval("this.y", &scope(), &data(), true).is_err());
    }

    #[test]
    fn resolves_an_empty_source_to_nothing() {
        assert_eq!(resolve("", &json!({})).unwrap(), "");
    }

    #[test]
    fn displays_values_for_interpolation() {
        assert_eq!(value_display(&json!(2.0)), "2");
        assert_eq!(value_display(&json!(2.5)), "2.5");
        assert_eq!(value_display(&json!(true)), "true");
        assert_eq!(value_display(&Value::Null), "");
        assert_eq!(value_display(&json!([1, 2])), "[1,2]");
        assert_eq!(value_display(&json!("s")), "s");
        assert!(value_as_bool(&json!(1)));
    }

    #[test]
    fn resolves_definitions_in_dependency_order() {
        let source = ":::define\nb: a + 1\na: 1\n:::\n\nvalue ${b}\n```\n${raw}\n```\nescaped \\${b} and ${if b}${else}${endif}";
        let (rendered, scope) = resolve_with_scope(source, &json!({})).unwrap();
        assert_eq!(scope.get("b"), Some(&json!(2.0)));
        assert!(rendered.contains("value 2"));
        assert!(rendered.contains("${raw}"));
        assert!(rendered.contains("escaped \\${b}"));
        assert!(rendered.contains("${if b}${else}${endif}"));
        assert_eq!(resolve(source, &json!({})).unwrap(), rendered);
    }

    #[test]
    fn reports_definition_failures() {
        assert_eq!(
            resolve(":::define\na: 1", &json!({})),
            Err(RenderError::UnclosedDefineBlock)
        );
        assert_eq!(
            resolve(":::define\nnot a pair\n:::", &json!({})),
            Err(RenderError::UnsupportedDefineValue {
                name: String::new(),
                expression: "not a pair".to_owned()
            })
        );
        assert!(matches!(
            resolve(":::define\na: b\nb: a\n:::", &json!({})),
            Err(RenderError::UnsupportedDefineValue { .. })
        ));
        assert!(matches!(
            resolve(":::define\na: 1 +\n:::", &json!({})),
            Err(RenderError::UnsupportedDefineValue { name, .. }) if name == "a"
        ));
        assert!(matches!(
            resolve("${missing}", &json!({})),
            Err(RenderError::UndefinedVariable(_))
        ));
        assert!(matches!(
            resolve("${unterminated", &json!({})),
            Err(RenderError::UndefinedVariable(_))
        ));
    }

    #[test]
    fn keeps_blank_lines_and_fences_around_definitions() {
        let source =
            "~~~\n:::define\n~~~\n```` \n```\n````\n:::define\n\nname: 'x'\n\n:::\n\ntext ${name}";
        let rendered = resolve(source, &json!({})).unwrap();
        assert!(rendered.contains(":::define"));
        assert!(rendered.ends_with("text x"));
    }

    #[test]
    fn limits_the_size_of_a_single_definition() {
        let data = json!({"big": "x".repeat(crate::MAX_RENDERED_OUTPUT_LENGTH + 1)});
        assert!(matches!(
            resolve(":::define\nv: big\n:::\nx", &data),
            Err(RenderError::DocumentTooLarge { .. })
        ));
    }
}
