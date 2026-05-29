//! The predicate language of `<?if cond=…?>` — a small total decision procedure.
//!
//! A condition denotes a semantic predicate `eval : Cond → Context → Bool`
//! (Moggi-style: the context is the only effect). The surface syntax is tiny:
//!
//! ```text
//! Pred    ::= '!' Pred                 — negation
//!           | VRef '==' Operand         — equality
//!           | VRef '!=' Operand         — disequality
//!           | VRef                       — truthiness (non-empty)
//! VRef    ::= '$' Path | Path           — a context lookup
//! Operand ::= '$' Path | "'" … "'" | '"' … '"' | bare   — ref or literal
//! ```
//!
//! Parsing is a single left-to-right scan (a two-state automaton: outside vs.
//! inside an operator), and evaluation is structural recursion — so `eval`
//! is total and deterministic for every input string.

/// Resolves a context path (already stripped of a leading `$`) to its string
/// value, yielding the empty string when unbound.
pub type Lookup<'a> = dyn Fn(&str) -> String + 'a;

/// Decide a condition against the context. Unparseable or empty conditions are
/// false — the predicate is total, never an error.
pub fn eval(cond: &str, lookup: &Lookup) -> bool {
    Pred::parse(cond).eval(lookup)
}

enum Pred {
    /// Value is non-empty.
    Truthy(String),
    /// `left == right` (`equal = true`) or `left != right` (`equal = false`).
    Cmp { left: String, equal: bool, right: Operand },
    Not(Box<Pred>),
}

/// The right-hand side of a comparison: another context reference or a literal.
enum Operand {
    Ref(String),
    Literal(String),
}

impl Pred {
    fn parse(cond: &str) -> Pred {
        let cond = cond.trim();

        if let Some(inner) = cond.strip_prefix('!') {
            return Pred::Not(Box::new(Pred::parse(inner)));
        }

        for (op, equal) in [("==", true), ("!=", false)] {
            if let Some((l, r)) = cond.split_once(op) {
                return Pred::Cmp {
                    left: vref(l),
                    equal,
                    right: Operand::parse(r),
                };
            }
        }

        Pred::Truthy(vref(cond))
    }

    fn eval(&self, lookup: &Lookup) -> bool {
        match self {
            Pred::Truthy(path) => !lookup(path).is_empty(),
            Pred::Cmp { left, equal, right } => (lookup(left) == right.value(lookup)) == *equal,
            Pred::Not(inner) => !inner.eval(lookup),
        }
    }
}

impl Operand {
    fn parse(s: &str) -> Operand {
        let s = s.trim();
        if let Some(name) = s.strip_prefix('$') {
            Operand::Ref(name.to_string())
        } else {
            Operand::Literal(unquote(s).to_string())
        }
    }

    fn value(&self, lookup: &Lookup) -> String {
        match self {
            Operand::Ref(path) => lookup(path),
            Operand::Literal(lit) => lit.clone(),
        }
    }
}

/// A context reference: drop a leading `$`, keeping the dotted path.
fn vref(s: &str) -> String {
    s.trim().trim_start_matches('$').to_string()
}

/// Strip a single matching pair of surrounding quotes.
fn unquote(s: &str) -> &str {
    let bytes = s.as_bytes();
    if bytes.len() >= 2 {
        let (first, last) = (bytes[0], bytes[bytes.len() - 1]);
        if (first == b'\'' && last == b'\'') || (first == b'"' && last == b'"') {
            return &s[1..s.len() - 1];
        }
    }
    s
}

#[cfg(test)]
mod tests {
    use super::eval;
    use std::collections::BTreeMap;

    fn ctx(pairs: &[(&str, &str)]) -> impl Fn(&str) -> String {
        let map: BTreeMap<String, String> =
            pairs.iter().map(|(k, v)| (k.to_string(), v.to_string())).collect();
        move |path: &str| map.get(path).cloned().unwrap_or_default()
    }

    #[test]
    fn truthiness_is_nonemptiness() {
        let c = ctx(&[("label", "Hi"), ("empty", "")]);
        assert!(eval("$label", &c));
        assert!(!eval("$empty", &c));
        assert!(!eval("$missing", &c));
    }

    #[test]
    fn negation() {
        let c = ctx(&[("label", "Hi")]);
        assert!(!eval("!$label", &c)); // label present ⇒ !truthy is false
        assert!(eval("!$missing", &c)); // absent ⇒ !truthy is true
        assert!(eval("!!$label", &c)); // double negation
    }

    #[test]
    fn equality_against_literal() {
        let c = ctx(&[("icon", "book"), ("cols", "2")]);
        assert!(eval("$icon == 'book'", &c));
        assert!(eval("$icon == \"book\"", &c));
        assert!(!eval("$icon == 'code'", &c));
        assert!(eval("$cols == 2", &c)); // bare numeric literal
    }

    #[test]
    fn disequality_and_ref_rhs() {
        let c = ctx(&[("a", "x"), ("b", "y"), ("c", "x")]);
        assert!(eval("$a != 'y'", &c));
        assert!(!eval("$a != 'x'", &c));
        assert!(eval("$a == $c", &c)); // ref on the right
        assert!(eval("$a != $b", &c));
    }

    #[test]
    fn empty_condition_is_false() {
        let c = ctx(&[]);
        assert!(!eval("", &c));
        assert!(!eval("   ", &c));
    }
}
