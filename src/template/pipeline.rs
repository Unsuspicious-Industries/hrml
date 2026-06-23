//! Array transforms of the data pipeline: `<?filter?>`, `<?sort?>`,
//! `<?slice?>`, `<?tally?>`.
//!
//! Each is a pure endomorphism on lists, `[Value] → [Value]`, parameterised by
//! the directive's attributes. They share one Kleisli wrapper in the engine
//! (read `over`, write `as`); only the transform over the vector differs, so
//! the pipeline `data → sort → filter → slice → map` composes by function
//! composition. The equational laws of these operations (idempotent sort,
//! conjunctive filter) are the algebra of Plotkin–Power operations on `[Value]`.
//!
//! `tally` is the one non-endomorphism: it is the free commutative monoid
//! homomorphism `[Value] → Multiset(String)` induced by the `by` projection,
//! returned as the sorted list of `{name, depth, count}` records.

use super::{predicate, project, truthy_str};
use serde_json::{json, Value};
use std::collections::{BTreeMap, BTreeSet};

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
        "tally" => Some(tally(items, attrs)),
        _ => None,
    }
}

/// Keep items satisfying the `where` predicate — the same language as
/// `<?if cond?>` (`x`, `!x`, `x == 'lit'`, `x != y`), with paths resolved
/// against each item instead of the context.
fn filter(items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let cond = attrs.get("where").map(String::as_str).unwrap_or("");
    items
        .into_iter()
        .filter(|item| {
            predicate::eval(cond, &|path| {
                project(item, path).map(truthy_str).unwrap_or_default()
            })
        })
        .collect()
}

/// Stably order items by a (dotted) field path; `order="desc"` reverses.
fn sort(mut items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let by = attrs.get("by").cloned().unwrap_or_default();
    let desc = attrs.get("order").is_some_and(|o| o == "desc");
    items.sort_by(|a, b| {
        let (ka, kb) = (sort_key(project(a, &by)), sort_key(project(b, &by)));
        if desc {
            kb.cmp(&ka)
        } else {
            ka.cmp(&kb)
        }
    });
    items
}

/// Take a sub-range: `start` (alias `from`) is the first index, `to` an
/// exclusive end index, `count` a length. `count` wins over `to`; with
/// neither, the slice runs to the end.
fn slice(items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let start = num(attrs, &["start", "from"]).unwrap_or(0);
    let end = match (num(attrs, &["count"]), num(attrs, &["to"])) {
        (Some(count), _) => start + count,
        (None, Some(to)) => to,
        (None, None) => items.len(),
    };
    items
        .into_iter()
        .skip(start)
        .take(end.saturating_sub(start))
        .collect()
}

/// Count items per value of the `by` field, yielding `{name, depth, count}`
/// records sorted by name.
///
/// Array-valued fields contribute every element; with `sep`, a hierarchical
/// value also counts toward each of its prefixes (`a/b/c` → `a`, `a/b`,
/// `a/b/c`), and `depth` is the number of separators in the name. An item
/// counts at most once per name, however many of its values map to it.
fn tally(items: Vec<Value>, attrs: &BTreeMap<String, String>) -> Vec<Value> {
    let by = attrs.get("by").map(String::as_str).unwrap_or("");
    let sep = attrs
        .get("sep")
        .map(String::as_str)
        .filter(|s| !s.is_empty());

    let mut counts: BTreeMap<String, u64> = BTreeMap::new();
    for item in &items {
        let values: Vec<&str> = match item.get(by) {
            Some(Value::Array(a)) => a.iter().filter_map(Value::as_str).collect(),
            Some(Value::String(s)) => vec![s.as_str()],
            _ => Vec::new(),
        };
        let mut seen: BTreeSet<String> = BTreeSet::new();
        for value in values {
            match sep {
                Some(sep) => {
                    let mut prefix = String::new();
                    for part in value.split(sep) {
                        if !prefix.is_empty() {
                            prefix.push_str(sep);
                        }
                        prefix.push_str(part);
                        seen.insert(prefix.clone());
                    }
                }
                None => {
                    seen.insert(value.to_string());
                }
            }
        }
        for name in seen {
            *counts.entry(name).or_insert(0) += 1;
        }
    }

    counts
        .into_iter()
        .map(|(name, count)| {
            let depth = sep.map_or(0, |s| name.matches(s).count());
            json!({ "name": name, "depth": depth, "count": count })
        })
        .collect()
}

