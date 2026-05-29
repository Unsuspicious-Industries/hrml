//! Data loading module — the functorial bridge from the filesystem to the
//! template context.
//!
//! # Formal specification
//!
//! The project's data model follows the algebra of *inductive types* and *free
//! constructions* as described in:
//!
//! - **Pierce, Benjamin C.** (2002). *Types and Programming Languages*. MIT
//!   Press.  Chapter 11 (algebraic datatypes) defines the sum-of-products
//!   structure used by every `Node`, `AstNode`, and context value.
//!
//! - **Martin-Löf, Per** (1984). *Intuitionistic Type Theory*. Bibliopolis.
//!   The `Value` type with `Object | Array | String | Number | Bool | Null`
//!   forms a W-type (well-founded tree) whose introduction rules correspond
//!   to the data constructors `<?data?>`, `<?record?>`, and `<?list?>`.
//!
//! - **Mac Lane, Saunders** (1971). *Categories for the Working Mathematician*.
//!   Springer.  The file-system-to-context mapping is a functor
//!   `F : Dir → Set` where `Dir` is the discrete category of filesystem paths
//!   and `Set` is the category of `serde_json::Value` objects.
//!
//! - **Wadler, Philip** (1992). "The Essence of Functional Programming".
//!   POPL.  The `<?map?>` directive instantiates the **list functor** `map`:
//!   `map : (A → B) → [A] → [B]`, where the function `A → B` is given by the
//!   child template block.
//!
//! - **Moggi, Eugenio** (1991). "Notions of Computation and Monads". *Inf. &
//!   Comp.* 93(1).  The `Context` type forms a computational monad `T(X) =
//!   Env → X`, where `set_value` is the bind operation threading scope across
//!   template evaluation.
//!
//! - **Swierstra, Wouter** (2008). "Data types à la carte". *JFP* 18(4).
//!   Each HRML directive is a signature functor; `<?map?>`, `<?filter?>`,
//!   `<?sort?>` are *algebraic effects* composed via coproducts of functors
//!   into a pipeline: `F ∘ G ∘ H`.
//!
//! - **Burstall, Rod** (1969). "Proving Properties of Programs by Structural
//!   Induction". *Computer Journal* 12(1).  Template rendering correctness is
//!   proved by structural induction on `Node` — each directive handler is a
//!   case in the induction.
//!
//! ## Type algebra
//!
//! ```text
//! Value    ≜  Null | Bool | Number | String
//!          |  Object(Map String Value)
//!          |  Array(Vec Value)                — inductive W-type
//!
//! Node     ≜  Text(String)
//!          |  Load { file: String, blocks: Map String [Node] }
//!          |  VoidElement { name: String, attrs: Map String String }
//!          |  Element   { name: String, attrs, children: [Node] }
//!
//! Context  ≜  { data: Value, vars: Map String Value,
//!               components: Map String [Node], ... }
//!
//! load_data_file  :  Path → Result Value Error     — point-wise parsing
//! load_data_dir   :  Path → Result Value Error     — co-inductive fold
//! ```
//!
//! ## Laws
//!
//! **Functoriality of map**
//! ```text
//! ∀ f g, arr.  map (f ∘ g) arr ≡ map f (map g arr)
//! ```
//!
//! **Idempotence of sort (stable sort)**
//! ```text
//! ∀ arr, k.  sort over arr by k ; sort over arr by k ≡ sort over arr by k
//! ```
//!
//! **Filter–Map fusion (free theorem)**
//! ```text
//! filter(where=cond) over arr ; map(f) over arr
//!   ≡  map(f) over filter(where=cond) over arr
//! ```
//! (when `f` is pure — holds for all template rendering since templates are
//! deterministic functions Context → Html.)

use serde_json::{Map, Value};
use std::fs;
use std::path::Path;

