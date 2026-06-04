//! Array transforms of the data pipeline: `<?filter?>`, `<?sort?>`, `<?slice?>`.
//!
//! Each is a pure endomorphism on lists, `[Value] → [Value]`, parameterised by
//! the directive's attributes. They share one Kleisli wrapper in the engine
//! (read `over`, write `as`); only the transform over the vector differs, so
//! the pipeline `data → sort → filter → slice → map` composes by function
//! composition. The equational laws of these operations (idempotent sort,
//! conjunctive filter) are the algebra of Plotkin–Power operations on `[Value]`.

use serde_json::Value;
use std::collections::BTreeMap;

/// Apply the named list transform, or `None` if `name` is not a pipeline op.
pub fn transform(
    name: &str,
    items: Vec<Value>,
    attrs: &BTreeMap<String, String>,
) -> Option<Vec<Value>> {
    match name {
        "filter" => Some(filter(items, attrs)),
        "sort" => Some(sort(items, attrs)),
        "slice" => Some(slice(items, attrs)),
        _ => None,
    }
}

/// Keep items whose `where` field is present and truthy.
fn filter(items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let where_key = attrs.get("where").map(String::as_str).unwrap_or("");
    items
        .into_iter()
        .filter(|item| item.get(where_key).is_some_and(is_truthy))
        .collect()
}

/// Stably order items by a named field; `order="desc"` reverses.
fn sort(mut items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let by = attrs.get("by").cloned().unwrap_or_default();
    let desc = attrs.get("order").is_some_and(|o| o == "desc");
    items.sort_by(|a, b| {
        let (ka, kb) = (sort_key(a.get(&by)), sort_key(b.get(&by)));
        if desc {
            kb.cmp(&ka)
        } else {
            ka.cmp(&kb)
        }
    });
    items
}

/// Take `count` items starting at `start` (aliases: `from`/`to`); `count=0`
/// means "to the end".
fn slice(items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let start = num(attrs, &["start", "from"]);
    let count = num(attrs, &["count", "to"]);
    let end = if count > 0 {
        (start + count).min(items.len())
    } else {
        items.len()
    };
    items
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Non-emptiness as the canonical notion of truth for a JSON value.
fn is_truthy(val: &Value) -> bool {
    match val {
        Value::Null => false,
        Value::Bool(b) => *b,
        Value::String(s) => !s.is_empty(),
        Value::Array(a) => !a.is_empty(),
        Value::Number(_) | Value::Object(_) => true,
    }
}

/// First parsed `usize` among the given attribute aliases, else 0.
fn num(attrs: &BTreeMap<String, String>, keys: &[&str]) -> usize {
    keys.iter()
        .find_map(|k| attrs.get(*k))
        .and_then(|v| v.parse().ok())
        .unwrap_or(0)
}

/// A total, order-preserving string key for a sortable field. Numbers are
/// zero-padded so they compare numerically under lexicographic `Ord`.
fn sort_key(val: Option<&Value>) -> String {
    match val {
        Some(Value::Number(n)) => format!("{:020}", n.as_u64().unwrap_or(0)),
        Some(Value::String(s)) => s.clone(),
        Some(Value::Bool(b)) => (if *b { "1" } else { "0" }).to_string(),
        _ => String::new(),
    }
}
