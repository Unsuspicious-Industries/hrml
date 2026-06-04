use std::collections::BTreeMap;

use crate::template::{
    error::{TemplateError, TemplateErrorLocation, TemplateErrorPhase, TemplateResult},
    Node,
};

use super::ParseTree;

pub struct HParser;

impl ParseTree for HParser {
    fn parse(&self, source: &str, template_path: Option<&str>) -> TemplateResult<Vec<Node>> {
        let mut inner = InnerParser::new(source, template_path);
        inner.parse()
    }
}

const HTML_VOID_TAGS: &[&str] = &[
    "area", "base", "br", "col", "embed", "hr", "img", "input", "link", "meta", "source", "track",
    "wbr",
];

fn is_html_void(name: &str) -> bool {
    HTML_VOID_TAGS.iter().any(|v| v.eq_ignore_ascii_case(name))
}

pub(crate) const HTML_TAG_PREFIX: &str = "\u{1}html:";

fn html_name(name: &str) -> String {
    format!("{HTML_TAG_PREFIX}{name}")
}

struct InnerParser {
    chars: Vec<char>,
    pos: usize,
    template_path: Option<String>,
}

impl InnerParser {
    fn new(input: &str, template_path: Option<&str>) -> Self {
        Self {
            chars: input.chars().collect(),
            pos: 0,
            template_path: template_path.map(ToOwned::to_owned),
        }
    }