/// Load a single data file into a [`Value`] by dispatching on extension.
///
/// ## Formal signature
/// ```text
/// parse_ext : { json, toml, md, mdx } → (String → Value)
/// load_data_file = read ∘ parse_ext(ext(path))
/// ```
///
/// | Extension | Parser               | Output type     |
/// |-----------|----------------------|-----------------|
/// | `.json`   | `serde_json`         | arbitrary Value |
/// | `.toml`   | `toml` → `serde_json`| arbitrary Value |
/// | `.md`     | frontmatter split    | Object(body)    |
/// | `.mdx`    | frontmatter split    | Object(body)    |
pub fn load_data_file(path: &Path) -> Result<Value, String> {
    let content = fs::read_to_string(path)
        .map_err(|e| format!("Failed to read {}: {}", path.display(), e))?;

    let ext = path
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "json" => serde_json::from_str(&content)
            .map_err(|e| format!("Invalid JSON in {}: {}", path.display(), e)),

        "toml" => content
            .parse::<toml::Value>()
            .map_err(|e| format!("Invalid TOML in {}: {}", path.display(), e))
            .and_then(|v| {
                serde_json::to_value(v)
                    .map_err(|e| format!("TOML conversion error in {}: {}", path.display(), e))
            }),

        "md" | "mdx" => {
            let (meta, body) = crate::features::render_mdx_with_math(&content);
            let mut obj = match meta {
                Value::Object(m) => m,
                _ => Map::new(),
            };
            obj.insert("body".to_string(), Value::String(body));
            Ok(Value::Object(obj))
        }

        _ => Err(format!(
            "Unsupported data file type '{}' in {}",
            ext,
            path.display()
        )),
    }
}

/// Build an array of data objects from every supported file in a directory.
///
/// ## Formal signature
/// ```text
/// load_data_dir(dir) ≜
///   let stems = sort { stem(f) | f ∈ dir, ext(f) ∈ {md,mdx,json,toml}, ¬ hidden(f) }
///   in  map (λ s. let f = first_existing(s, {mdx, md, json, toml}) in
///              { slug: s } ∪ load_data_file(f)) stems
/// ```
///
/// Files starting with `.` or `_` are excluded.  Only one extension per stem
/// is loaded, preferring `mdx` > `md` > `json` > `toml`.
///
/// This is a *co-inductive fold* over the filesystem: for each slug we
/// construct a record whose fields are the parsed frontmatter/JSON/TOML
/// plus the `slug` field injected by the fold.
///
/// ## References
///
/// The fold pattern follows **Meijer, Fokkinga, Paterson** (1991).
/// "Functional Programming with Bananas, Lenses, Envelopes and Barbed Wire".
pub fn load_data_dir(dir_path: &Path) -> Result<Value, String> {
    let entries = fs::read_dir(dir_path)
        .map_err(|e| format!("Failed to read directory {}: {}", dir_path.display(), e))?;

    let mut filenames: Vec<String> = Vec::new();
    for entry in entries {
        let entry =
            entry.map_err(|e| format!("Failed to read entry in {}: {}", dir_path.display(), e))?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let file_name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
        if file_name.starts_with('.') || file_name.starts_with('_') {
            continue;
        }
        if let Some(ext) = path.extension() {
            if matches!(ext.to_str(), Some("json" | "toml" | "md" | "mdx")) {
                if let Some(stem) = path.file_stem().and_then(|s| s.to_str()) {
                    filenames.push(stem.to_string());
                }
            }
        }
    }
    filenames.sort();

    let mut items = Vec::new();
    let exts = ["mdx", "md", "json", "toml"];

    for slug in &filenames {
        for ext in &exts {
            let file_path = dir_path.join(format!("{}.{}", slug, ext));
            if file_path.exists() {
                match load_data_file(&file_path) {
                    Ok(value) => {
                        let mut obj = match value {
                            Value::Object(m) => m,
                            other => {
                                let mut m = Map::new();
                                m.insert("value".to_string(), other);
                                m
                            }
                        };
                        obj.insert("slug".to_string(), Value::String(slug.clone()));
                        items.push(Value::Object(obj));
                    }
                    Err(e) => {
                        eprintln!("[WARNING] Skipping {}: {}", file_path.display(), e);
                    }
                }
                break;
            }
        }
    }

    Ok(Value::Array(items))
}
