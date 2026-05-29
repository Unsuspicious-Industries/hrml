---
name: unsuspicious-style
description: The house style for HRML/xrml code, docs, and framework design
metadata:
  type: feedback
---

Always write in the "Unsuspicious Industries" style: **formal methods with informal vibes.** Their ethos (unsuspicious.org) — λ-calculus, type theory, proof assistants pursued as art; "ethics and æsthetics are one"; elegance is inseparable from beauty; wry humour over stuffy academia.

**Why:** This is the team HRML is being built for; the framework is meant to become their go-to declarative web-dev tool. Code that doesn't read this way feels foreign to them.

**How to apply:**
- Doc comments frame constructs in their real CS lineage: W-types/sum-of-products (Pierce TAPL), capture-avoiding substitution, Kleisli composition, signature-functor coproducts (data types à la carte). See `src/template.rs` module header, `src/template/resolve.rs`, `src/template/head.rs`.
- Prefer hierarchical, composable components over flat `do_thing_like_this` methods. The user explicitly wants fewer underscored functions.
- Everything declarative; no imperative scripting in templates.
- Determinism is non-negotiable: output must be a byte-stable function of the tree (BTreeMap everywhere, verified by rebuilding `usi/` and diffing md5). See [[hrml-architecture]].
- Users should write the least, most meaningful code: global configs, tools that wrap verbose layout/styling boilerplate.
