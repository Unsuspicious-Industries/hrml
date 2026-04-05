#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ComposeOp {
    Sum,
    Product,
}

impl ComposeOp {
    pub fn from_attr(value: &str) -> Self {
        match value.trim().to_ascii_lowercase().as_str() {
            "product" | "tensor" | "vertical" | "v" | "otimes" => Self::Product,
            _ => Self::Sum,
        }
    }
}

#[derive(Debug, Clone, Default)]
pub struct Hypertext {
    chunks: Vec<String>,
}

impl Hypertext {
    pub fn empty() -> Self {
        Self { chunks: Vec::new() }
    }

    pub fn pure(value: impl Into<String>) -> Self {
        Self {
            chunks: vec![value.into()],
        }
    }

    pub fn map(self, f: impl Fn(String) -> String) -> Self {
        let chunks = self.chunks.into_iter().map(f).collect();
        Self { chunks }
    }

    pub fn and_then(self, f: impl Fn(String) -> Hypertext) -> Self {
        let mut out = Vec::new();
        for chunk in self.chunks {
            out.extend(f(chunk).chunks);
        }
        Self { chunks: out }
    }

    pub fn combine(mut self, rhs: Hypertext) -> Self {
        self.chunks.extend(rhs.chunks);
        self
    }

    pub fn compose(self, rhs: Hypertext, op: ComposeOp) -> Self {
        match op {
            ComposeOp::Sum => self.combine(rhs),
            ComposeOp::Product => {
                let left = self.render();
                let right = rhs.render();
                Hypertext::pure(format!(
                    "<div class=\"hrml-product\"><div class=\"hrml-factor\">{}</div><div class=\"hrml-factor\">{}</div></div>",
                    left, right
                ))
            }
        }
    }

    pub fn render(self) -> String {
        self.chunks.join("")
    }
}
