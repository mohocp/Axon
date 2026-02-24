//! AgentLang Standard Library MVP v0.1
//!
//! Defines the module registry and operation signatures for MVP-included modules.

use serde::{Deserialize, Serialize};

/// MVP module identifiers.
pub const MVP_MODULES: &[&str] = &[
    "core.data",
    "core.io",
    "core.text",
    "core.http",
    "agent.llm",
    "agent.memory",
];

/// Whether a module is included in MVP.
pub fn is_mvp_module(module: &str) -> bool {
    MVP_MODULES.contains(&module)
}

/// Excluded modules that must return NOT_IMPLEMENTED.
pub const EXCLUDED_MODULES: &[&str] = &[
    "core.math",
    "core.time",
    "core.crypto",
    "core.json",
    "db.sql",
    "db.vector",
    "db.graph",
    "api.rest",
    "api.grpc",
    "queue.pubsub",
    "agent.tools",
    "agent.planning",
    "agent.reflection",
];

/// Whether a module is explicitly excluded from MVP.
pub fn is_excluded_module(module: &str) -> bool {
    EXCLUDED_MODULES.contains(&module)
}

/// Operations included in each MVP module.
pub fn mvp_ops(module: &str) -> Option<&'static [&'static str]> {
    match module {
        "core.data" => Some(&["FILTER", "MAP", "REDUCE", "SORT", "GROUP", "TAKE", "SKIP"]),
        "core.io" => Some(&["READ", "WRITE", "FETCH"]),
        "core.text" => Some(&["PARSE", "FORMAT", "REGEX", "TOKENIZE"]),
        "core.http" => Some(&["GET", "POST"]),
        "agent.llm" => Some(&["GENERATE", "CLASSIFY", "EXTRACT"]),
        "agent.memory" => Some(&["REMEMBER", "RECALL", "FORGET"]),
        _ => None,
    }
}

/// Whether a specific operation is included in MVP for a given module.
pub fn is_mvp_op(module: &str, op: &str) -> bool {
    mvp_ops(module).map_or(false, |ops| ops.contains(&op))
}

/// Whether a module's operations are fallible (return Result[T]).
pub fn is_fallible_module(module: &str) -> bool {
    match module {
        "core.data" => false, // pure, returns bare T
        "core.io" | "core.text" | "core.http" | "agent.llm" | "agent.memory" => true,
        _ => true, // default to fallible for safety
    }
}

/// Excluded operations within otherwise-included modules.
pub fn excluded_ops_in_module(module: &str) -> &'static [&'static str] {
    match module {
        "core.http" => &["PUT", "DELETE"],
        "core.io" => &["STREAM"],
        _ => &[],
    }
}

// ---------------------------------------------------------------------------
// Signature types for STDLIB_MVP_SIGNATURES.json validation
// ---------------------------------------------------------------------------

/// A parameter in a stdlib operation signature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SigParam {
    pub name: String,
    #[serde(rename = "type")]
    pub type_name: String,
}

/// A single stdlib operation signature.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct OpSignature {
    pub module: String,
    pub inputs: Vec<SigParam>,
    pub output: String,
    pub fallible: bool,
    pub description: String,
}

/// The top-level signatures file structure.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct SignaturesFile {
    pub version: String,
    pub generated: String,
    pub description: String,
    pub operations: std::collections::HashMap<String, OpSignature>,
}

/// Names of stdlib data operations that have runtime implementations.
pub const IMPLEMENTED_STDLIB_OPS: &[&str] = &["FILTER", "MAP", "REDUCE"];

