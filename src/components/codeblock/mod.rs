mod bash;
mod c;
mod lamina;
mod rust;

use bash::BashLexer;
use c::CLexer;
use lamina::LaminaLexer;
use rust::RustLexer;

#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    Keyword,
    String,
    Comment,
    Number,
    Operator,
    Identifier,
    Type,
    Macro,
    Lifetime,
    Whitespace,
    Newline,
    Other,
    Variable,
    Command,
    Shebang,
    Function,
}

#[derive(Debug, Clone)]
pub struct Token {
    pub token_type: TokenType,
    pub value: String,
}

pub trait LanguageLexer {
    fn lex(&self, input: &str) -> Vec<Token>;
    fn get_keywords(&self) -> &[&str];
}

pub struct BaseLexer {
    pub input: Vec<char>,
    pub position: usize,
    tokens: Vec<Token>,
}

impl BaseLexer {
    pub fn new(input: &str) -> Self {
        Self {
            input: input.chars().collect(),
            position: 0,
            tokens: Vec::new(),
        }
    }

    pub fn current_char(&self) -> Option<char> {
        self.input.get(self.position).copied()
    }

    pub fn peek_char(&self, offset: usize) -> Option<char> {
        self.input.get(self.position + offset).copied()
    }

    pub fn advance(&mut self) {
        self.position += 1;
    }

    pub fn add_token(&mut self, token_type: TokenType, value: String, _start: usize, _end: usize) {
        self.tokens.push(Token { token_type, value });
    }