    fn parse(&mut self) -> TemplateResult<Vec<Node>> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 100_000 {
                let remaining: String = self.chars[self.pos..].iter().take(50).collect();
                return Err(self.internal_error(
                    self.pos,
                    format!(
                        "Parser infinite loop at pos {}, remaining='{}'",
                        self.pos, remaining
                    ),
                ));
            }
            if let Some(node) = self.parse_node(StopAt::Eof)? {
                nodes.push(node);
            } else {
                break;
            }
        }
        Ok(nodes)
    }

    fn parse_until(&mut self, closing_tag: &str) -> TemplateResult<Vec<Node>> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 100_000 {
                let remaining: String = self.chars[self.pos..].iter().take(50).collect();
                return Err(self.internal_error(
                    self.pos,
                    format!(
                        "Parser infinite loop in parse_until('{}') at pos {}, remaining='{}'",
                        closing_tag, self.pos, remaining
                    ),
                ));
            }
            if self.is_hrml_closing(closing_tag) {
                self.consume_hrml_closing();
                return Ok(nodes);
            }
            if let Some(node) = self.parse_node(StopAt::HrmlClose)? {
                nodes.push(node);
            } else {
                break;
            }
        }
        Err(self.code_error(
            self.pos,
            format!(
                "Unclosed HRML directive '{}': missing closing tag",
                closing_tag
            ),
        ))
    }

    fn parse_html_until(&mut self, tag: &str) -> TemplateResult<Vec<Node>> {
        let mut nodes = Vec::new();
        let mut iterations = 0;
        while self.pos < self.chars.len() {
            iterations += 1;
            if iterations > 100_000 {
                return Err(self.internal_error(
                    self.pos,
                    format!("Parser infinite loop in parse_html_until('{}')", tag),
                ));
            }
            if self.is_html_closing(tag) {
                self.consume_html_closing();
                return Ok(nodes);
            }
            if let Some(node) = self.parse_node(StopAt::HtmlClose)? {
                nodes.push(node);
            } else {
                break;
            }
        }
        Err(self.code_error(
            self.pos,
            format!("Unclosed HTML element '<{}>': missing '</{}>'", tag, tag),
        ))
    }

    fn is_hrml_closing(&self, name: &str) -> bool {
        let pattern1 = format!("</?{}", name);
        if self.starts_with(&pattern1) {
            let end = self.pos + pattern1.len();
            if end >= self.chars.len() {
                return true;
            }
            let next = self.chars[end];
            if next.is_whitespace() || next == '?' || next == '>' {
                return true;
            }
        }
        let pattern2 = format!("<?/{}?>", name);
        if self.starts_with(&pattern2) {
            return true;
        }
        false
    }

    fn consume_hrml_closing(&mut self) {
        while self.pos < self.chars.len() {
            if self.starts_with("?>") {
                self.pos += 2;
                return;
            }
            if self.chars[self.pos] == '>' {
                self.pos += 1;
                return;
            }
            self.pos += 1;
        }
    }

    fn is_html_closing(&self, name: &str) -> bool {
        let pat = format!("</{}", name);
        if !self.starts_with(&pat) {
            return false;
        }
        let after = self.pos + pat.len();
        match self.chars.get(after) {
            None => true,
            Some(c) if c.is_whitespace() || *c == '>' => true,
            _ => false,
        }
    }

    fn consume_html_closing(&mut self) {
        while self.pos < self.chars.len() {
            if self.chars[self.pos] == '>' {
                self.pos += 1;
                return;
            }
            self.pos += 1;
        }
    }

    fn starts_with(&self, s: &str) -> bool {
        let chars: Vec<char> = s.chars().collect();
        if self.pos + chars.len() > self.chars.len() {
            return false;
        }
        for (i, c) in chars.iter().enumerate() {
            if self.chars[self.pos + i] != *c {
                return false;
            }
        }
        true
    }

    fn lookahead(&self, offset: usize) -> Option<char> {
        self.chars.get(self.pos + offset).copied()
    }

    fn at_hrml_open(&self) -> bool {
        self.starts_with("<?") && !self.starts_with("</?") && !self.starts_with("<?/")
    }

    fn at_hrml_close(&self) -> bool {
        self.starts_with("</?") || self.starts_with("<?/")
    }

    fn at_html_open(&self) -> bool {
        if self.lookahead(0) != Some('<') {
            return false;
        }
        if self.at_hrml_open() || self.at_hrml_close() {
            return false;
        }
        if self.starts_with("<!--") || self.starts_with("<!") {
            return false;
        }
        matches!(self.lookahead(1), Some(c) if c.is_ascii_alphabetic())
    }

    fn at_html_close(&self) -> bool {
        if !self.starts_with("</") {
            return false;
        }
        if self.at_hrml_close() {
            return false;
        }
        matches!(self.lookahead(2), Some(c) if c.is_ascii_alphabetic())
    }

    fn at_html_comment(&self) -> bool {
        self.starts_with("<!--")
    }

    fn at_doctype(&self) -> bool {
        self.starts_with("<!") && !self.at_html_comment()
    }

    fn parse_node(&mut self, stop: StopAt) -> TemplateResult<Option<Node>> {
        if self.pos >= self.chars.len() {
            return Ok(None);
        }

        if self.at_hrml_open() {
            return self.parse_hrml_element().map(Some);
        }
        if self.at_hrml_close() {
            match stop {
                StopAt::Eof => {
                    return Err(self.code_error(
                        self.pos,
                        "Unexpected closing HRML tag without matching opener",
                    ));
                }
                _ => {
                    self.consume_hrml_closing();
                    return Ok(None);
                }
            }
        }
        if self.at_html_comment() {
            self.consume_html_comment();
            return self.parse_node(stop);
        }
        if self.at_doctype() {
            return Ok(Some(Node::Text(self.consume_doctype())));
        }
        if self.at_html_close() {
            match stop {
                StopAt::HtmlClose => return Ok(None),
                _ => {
                    let snippet: String = self.chars[self.pos..].iter().take(40).collect();
                    return Err(self.code_error(
                        self.pos,
                        format!(
                            "Unexpected HTML closing tag without matching opener near '{}'",
                            snippet
                        ),
                    ));
                }
            }
        }
        if self.at_html_open() {
            return self.parse_html_element().map(Some);
        }

        Ok(Some(Node::Text(self.consume_text())))
    }

    fn consume_text(&mut self) -> String {
        let mut text = String::new();
        while self.pos < self.chars.len() {
            if self.at_hrml_open()
                || self.at_hrml_close()
                || self.at_html_open()
                || self.at_html_close()
                || self.at_html_comment()
                || self.at_doctype()
            {
                break;
            }
            text.push(self.chars[self.pos]);
            self.pos += 1;
        }
        text
    }

    fn consume_html_comment(&mut self) {
        self.pos += 4; // skip <!--
        while self.pos + 2 < self.chars.len() {
            if self.chars[self.pos] == '-'
                && self.chars[self.pos + 1] == '-'
                && self.chars[self.pos + 2] == '>'
            {
                self.pos += 3;
                return;
            }
            self.pos += 1;
        }
        self.pos = self.chars.len();
    }

    fn consume_doctype(&mut self) -> String {
        let mut out = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            out.push(c);
            self.pos += 1;
            if c == '>' {
                break;
            }
        }
        out
    }

    fn parse_hrml_element(&mut self) -> TemplateResult<Node> {
        let start = self.pos;
        self.pos += 2;

        let name = self.consume_identifier();
        if name.is_empty() {
            let remaining: String = self.chars[self.pos..].iter().take(40).collect();
            return Err(self.internal_error(
                start,
                format!(
                    "BUG: parse_hrml_element got empty name at pos {}, remaining='{}'",
                    self.pos, remaining
                ),
            ));
        }
        let attrs = self.parse_hrml_attributes(&name)?;

        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }

        let explicit_self_closing = if self.starts_with("/?>") {
            self.pos += 3;
            true
        } else if self.starts_with("/>") {
            self.pos += 2;
            true
        } else if self.starts_with("?>") {
            self.pos += 2;
            false
        } else {
            return Err(self.code_error(
                start,
                format!("Unclosed HRML directive '<?{}': missing '?>'", name),
            ));
        };

        if explicit_self_closing {
            return Ok(Node::VoidElement { name, attrs });
        }

        let has_closing = self.has_matching_hrml_closing(&name);
        let is_void = !has_closing;

        if is_void {
            Ok(Node::VoidElement { name, attrs })
        } else {
            // `<?style?>` / `<?script?>` are raw-text directives: their bodies are
            // CSS/JS, which must not be parsed as markup (braces, `<`, `>` and the
            // like are not HRML). Everything up to the matching close is one text
            // node — mirroring how HTML treats <style>/<script>.
            let children = if name == "style" || name == "script" {
                self.parse_raw_until(&name)?
            } else {
                self.parse_until(&name)?
            };
            Ok(Node::Element {
                name,
                attrs,
                children,
            })
        }
    }

    /// Consume everything up to the matching close tag of `name` as a single
    /// verbatim text node (no inner parsing).
    fn parse_raw_until(&mut self, name: &str) -> TemplateResult<Vec<Node>> {
        let mut text = String::new();
        while self.pos < self.chars.len() {
            if self.is_hrml_closing(name) {
                self.consume_hrml_closing();
                return Ok(if text.is_empty() {
                    Vec::new()
                } else {
                    vec![Node::Text(text)]
                });
            }
            text.push(self.chars[self.pos]);
            self.pos += 1;
        }
        Err(self.code_error(
            self.pos,
            format!("Unclosed HRML directive '{}': missing closing tag", name),
        ))
    }

    fn parse_html_element(&mut self) -> TemplateResult<Node> {
        let start = self.pos;
        self.pos += 1; // skip <
        let name = self.consume_html_tag_name();
        if name.is_empty() {
            return Err(self.internal_error(start, "BUG: parse_html_element got empty name"));
        }
        let attrs = self.parse_html_attributes(&name)?;

        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }

        let explicit_self_closing = if self.starts_with("/>") {
            self.pos += 2;
            true
        } else if self.lookahead(0) == Some('>') {
            self.pos += 1;
            false
        } else {
            return Err(
                self.code_error(start, format!("Unclosed HTML tag '<{}': missing '>'", name))
            );
        };

        if explicit_self_closing || is_html_void(&name) {
            return Ok(Node::VoidElement {
                name: html_name(&name),
                attrs,
            });
        }

        let children = self.parse_html_until(&name)?;
        Ok(Node::Element {
            name: html_name(&name),
            attrs,
            children,
        })
    }

    fn consume_html_tag_name(&mut self) -> String {
        let mut id = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' {
                id.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        id
    }

    fn error_location(&self, pos: usize) -> TemplateErrorLocation {
        let mut line = 1;
        let mut column = 1;
        for ch in self.chars.iter().take(pos) {
            if *ch == '\n' {
                line += 1;
                column = 1;
            } else {
                column += 1;
            }
        }
        TemplateErrorLocation { line, column }
    }

    fn internal_error(&self, pos: usize, message: impl Into<String>) -> TemplateError {
        let location = self.error_location(pos);
        let mut error = TemplateError::internal(TemplateErrorPhase::Parse, message)
            .with_location(location.line, location.column);
        if let Some(path) = &self.template_path {
            error = error.with_template_path(path.clone());
        }
        error
    }

    fn code_error(&self, pos: usize, message: impl Into<String>) -> TemplateError {
        let location = self.error_location(pos);
        let mut error = TemplateError::code(TemplateErrorPhase::Parse, message)
            .with_location(location.line, location.column);
        if let Some(path) = &self.template_path {
            error = error.with_template_path(path.clone());
        }
        error
    }

    fn has_matching_hrml_closing(&self, name: &str) -> bool {
        let open_pfx = format!("<?{}", name);
        let close_pat1 = format!("</?{}", name);
        let close_pat2 = format!("<?/{}?>", name);

        let mut depth: i32 = 0;
        let mut i = self.pos;

        while i < self.chars.len() {
            if self.match_at(i, &close_pat1) || self.match_at(i, &close_pat2) {
                if depth == 0 {
                    return true;
                }
                depth -= 1;
                i += 1;
                continue;
            }
            if self.match_at(i, &open_pfx) {
                let after = i + open_pfx.len();
                let is_tag_start = self
                    .chars
                    .get(after)
                    .map_or(true, |&c| !c.is_alphanumeric());
                if is_tag_start && !self.is_self_closing_at(after) {
                    depth += 1;
                }
            }
            i += 1;
        }
        false
    }

    fn match_at(&self, pos: usize, pattern: &str) -> bool {
        let pat_chars: Vec<char> = pattern.chars().collect();
        if pos + pat_chars.len() > self.chars.len() {
            return false;
        }
        for (j, ch) in pat_chars.iter().enumerate() {
            if self.chars[pos + j] != *ch {
                return false;
            }
        }
        true
    }

    fn is_self_closing_at(&self, start: usize) -> bool {
        let mut j = start;
        while j + 1 < self.chars.len() {
            if self.chars[j] == '/' && self.chars[j + 1] == '?' {
                return true;
            }
            if self.chars[j] == '?' && self.chars[j + 1] == '>' {
                return false;
            }
            j += 1;
        }
        false
    }

    fn consume_identifier(&mut self) -> String {
        while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
            self.pos += 1;
        }
        let mut id = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_alphanumeric() || c == '_' || c == '-' || c == ':' || c == '.' {
                id.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        id
    }

    fn parse_hrml_attributes(&mut self, tag: &str) -> TemplateResult<BTreeMap<String, String>> {
        let mut attrs = BTreeMap::new();
        loop {
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }

            if self.pos >= self.chars.len()
                || self.starts_with("?>")
                || self.starts_with("/?>")
                || self.starts_with("/>")
            {
                break;
            }

            let key = self.consume_identifier();
            if key.is_empty() {
                if !self.starts_with("?>") {
                    self.pos += 1;
                }
                continue;
            }

            let mut value = String::new();
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }

            if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                self.pos += 1;
                while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                    self.pos += 1;
                }

                if self.pos < self.chars.len() {
                    let quote = self.chars[self.pos];
                    if quote == '"' || quote == '\'' {
                        let val_start = self.pos;
                        self.pos += 1;
                        while self.pos < self.chars.len() && self.chars[self.pos] != quote {
                            if self.chars[self.pos] == '<' && self.lookahead(1) == Some('?') {
                                return Err(self.code_error(
                                    val_start,
                                    format!(
                                        "Nested HRML directive inside attribute value of '<?{} {}=…?>' is not allowed — lift to $var form",
                                        tag, key
                                    ),
                                ));
                            }
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                        self.pos += 1;
                    } else {
                        while self.pos < self.chars.len()
                            && !self.chars[self.pos].is_whitespace()
                            && self.chars[self.pos] != '?'
                            && self.chars[self.pos] != '>'
                        {
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                    }
                }
            }

            attrs.insert(key, value);
        }
        Ok(attrs)
    }

    fn parse_html_attributes(&mut self, tag: &str) -> TemplateResult<BTreeMap<String, String>> {
        let mut attrs = BTreeMap::new();
        loop {
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }

            if self.pos >= self.chars.len()
                || self.lookahead(0) == Some('>')
                || self.starts_with("/>")
            {
                break;
            }

            let key = self.consume_html_attr_name();
            if key.is_empty() {
                // Skip stray characters defensively to avoid infinite loops.
                self.pos += 1;
                continue;
            }

            let mut value = String::new();
            while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                self.pos += 1;
            }

            if self.pos < self.chars.len() && self.chars[self.pos] == '=' {
                self.pos += 1;
                while self.pos < self.chars.len() && self.chars[self.pos].is_whitespace() {
                    self.pos += 1;
                }

                if self.pos < self.chars.len() {
                    let quote = self.chars[self.pos];
                    if quote == '"' || quote == '\'' {
                        let val_start = self.pos;
                        self.pos += 1;
                        while self.pos < self.chars.len() && self.chars[self.pos] != quote {
                            if self.chars[self.pos] == '<' && self.lookahead(1) == Some('?') {
                                return Err(self.code_error(
                                    val_start,
                                    format!(
                                        "Nested HRML directive inside attribute value of '<{} {}=…>' is not allowed — lift to $var form",
                                        tag, key
                                    ),
                                ));
                            }
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                        self.pos += 1;
                    } else {
                        while self.pos < self.chars.len()
                            && !self.chars[self.pos].is_whitespace()
                            && self.chars[self.pos] != '>'
                            && self.chars[self.pos] != '/'
                        {
                            value.push(self.chars[self.pos]);
                            self.pos += 1;
                        }
                    }
                }
            }

            attrs.insert(key, value);
        }
        Ok(attrs)
    }

    fn consume_html_attr_name(&mut self) -> String {
        let mut id = String::new();
        while self.pos < self.chars.len() {
            let c = self.chars[self.pos];
            if c.is_ascii_alphanumeric() || c == '-' || c == '_' || c == ':' || c == '.' || c == '@'
            {
                id.push(c);
                self.pos += 1;
            } else {
                break;
            }
        }
        id
    }
}

#[derive(Copy, Clone)]
enum StopAt {
    Eof,
    HrmlClose,
    HtmlClose,
}