/// Load and parse the STDLIB_MVP_SIGNATURES.json from the embedded content.
pub fn load_signatures() -> SignaturesFile {
    let content = include_str!("../STDLIB_MVP_SIGNATURES.json");
    serde_json::from_str(content).expect("STDLIB_MVP_SIGNATURES.json must be valid JSON")
}

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mvp_modules_included() {
        assert!(is_mvp_module("core.data"));
        assert!(is_mvp_module("agent.llm"));
        assert!(!is_mvp_module("db.sql"));
    }

    #[test]
    fn excluded_modules() {
        assert!(is_excluded_module("db.sql"));
        assert!(is_excluded_module("core.math"));
        assert!(!is_excluded_module("core.data"));
    }

    #[test]
    fn mvp_ops_correct() {
        let ops = mvp_ops("core.data").unwrap();
        assert!(ops.contains(&"FILTER"));
        assert!(ops.contains(&"MAP"));
        assert_eq!(ops.len(), 7);
    }

    #[test]
    fn fallibility_correct() {
        assert!(!is_fallible_module("core.data"));
        assert!(is_fallible_module("core.io"));
        assert!(is_fallible_module("agent.llm"));
    }

    #[test]
    fn excluded_ops_in_included_modules() {
        let excluded = excluded_ops_in_module("core.http");
        assert!(excluded.contains(&"PUT"));
        assert!(excluded.contains(&"DELETE"));
    }

    // -----------------------------------------------------------------------
    // Signature lock tests (Round 6)
    // -----------------------------------------------------------------------

    #[test]
    fn signatures_file_parses() {
        let sigs = load_signatures();
        assert_eq!(sigs.version, "mvp-0.1");
    }

    #[test]
    fn signatures_lock_all_implemented_ops_present() {
        let sigs = load_signatures();
        for op_name in IMPLEMENTED_STDLIB_OPS {
            assert!(
                sigs.operations.contains_key(*op_name),
                "STDLIB_MVP_SIGNATURES.json missing implemented op: {}",
                op_name
            );
        }
    }

    #[test]
    fn signatures_lock_all_ops_in_core_data() {
        let sigs = load_signatures();
        for (name, sig) in &sigs.operations {
            assert_eq!(
                sig.module, "core.data",
                "op {} should be in core.data module",
                name
            );
        }
    }

    #[test]
    fn signatures_lock_core_data_ops_not_fallible() {
        let sigs = load_signatures();
        for (name, sig) in &sigs.operations {
            assert!(
                !sig.fallible,
                "core.data op {} should not be fallible",
                name
            );
        }
    }

    #[test]
    fn signatures_lock_filter_shape() {
        let sigs = load_signatures();
        let filter = sigs.operations.get("FILTER").expect("FILTER must exist");
        assert_eq!(filter.inputs.len(), 2);
        assert_eq!(filter.inputs[0].name, "list");
        assert_eq!(filter.inputs[1].name, "predicate");
        assert_eq!(filter.output, "List[T]");
    }

    #[test]
    fn signatures_lock_map_shape() {
        let sigs = load_signatures();
        let map = sigs.operations.get("MAP").expect("MAP must exist");
        assert_eq!(map.inputs.len(), 2);
        assert_eq!(map.inputs[0].name, "list");
        assert_eq!(map.inputs[1].name, "transform");
        assert_eq!(map.output, "List[U]");
    }

    #[test]
    fn signatures_lock_reduce_shape() {
        let sigs = load_signatures();
        let reduce = sigs.operations.get("REDUCE").expect("REDUCE must exist");
        assert_eq!(reduce.inputs.len(), 3);
        assert_eq!(reduce.inputs[0].name, "list");
        assert_eq!(reduce.inputs[1].name, "initial");
        assert_eq!(reduce.inputs[2].name, "reducer");
        assert_eq!(reduce.output, "U");
    }

    #[test]
    fn signatures_lock_consistency_with_registry() {
        let sigs = load_signatures();
        // Every implemented op must appear in both the signature file and the registry.
        let registry_ops = mvp_ops("core.data").unwrap();
        for op_name in IMPLEMENTED_STDLIB_OPS {
            assert!(
                registry_ops.contains(op_name),
                "implemented op {} not in core.data registry",
                op_name
            );
            assert!(
                sigs.operations.contains_key(*op_name),
                "implemented op {} not in signatures file",
                op_name
            );
        }
    }
}