/// First parsed `usize` among the given attribute aliases.
fn num(attrs: &BTreeMap<String, String>, keys: &[&str]) -> Option<usize> {
    keys.iter()
        .find_map(|k| attrs.get(*k))
        .and_then(|v| v.parse().ok())
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

#[cfg(test)]
mod tests {
    use super::*;

    fn attrs(pairs: &[(&str, &str)]) -> BTreeMap<String, String> {
        pairs
            .iter()
            .map(|(k, v)| (k.to_string(), v.to_string()))
            .collect()
    }

    #[test]
    fn tally_counts_array_fields_once_per_item() {
        let items = vec![
            json!({ "subjects": ["logic", "cs"] }),
            json!({ "subjects": ["logic"] }),
            json!({ "subjects": [] }),
        ];
        let out = tally(items, &attrs(&[("by", "subjects")]));
        assert_eq!(
            out,
            vec![
                json!({ "name": "cs", "depth": 0, "count": 1 }),
                json!({ "name": "logic", "depth": 0, "count": 2 }),
            ]
        );
    }

    #[test]
    fn tally_expands_hierarchical_prefixes() {
        let items = vec![
            json!({ "subjects": ["philosophy/language"] }),
            json!({ "subjects": ["philosophy"] }),
        ];
        let out = tally(items, &attrs(&[("by", "subjects"), ("sep", "/")]));
        assert_eq!(
            out,
            vec![
                json!({ "name": "philosophy", "depth": 0, "count": 2 }),
                json!({ "name": "philosophy/language", "depth": 1, "count": 1 }),
            ]
        );
    }

    #[test]
    fn tally_accepts_scalar_fields_and_skips_missing() {
        let items = vec![
            json!({ "kind": "book" }),
            json!({ "kind": "book" }),
            json!({ "title": "no kind" }),
        ];
        let out = tally(items, &attrs(&[("by", "kind")]));
        assert_eq!(out, vec![json!({ "name": "book", "depth": 0, "count": 2 })]);
    }

    #[test]
    fn filter_speaks_the_if_predicate_language() {
        let items = vec![
            json!({ "status": "done", "remote": true }),
            json!({ "status": "todo", "remote": false }),
            json!({ "status": "todo" }),
        ];
        let eq = filter(items.clone(), &attrs(&[("where", "status == 'done'")]));
        assert_eq!(eq.len(), 1);
        let neq = filter(items.clone(), &attrs(&[("where", "status != 'done'")]));
        assert_eq!(neq.len(), 2);
        let not = filter(items.clone(), &attrs(&[("where", "!remote")]));
        assert_eq!(not.len(), 2); // false and missing are both falsy
        let truthy = filter(items, &attrs(&[("where", "remote")]));
        assert_eq!(truthy.len(), 1); // bare truthiness keeps working
    }

    #[test]
    fn sort_and_filter_project_dotted_paths() {
        let items = vec![
            json!({ "meta": { "rank": 2 } }),
            json!({ "meta": { "rank": 1 } }),
        ];
        let sorted = sort(items.clone(), &attrs(&[("by", "meta.rank")]));
        assert_eq!(sorted[0]["meta"]["rank"], 1);
        let kept = filter(items, &attrs(&[("where", "meta.rank == 1")]));
        assert_eq!(kept.len(), 1);
    }

    #[test]
    fn slice_to_is_an_end_index_and_count_a_length() {
        let items: Vec<Value> = (0..6).map(|n| json!(n)).collect();
        let by_to = slice(items.clone(), &attrs(&[("from", "2"), ("to", "5")]));
        assert_eq!(by_to, vec![json!(2), json!(3), json!(4)]);
        let by_count = slice(items.clone(), &attrs(&[("from", "2"), ("count", "2")]));
        assert_eq!(by_count, vec![json!(2), json!(3)]);
        let open_ended = slice(items, &attrs(&[("from", "4")]));
        assert_eq!(open_ended.len(), 2);
    }
}
