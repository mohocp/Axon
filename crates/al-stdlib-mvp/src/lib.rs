//! AgentLang Standard Library MVP v0.1
//!
//! Defines the module registry and operation signatures for MVP-included modules.

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
}