    pub fn consume_whitespace(&mut self) {
        let start = self.position;
        let mut value = String::new();
        while let Some(c) = self.current_char() {
            if c == ' ' || c == '\t' {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }
        self.add_token(TokenType::Whitespace, value, start, self.position);
    }

    pub fn consume_newlines(&mut self) {
        let start = self.position;
        let mut value = String::new();
        while let Some(c) = self.current_char() {
            if c == '\n' || c == '\r' {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }
        self.add_token(TokenType::Newline, value, start, self.position);
    }

    pub fn consume_number(&mut self) {
        let start = self.position;
        let mut value = String::new();
        while let Some(c) = self.current_char() {
            if c.is_ascii_alphanumeric() || matches!(c, '.' | '_') {
                value.push(c);
                self.advance();
            } else {
                break;
            }
        }
        self.add_token(TokenType::Number, value, start, self.position);
    }

    pub fn consume_string(&mut self, quote: char) {
        let start = self.position;
        let mut value = String::new();
        while let Some(c) = self.current_char() {
            value.push(c);
            self.advance();
            if c == '\\' {
                if let Some(next) = self.current_char() {
                    value.push(next);
                    self.advance();
                }
            } else if c == quote && self.position > start + 1 {
                break;
            }
        }
        self.add_token(TokenType::String, value, start, self.position);
    }

    pub fn consume_single_line_comment(&mut self, marker: &str) {
        let start = self.position;
        let mut value = String::new();
        for _ in marker.chars() {
            if let Some(c) = self.current_char() {
                value.push(c);
                self.advance();
            }
        }
        while let Some(c) = self.current_char() {
            if c == '\n' || c == '\r' {
                break;
            }
            value.push(c);
            self.advance();
        }
        self.add_token(TokenType::Comment, value, start, self.position);
    }

    pub fn consume_multi_line_comment(&mut self, start_marker: &str, end_marker: &str) {
        let start = self.position;
        let mut value = String::new();
        while self.position < self.input.len() {
            if self.starts_with(end_marker) {
                for _ in end_marker.chars() {
                    if let Some(c) = self.current_char() {
                        value.push(c);
                        self.advance();
                    }
                }
                break;
            }
            if let Some(c) = self.current_char() {
                value.push(c);
                self.advance();
            }
        }
        if value.is_empty() {
            value.push_str(start_marker);
        }
        self.add_token(TokenType::Comment, value, start, self.position);
    }

    fn starts_with(&self, value: &str) -> bool {
        self.input[self.position..]
            .iter()
            .zip(value.chars())
            .all(|(left, right)| *left == right)
            && self.input.len() - self.position >= value.chars().count()
    }

    pub fn add_operator(&mut self, value: String) {
        let start = self.position;
        self.advance();
        self.add_token(TokenType::Operator, value, start, self.position);
    }

    pub fn add_other(&mut self, value: String) {
        let start = self.position;
        self.advance();
        self.add_token(TokenType::Other, value, start, self.position);
    }

    pub fn get_tokens(self) -> Vec<Token> {
        self.tokens
    }
}

pub fn highlight_html_code_blocks(html: &str) -> String {
    let mut result = String::with_capacity(html.len());
    let mut rest = html;
    const OPEN: &str = "<pre><code class=\"language-";
    const CLOSE: &str = "</code></pre>";

    while let Some(block_offset) = rest.find(OPEN) {
        result.push_str(&rest[..block_offset]);
        let block = &rest[block_offset..];
        let Some(language_end) = block[OPEN.len()..].find("\">") else {
            result.push_str(block);
            return result;
        };
        let language = &block[OPEN.len()..OPEN.len() + language_end];
        let content_start = OPEN.len() + language_end + 2;
        let Some(content_end) = block[content_start..].find(CLOSE) else {
            result.push_str(block);
            return result;
        };
        let content = &block[content_start..content_start + content_end];
        let highlighted = highlight(language, content);
        result.push_str(&format!(
            "<pre data-language=\"{}\" class=\"code-block-container\"><code class=\"language-{}\">{}</code></pre>",
            escape_html(language),
            escape_html(language),
            highlighted
        ));
        rest = &block[content_start + content_end + CLOSE.len()..];
    }

    result.push_str(rest);
    result
}

fn highlight(language: &str, content: &str) -> String {
    let decoded = decode_html(content);
    let lexer: Box<dyn LanguageLexer> = match language {
        "bash" | "shell" | "sh" => Box::new(BashLexer),
        "c" => Box::new(CLexer),
        "lamina" | "lamina-ir" => Box::new(LaminaLexer),
        "rust" | "rs" => Box::new(RustLexer),
        _ => return escape_html(&decoded),
    };

    lexer
        .lex(&decoded)
        .into_iter()
        .map(|token| render_token(language, token))
        .collect()
}

fn render_token(language: &str, token: Token) -> String {
    let class = match token.token_type {
        TokenType::Whitespace | TokenType::Newline | TokenType::Other => None,
        TokenType::Keyword if language == "lamina" && is_lamina_type(&token.value) => {
            Some("syntax-type")
        }
        TokenType::Keyword => Some("syntax-keyword"),
        TokenType::String => Some("syntax-string"),
        TokenType::Comment => Some("syntax-comment"),
        TokenType::Number => Some("syntax-number"),
        TokenType::Operator => Some("syntax-operator"),
        TokenType::Identifier if language == "lamina" && token.value.starts_with('@') => {
            Some("syntax-symbol")
        }
        TokenType::Identifier if language == "lamina" && is_lamina_instruction(&token.value) => {
            Some("syntax-keyword")
        }
        TokenType::Identifier if language == "c" && token.value.ends_with("_t") => {
            Some("syntax-type")
        }
        TokenType::Identifier if token.value.starts_with('-') => Some("syntax-flag"),
        TokenType::Identifier => Some("syntax-identifier"),
        TokenType::Type => Some("syntax-type"),
        TokenType::Macro => Some("syntax-macro"),
        TokenType::Lifetime => Some("syntax-lifetime"),
        TokenType::Variable if language == "shell" && token.value == "$" => Some("syntax-prompt"),
        TokenType::Variable => Some("syntax-variable"),
        TokenType::Command if token.value.starts_with('-') => Some("syntax-flag"),
        TokenType::Command => Some("syntax-command"),
        TokenType::Shebang => Some("syntax-comment"),
        TokenType::Function => Some("syntax-function"),
    };
    let escaped = escape_html(&token.value);
    match class {
        Some(class) => format!("<span class=\"{class}\">{escaped}</span>"),
        None => escaped,
    }
}

fn is_lamina_type(value: &str) -> bool {
    matches!(
        value,
        "i8" | "i16"
            | "i32"
            | "i64"
            | "i128"
            | "u8"
            | "u16"
            | "u32"
            | "u64"
            | "u128"
            | "f32"
            | "f64"
            | "bool"
            | "ptr"
    )
}

fn is_lamina_instruction(value: &str) -> bool {
    matches!(
        value.split('.').next().unwrap_or(value),
        "add"
            | "alloc"
            | "br"
            | "call"
            | "dealloc"
            | "div"
            | "eq"
            | "ge"
            | "getelementptr"
            | "getfieldptr"
            | "gt"
            | "jmp"
            | "le"
            | "load"
            | "lt"
            | "mod"
            | "mul"
            | "ne"
            | "phi"
            | "print"
            | "read"
            | "readbyte"
            | "ret"
            | "shl"
            | "shr"
            | "store"
            | "sub"
            | "switch"
            | "write"
            | "writebyte"
            | "writeptr"
    )
}

fn decode_html(value: &str) -> String {
    value
        .replace("&lt;", "<")
        .replace("&gt;", ">")
        .replace("&quot;", "\"")
        .replace("&#39;", "'")
        .replace("&amp;", "&")
}

fn escape_html(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
        .replace('\'', "&#39;")
}

#[cfg(test)]
mod tests {
    use super::highlight_html_code_blocks;

    #[test]
    fn highlights_lamina_blocks_as_static_html() {
        let html = r#"<pre><code class="language-lamina">fn @main() -&gt; i64 {
  ret.i64 42
}</code></pre>"#;
        let highlighted = highlight_html_code_blocks(html);
        assert!(highlighted.contains(r#"class="syntax-keyword">fn</span>"#));
        assert!(highlighted.contains(r#"class="syntax-symbol">@main</span>"#));
        assert!(highlighted.contains(r#"class="syntax-keyword">ret.i64</span>"#));
        assert!(highlighted.contains(r#"class="syntax-number">42</span>"#));
        assert!(!highlighted.contains("<script"));
    }
}
