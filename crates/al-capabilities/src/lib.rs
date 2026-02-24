//! AgentLang Capabilities — canonical capability registry, capability sets,
//! alias normalization, and delegation/access checking.
//!
//! Implements the **Canonical Capability Registry** from the AgentLang MVP
//! Profile v0.1 (`specs/MVP_PROFILE.md`, Section 5).
//!
//! # Design decisions
//!
//! * Capabilities are a closed enum — adding a new capability is a
//!   deliberate, versioned change.
//! * [`CapabilitySet`] is a thin wrapper around `HashSet<Capability>` so
//!   callers get convenient builder-style APIs while the representation
//!   stays cheap to clone and inspect.
//! * Deprecated aliases (Section 5, table) are normalized by
//!   [`normalize_alias`] which returns both the canonical capability and a
//!   human-readable deprecation warning suitable for diagnostic emission.
//! * Delegation follows MVP Section 8: callee runs under callee
//!   capabilities only — no implicit inheritance or intersection.

use std::collections::HashSet;
use std::fmt;

use al_diagnostics::{Diagnostic, ErrorCode, Span, WarningCode};
#[cfg(test)]
use al_diagnostics::Severity;
use serde::{Deserialize, Serialize};

// ===========================================================================
// Capability enum
// ===========================================================================

/// A single, canonical capability identifier as defined by the AgentLang MVP
/// Profile v0.1 (Section 5).
///
/// Every variant maps 1:1 to the identifiers listed in the spec.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub enum Capability {
    /// Read from a database.
    DbRead,
    /// Write to a database.
    DbWrite,
    /// Read from the file system.
    FileRead,
    /// Write to the file system.
    FileWrite,
    /// Make outbound API/HTTP calls.
    ApiCall,
    /// Define / expose API endpoints.
    ApiDefine,
    /// Publish messages to a queue.
    QueuePublish,
    /// Subscribe to a queue for messages.
    QueueSubscribe,
    /// Perform LLM inference.
    LlmInfer,
    /// Read from agent memory.
    MemoryRead,
    /// Write to agent memory.
    MemoryWrite,
    /// Register a tool in the tool registry.
    ToolRegister,
    /// Invoke a registered tool.
    ToolInvoke,
    /// Reflect on own schema / capabilities.
    Reflect,
    /// Schedule deferred or periodic work.
    Scheduler,
    /// Delegate execution to another agent.
    Delegate,
    /// Cryptographic signing.
    CryptoSign,
    /// Cryptographic encryption / decryption.
    CryptoEncrypt,
    /// Raw network (TCP/UDP/etc.) access.
    NetworkRaw,
    /// Spawn a new agent instance.
    AgentSpawn,
    /// Modify own operation / schema at runtime.
    SelfModify,
    /// Escalate to a human operator.
    EscalateHuman,
}

impl Capability {
    /// The canonical SCREAMING_SNAKE identifier as it appears in AgentLang
    /// source and the MVP Profile specification.
    pub fn canonical_name(&self) -> &'static str {
        match self {
            Capability::DbRead => "DB_READ",
            Capability::DbWrite => "DB_WRITE",
            Capability::FileRead => "FILE_READ",
            Capability::FileWrite => "FILE_WRITE",
            Capability::ApiCall => "API_CALL",
            Capability::ApiDefine => "API_DEFINE",
            Capability::QueuePublish => "QUEUE_PUBLISH",
            Capability::QueueSubscribe => "QUEUE_SUBSCRIBE",
            Capability::LlmInfer => "LLM_INFER",
            Capability::MemoryRead => "MEMORY_READ",
            Capability::MemoryWrite => "MEMORY_WRITE",
            Capability::ToolRegister => "TOOL_REGISTER",
            Capability::ToolInvoke => "TOOL_INVOKE",
            Capability::Reflect => "REFLECT",
            Capability::Scheduler => "SCHEDULER",
            Capability::Delegate => "DELEGATE",
            Capability::CryptoSign => "CRYPTO_SIGN",
            Capability::CryptoEncrypt => "CRYPTO_ENCRYPT",
            Capability::NetworkRaw => "NETWORK_RAW",
            Capability::AgentSpawn => "AGENT_SPAWN",
            Capability::SelfModify => "SELF_MODIFY",
            Capability::EscalateHuman => "ESCALATE_HUMAN",
        }
    }

    /// Parse a canonical SCREAMING_SNAKE name into a [`Capability`].
    ///
    /// Returns `None` if the string does not match any canonical name.
    pub fn from_canonical(name: &str) -> Option<Self> {
        match name {
            "DB_READ" => Some(Capability::DbRead),
            "DB_WRITE" => Some(Capability::DbWrite),
            "FILE_READ" => Some(Capability::FileRead),
            "FILE_WRITE" => Some(Capability::FileWrite),
            "API_CALL" => Some(Capability::ApiCall),
            "API_DEFINE" => Some(Capability::ApiDefine),
            "QUEUE_PUBLISH" => Some(Capability::QueuePublish),
            "QUEUE_SUBSCRIBE" => Some(Capability::QueueSubscribe),
            "LLM_INFER" => Some(Capability::LlmInfer),
            "MEMORY_READ" => Some(Capability::MemoryRead),
            "MEMORY_WRITE" => Some(Capability::MemoryWrite),
            "TOOL_REGISTER" => Some(Capability::ToolRegister),
            "TOOL_INVOKE" => Some(Capability::ToolInvoke),
            "REFLECT" => Some(Capability::Reflect),
            "SCHEDULER" => Some(Capability::Scheduler),
            "DELEGATE" => Some(Capability::Delegate),
            "CRYPTO_SIGN" => Some(Capability::CryptoSign),
            "CRYPTO_ENCRYPT" => Some(Capability::CryptoEncrypt),
            "NETWORK_RAW" => Some(Capability::NetworkRaw),
            "AGENT_SPAWN" => Some(Capability::AgentSpawn),
            "SELF_MODIFY" => Some(Capability::SelfModify),
            "ESCALATE_HUMAN" => Some(Capability::EscalateHuman),
            _ => None,
        }
    }

    /// Return a slice of every canonical capability in declaration order.
    pub fn all() -> &'static [Capability] {
        &[
            Capability::DbRead,
            Capability::DbWrite,
            Capability::FileRead,
            Capability::FileWrite,
            Capability::ApiCall,
            Capability::ApiDefine,
            Capability::QueuePublish,
            Capability::QueueSubscribe,
            Capability::LlmInfer,
            Capability::MemoryRead,
            Capability::MemoryWrite,
            Capability::ToolRegister,
            Capability::ToolInvoke,
            Capability::Reflect,
            Capability::Scheduler,
            Capability::Delegate,
            Capability::CryptoSign,
            Capability::CryptoEncrypt,
            Capability::NetworkRaw,
            Capability::AgentSpawn,
            Capability::SelfModify,
            Capability::EscalateHuman,
        ]
    }
}

impl fmt::Display for Capability {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.canonical_name())
    }
}

// ===========================================================================
// CapabilitySet
// ===========================================================================

/// An unordered set of capabilities granted to an agent.
///
/// Internally backed by a `HashSet<Capability>` for O(1) membership checks.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CapabilitySet {
    inner: HashSet<Capability>,
}

impl CapabilitySet {
    // -- constructors -------------------------------------------------------

    /// Create an empty capability set.
    pub fn empty() -> Self {
        Self {
            inner: HashSet::new(),
        }
    }

    /// Create a capability set containing all 22 canonical capabilities.
    pub fn all() -> Self {
        Self {
            inner: Capability::all().iter().copied().collect(),
        }
    }

    /// Create a capability set from an iterator of capabilities.
    pub fn from_iter(iter: impl IntoIterator<Item = Capability>) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }

    // -- builder helpers ----------------------------------------------------

    /// Insert a capability, returning `&mut self` for chaining.
    pub fn insert(&mut self, cap: Capability) -> &mut Self {
        self.inner.insert(cap);
        self
    }

    /// Remove a capability if present.
    pub fn remove(&mut self, cap: &Capability) -> bool {
        self.inner.remove(cap)
    }

    // -- queries ------------------------------------------------------------

    /// Returns `true` if the given capability is present.
    pub fn contains(&self, cap: &Capability) -> bool {
        self.inner.contains(cap)
    }

    /// Returns the number of capabilities in the set.
    pub fn len(&self) -> usize {
        self.inner.len()
    }

    /// Returns `true` if the set is empty.
    pub fn is_empty(&self) -> bool {
        self.inner.is_empty()
    }

    /// Returns an iterator over the capabilities in the set.
    pub fn iter(&self) -> impl Iterator<Item = &Capability> {
        self.inner.iter()
    }

    /// Returns `true` if `self` is a superset of `other`.
    pub fn is_superset_of(&self, other: &CapabilitySet) -> bool {
        self.inner.is_superset(&other.inner)
    }

    /// Returns `true` if `self` is a subset of `other`.
    pub fn is_subset_of(&self, other: &CapabilitySet) -> bool {
        self.inner.is_subset(&other.inner)
    }

    /// Return the set-intersection of two capability sets.
    pub fn intersection(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            inner: self.inner.intersection(&other.inner).copied().collect(),
        }
    }

    /// Return the set-union of two capability sets.
    pub fn union(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            inner: self.inner.union(&other.inner).copied().collect(),
        }
    }

    /// Return capabilities present in `self` but missing from `other`.
    pub fn difference(&self, other: &CapabilitySet) -> CapabilitySet {
        CapabilitySet {
            inner: self.inner.difference(&other.inner).copied().collect(),
        }
    }
}

impl Default for CapabilitySet {
    fn default() -> Self {
        Self::empty()
    }
}

impl fmt::Display for CapabilitySet {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        let mut names: Vec<&str> = self.inner.iter().map(|c| c.canonical_name()).collect();
        names.sort_unstable();
        write!(f, "{{{}}}", names.join(", "))
    }
}

impl std::iter::FromIterator<Capability> for CapabilitySet {
    fn from_iter<I: IntoIterator<Item = Capability>>(iter: I) -> Self {
        Self {
            inner: iter.into_iter().collect(),
        }
    }
}

impl IntoIterator for CapabilitySet {
    type Item = Capability;
    type IntoIter = std::collections::hash_set::IntoIter<Capability>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.into_iter()
    }
}

impl<'a> IntoIterator for &'a CapabilitySet {
    type Item = &'a Capability;
    type IntoIter = std::collections::hash_set::Iter<'a, Capability>;

    fn into_iter(self) -> Self::IntoIter {
        self.inner.iter()
    }
}

// ===========================================================================
// CapabilityError
// ===========================================================================

/// Errors that arise from capability checks and delegation validation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum CapabilityError {
    /// The required capability is not present in the agent's set.
    Missing {
        required: Capability,
        available: Vec<String>,
    },
    /// The caller tried to delegate but lacks the DELEGATE capability.
    DelegationNotPermitted,
    /// A deprecated alias was used — carries the canonical name and a
    /// deprecation message. (This is a soft error / warning; callers
    /// decide whether to promote it to a hard error.)
    DeprecatedAlias {
        alias: String,
        canonical: Capability,
        warning: String,
    },
    /// The capability name is completely unrecognized.
    Unknown { name: String },
}

impl fmt::Display for CapabilityError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CapabilityError::Missing {
                required,
                available,
            } => {
                write!(
                    f,
                    "capability `{}` is required but not granted; available capabilities: [{}]",
                    required,
                    available.join(", ")
                )
            }
            CapabilityError::DelegationNotPermitted => {
                write!(
                    f,
                    "caller does not hold the DELEGATE capability and cannot delegate execution"
                )
            }
            CapabilityError::DeprecatedAlias {
                alias,
                canonical,
                warning,
            } => {
                write!(
                    f,
                    "deprecated alias `{alias}` resolved to `{canonical}`: {warning}"
                )
            }
            CapabilityError::Unknown { name } => {
                write!(f, "unknown capability `{name}`")
            }
        }
    }
}

impl std::error::Error for CapabilityError {}

impl CapabilityError {
    /// Convert this error into an [`al_diagnostics::Diagnostic`].
    ///
    /// Uses `Span::dummy()` since capability errors typically arise from
    /// semantic analysis after initial parsing.  Callers that have a real
    /// source span should construct the diagnostic manually or patch the
    /// span on the returned value.
    pub fn to_diagnostic(&self) -> Diagnostic {
        match self {
            CapabilityError::Missing { required, .. } => Diagnostic::error(
                ErrorCode::CapabilityDenied,
                format!(
                    "missing required capability `{}`",
                    required.canonical_name()
                ),
                Span::dummy(),
            ),

            CapabilityError::DelegationNotPermitted => Diagnostic::error(
                ErrorCode::CapabilityDenied,
                "caller does not hold DELEGATE capability; delegation is not permitted",
                Span::dummy(),
            ),

            CapabilityError::DeprecatedAlias {
                alias,
                canonical,
                warning,
            } => Diagnostic::warning(
                WarningCode::CapAliasDeprecated,
                format!(
                    "deprecated alias `{alias}` used; use canonical `{canonical}` instead. {warning}"
                ),
                Span::dummy(),
            ),

            CapabilityError::Unknown { name } => Diagnostic::error(
                ErrorCode::CapabilityDenied,
                format!("unknown capability `{name}`"),
                Span::dummy(),
            ),
        }
    }

    /// Convert this error into a [`Diagnostic`] with an explicit source span.
    pub fn to_diagnostic_at(&self, span: Span) -> Diagnostic {
        let mut diag = self.to_diagnostic();
        diag.span = span;
        diag
    }
}

// ===========================================================================
// Alias normalization
// ===========================================================================

/// Deprecated alias table entry.
struct AliasEntry {
    /// The deprecated alias string (stored lowercase for case-insensitive matching).
    alias: &'static str,
    /// The canonical capability it maps to.
    canonical: Capability,
    /// A human-readable deprecation warning.
    warning: &'static str,
}

/// Complete table of deprecated aliases from MVP Profile v0.1, Section 5.
///
/// All alias strings are stored in lowercase; comparison is done against
/// `alias.trim().to_ascii_lowercase()`.
const DEPRECATED_ALIASES: &[AliasEntry] = &[
    AliasEntry {
        alias: "read capability",
        canonical: Capability::FileRead,
        warning: "Use `FILE_READ` instead of `read capability`.",
    },
    AliasEntry {
        alias: "write capability",
        canonical: Capability::FileWrite,
        warning: "Use `FILE_WRITE` instead of `write capability`.",
    },
    AliasEntry {
        alias: "network read capability",
        canonical: Capability::ApiCall,
        warning: "Use `API_CALL` instead of `network read capability`.",
    },
    AliasEntry {
        alias: "network write capability",
        canonical: Capability::ApiCall,
        warning: "Use `API_CALL` instead of `network write capability`.",
    },
    AliasEntry {
        alias: "net read capability",
        canonical: Capability::ApiCall,
        warning: "Use `API_CALL` instead of `net read capability`.",
    },
    AliasEntry {
        alias: "net write capability",
        canonical: Capability::ApiCall,
        warning: "Use `API_CALL` instead of `net write capability`.",
    },
    AliasEntry {
        alias: "llm capability",
        canonical: Capability::LlmInfer,
        warning: "Use `LLM_INFER` instead of `LLM capability`.",
    },
    AliasEntry {
        alias: "memory read capability",
        canonical: Capability::MemoryRead,
        warning: "Use `MEMORY_READ` instead of `memory read capability`.",
    },
    AliasEntry {
        alias: "memory write capability",
        canonical: Capability::MemoryWrite,
        warning: "Use `MEMORY_WRITE` instead of `memory write capability`.",
    },
    AliasEntry {
        alias: "register capability",
        canonical: Capability::ToolRegister,
        warning: "Use `TOOL_REGISTER` instead of `register capability`.",
    },
    AliasEntry {
        alias: "invoke capability",
        canonical: Capability::ToolInvoke,
        warning: "Use `TOOL_INVOKE` instead of `invoke capability`.",
    },
    AliasEntry {
        alias: "reflect capability",
        canonical: Capability::Reflect,
        warning: "Use `REFLECT` instead of `reflect capability`.",
    },
    AliasEntry {
        alias: "scheduler capability",
        canonical: Capability::Scheduler,
        warning: "Use `SCHEDULER` instead of `scheduler capability`.",
    },
    AliasEntry {
        alias: "api define capability",
        canonical: Capability::ApiDefine,
        warning: "Use `API_DEFINE` instead of `API define capability`.",
    },
    AliasEntry {
        alias: "publish capability",
        canonical: Capability::QueuePublish,
        warning: "Use `QUEUE_PUBLISH` instead of `publish capability`.",
    },
    AliasEntry {
        alias: "subscribe capability",
        canonical: Capability::QueueSubscribe,
        warning: "Use `QUEUE_SUBSCRIBE` instead of `subscribe capability`.",
    },
    AliasEntry {
        alias: "sign capability",
        canonical: Capability::CryptoSign,
        warning: "Use `CRYPTO_SIGN` instead of `sign capability`.",
    },
    AliasEntry {
        alias: "encrypt capability",
        canonical: Capability::CryptoEncrypt,
        warning: "Use `CRYPTO_ENCRYPT` instead of `encrypt capability`.",
    },
];

/// Attempt to normalize a deprecated capability alias to its canonical form.
///
/// Matching is **case-insensitive** and leading/trailing whitespace is
/// trimmed. If the alias is recognized, returns
/// `Some((canonical_capability, deprecation_warning))`. If the alias does
/// not match any deprecated form, returns `None`.
///
/// Per MVP Profile v0.1, Section 9: "Any use of non-canonical capability
/// names should emit a deprecation warning and normalize to canonical
/// capability IDs."
///
/// # Examples
///
/// ```
/// use al_capabilities::normalize_alias;
///
/// let result = normalize_alias("read capability");
/// assert!(result.is_some());
/// let (cap, warning) = result.unwrap();
/// assert_eq!(cap, al_capabilities::Capability::FileRead);
/// assert!(warning.contains("FILE_READ"));
/// ```
pub fn normalize_alias(alias: &str) -> Option<(Capability, &'static str)> {
    let lower = alias.trim().to_ascii_lowercase();
    DEPRECATED_ALIASES
        .iter()
        .find(|entry| entry.alias == lower)
        .map(|entry| (entry.canonical, entry.warning))
}

/// Resolve a capability name that may be either a canonical identifier or a
/// deprecated alias.
///
/// Returns:
/// - `Ok(cap)` when the name is a canonical identifier.
/// - `Err(CapabilityError::DeprecatedAlias { .. })` when the name matches a
///   deprecated alias. The error carries the canonical capability and a
///   deprecation warning. Callers should still use the canonical capability
///   but emit a diagnostic.
/// - `Err(CapabilityError::Unknown { .. })` when the name is not recognized
///   at all.
pub fn resolve_capability(name: &str) -> Result<Capability, CapabilityError> {
    // 1. Try canonical name first.
    if let Some(cap) = Capability::from_canonical(name) {
        return Ok(cap);
    }

    // 2. Try deprecated alias lookup.
    if let Some((cap, warning)) = normalize_alias(name) {
        return Err(CapabilityError::DeprecatedAlias {
            alias: name.to_string(),
            canonical: cap,
            warning: warning.to_string(),
        });
    }

    // 3. Unknown.
    Err(CapabilityError::Unknown {
        name: name.to_string(),
    })
}

// ===========================================================================
// Capability checking
// ===========================================================================

/// Check whether a capability set contains a required capability.
///
/// Returns `Ok(())` if `caps` contains `required`, otherwise returns a
/// [`CapabilityError::Missing`] with the required capability and a sorted
/// list of the capabilities that *are* available (for diagnostics).
///
/// # Examples
///
/// ```
/// use al_capabilities::{Capability, CapabilitySet, check_capability};
///
/// let mut caps = CapabilitySet::empty();
/// caps.insert(Capability::FileRead);
///
/// assert!(check_capability(&caps, Capability::FileRead).is_ok());
/// assert!(check_capability(&caps, Capability::FileWrite).is_err());
/// ```
pub fn check_capability(caps: &CapabilitySet, required: Capability) -> Result<(), CapabilityError> {
    if caps.contains(&required) {
        Ok(())
    } else {
        let mut available: Vec<String> =
            caps.iter().map(|c| c.canonical_name().to_string()).collect();
        available.sort_unstable();
        Err(CapabilityError::Missing {
            required,
            available,
        })
    }
}

// ===========================================================================
// Delegation checking
// ===========================================================================

/// Validate a delegation from caller to callee according to MVP Profile v0.1
/// Section 8.
///
/// **Rules enforced:**
///
/// 1. The **caller** must hold the `DELEGATE` capability.
/// 2. The callee runs under its *own* capability set — there is no implicit
///    capability inheritance or intersection override.
///
/// This function validates rule 1. Rule 2 is structural: callers should use
/// `callee_caps` (not `caller_caps`) when running the delegated agent. This
/// function does **not** merge, intersect, or otherwise combine the two sets.
///
/// # Parameters
///
/// * `caller_caps` — the capability set of the delegating agent.
/// * `_callee_caps` — the capability set of the agent being delegated to.
///   Accepted for documentation/future use but not currently inspected
///   beyond affirming the structural rule.
///
/// # Examples
///
/// ```
/// use al_capabilities::{Capability, CapabilitySet, check_delegation};
///
/// let mut caller = CapabilitySet::empty();
/// caller.insert(Capability::Delegate);
/// caller.insert(Capability::FileRead);
///
/// let mut callee = CapabilitySet::empty();
/// callee.insert(Capability::LlmInfer);
///
/// // Caller has DELEGATE — succeeds.
/// assert!(check_delegation(&caller, &callee).is_ok());
///
/// // Without DELEGATE — fails.
/// let no_delegate = CapabilitySet::empty();
/// assert!(check_delegation(&no_delegate, &callee).is_err());
/// ```
pub fn check_delegation(
    caller_caps: &CapabilitySet,
    _callee_caps: &CapabilitySet,
) -> Result<(), CapabilityError> {
    // Rule 1: caller must hold DELEGATE.
    if !caller_caps.contains(&Capability::Delegate) {
        return Err(CapabilityError::DelegationNotPermitted);
    }

    // Rule 2 (structural): callee runs with callee capabilities only.
    // This is enforced by the runtime using `_callee_caps` directly;
    // no merging or inheritance happens here.
    Ok(())
}

// ===========================================================================
// Tests
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    // -- Capability enum tests ----------------------------------------------

    #[test]
    fn all_capabilities_have_unique_canonical_names() {
        let all = Capability::all();
        let mut names: HashSet<&str> = HashSet::new();
        for cap in all {
            assert!(
                names.insert(cap.canonical_name()),
                "duplicate canonical name: {}",
                cap.canonical_name()
            );
        }
        assert_eq!(names.len(), 22, "expected exactly 22 canonical capabilities");
    }

    #[test]
    fn from_canonical_roundtrips() {
        for cap in Capability::all() {
            let name = cap.canonical_name();
            let parsed = Capability::from_canonical(name)
                .unwrap_or_else(|| panic!("from_canonical failed for `{name}`"));
            assert_eq!(*cap, parsed);
        }
    }

    #[test]
    fn from_canonical_unknown_returns_none() {
        assert_eq!(Capability::from_canonical("NOT_A_CAPABILITY"), None);
        assert_eq!(Capability::from_canonical(""), None);
        assert_eq!(Capability::from_canonical("db_read"), None); // case-sensitive
    }

    #[test]
    fn display_matches_canonical_name() {
        for cap in Capability::all() {
            assert_eq!(format!("{cap}"), cap.canonical_name());
        }
    }

    // -- CapabilitySet tests ------------------------------------------------

    #[test]
    fn empty_set_is_empty() {
        let set = CapabilitySet::empty();
        assert!(set.is_empty());
        assert_eq!(set.len(), 0);
    }

    #[test]
    fn all_set_has_22_capabilities() {
        let set = CapabilitySet::all();
        assert_eq!(set.len(), 22);
        for cap in Capability::all() {
            assert!(set.contains(cap));
        }
    }

    #[test]
    fn insert_and_contains() {
        let mut set = CapabilitySet::empty();
        assert!(!set.contains(&Capability::FileRead));
        set.insert(Capability::FileRead);
        assert!(set.contains(&Capability::FileRead));
        assert_eq!(set.len(), 1);
    }

    #[test]
    fn remove() {
        let mut set = CapabilitySet::empty();
        set.insert(Capability::FileRead);
        assert!(set.remove(&Capability::FileRead));
        assert!(!set.contains(&Capability::FileRead));
        assert!(!set.remove(&Capability::FileRead)); // already gone
    }

    #[test]
    fn superset_and_subset() {
        let full = CapabilitySet::all();
        let mut partial = CapabilitySet::empty();
        partial.insert(Capability::FileRead);
        partial.insert(Capability::FileWrite);

        assert!(full.is_superset_of(&partial));
        assert!(partial.is_subset_of(&full));
        assert!(!partial.is_superset_of(&full));
    }

    #[test]
    fn intersection_union_difference() {
        let a: CapabilitySet = [Capability::FileRead, Capability::ApiCall]
            .into_iter()
            .collect();
        let b: CapabilitySet = [Capability::ApiCall, Capability::LlmInfer]
            .into_iter()
            .collect();

        let inter = a.intersection(&b);
        assert_eq!(inter.len(), 1);
        assert!(inter.contains(&Capability::ApiCall));

        let uni = a.union(&b);
        assert_eq!(uni.len(), 3);

        let diff = a.difference(&b);
        assert_eq!(diff.len(), 1);
        assert!(diff.contains(&Capability::FileRead));
    }

    #[test]
    fn display_format_is_sorted() {
        let mut set = CapabilitySet::empty();
        set.insert(Capability::LlmInfer);
        set.insert(Capability::ApiCall);
        let display = format!("{set}");
        assert_eq!(display, "{API_CALL, LLM_INFER}");
    }

    #[test]
    fn from_iterator_trait() {
        let set: CapabilitySet = vec![Capability::DbRead, Capability::DbWrite]
            .into_iter()
            .collect();
        assert_eq!(set.len(), 2);
        assert!(set.contains(&Capability::DbRead));
        assert!(set.contains(&Capability::DbWrite));
    }

    #[test]
    fn into_iterator() {
        let set: CapabilitySet = vec![Capability::Reflect].into_iter().collect();
        let collected: Vec<Capability> = set.into_iter().collect();
        assert_eq!(collected, vec![Capability::Reflect]);
    }

    // -- Alias normalization tests ------------------------------------------

    #[test]
    fn normalize_known_aliases() {
        let cases: Vec<(&str, Capability)> = vec![
            ("read capability", Capability::FileRead),
            ("write capability", Capability::FileWrite),
            ("network read capability", Capability::ApiCall),
            ("network write capability", Capability::ApiCall),
            ("net read capability", Capability::ApiCall),
            ("net write capability", Capability::ApiCall),
            ("LLM capability", Capability::LlmInfer),
            ("memory read capability", Capability::MemoryRead),
            ("memory write capability", Capability::MemoryWrite),
            ("register capability", Capability::ToolRegister),
            ("invoke capability", Capability::ToolInvoke),
            ("reflect capability", Capability::Reflect),
            ("scheduler capability", Capability::Scheduler),
            ("API define capability", Capability::ApiDefine),
            ("publish capability", Capability::QueuePublish),
            ("subscribe capability", Capability::QueueSubscribe),
            ("sign capability", Capability::CryptoSign),
            ("encrypt capability", Capability::CryptoEncrypt),
        ];

        for (alias, expected) in cases {
            let result = normalize_alias(alias);
            assert!(
                result.is_some(),
                "expected alias `{alias}` to be recognized"
            );
            let (cap, warning) = result.unwrap();
            assert_eq!(
                cap, expected,
                "alias `{alias}` should map to `{expected}`, got `{cap}`"
            );
            assert!(
                !warning.is_empty(),
                "deprecation warning should not be empty for `{alias}`"
            );
            assert!(
                warning.contains(expected.canonical_name()),
                "warning for `{alias}` should mention canonical name `{}`",
                expected.canonical_name()
            );
        }
    }

    #[test]
    fn normalize_is_case_insensitive() {
        // "LLM capability" in mixed case
        assert!(normalize_alias("LLM capability").is_some());
        assert!(normalize_alias("llm capability").is_some());
        assert!(normalize_alias("Llm Capability").is_some());
    }

    #[test]
    fn normalize_trims_whitespace() {
        assert!(normalize_alias("  read capability  ").is_some());
    }

    #[test]
    fn normalize_unknown_returns_none() {
        assert!(normalize_alias("unknown capability").is_none());
        assert!(normalize_alias("").is_none());
        assert!(normalize_alias("FILE_READ").is_none()); // canonical, not an alias
    }

    // -- resolve_capability tests -------------------------------------------

    #[test]
    fn resolve_canonical_names() {
        for cap in Capability::all() {
            let result = resolve_capability(cap.canonical_name());
            assert!(result.is_ok(), "canonical `{}` should resolve", cap);
            assert_eq!(result.unwrap(), *cap);
        }
    }

    #[test]
    fn resolve_deprecated_alias_returns_error_with_canonical() {
        let result = resolve_capability("read capability");
        assert!(result.is_err());
        match result.unwrap_err() {
            CapabilityError::DeprecatedAlias {
                alias,
                canonical,
                warning,
            } => {
                assert_eq!(alias, "read capability");
                assert_eq!(canonical, Capability::FileRead);
                assert!(warning.contains("FILE_READ"));
            }
            other => panic!("expected DeprecatedAlias, got {other:?}"),
        }
    }

    #[test]
    fn resolve_unknown_returns_error() {
        let result = resolve_capability("DOES_NOT_EXIST");
        assert!(result.is_err());
        match result.unwrap_err() {
            CapabilityError::Unknown { name } => assert_eq!(name, "DOES_NOT_EXIST"),
            other => panic!("expected Unknown, got {other:?}"),
        }
    }

    // -- check_capability tests ---------------------------------------------

    #[test]
    fn check_capability_present() {
        let mut caps = CapabilitySet::empty();
        caps.insert(Capability::FileRead);
        assert!(check_capability(&caps, Capability::FileRead).is_ok());
    }

    #[test]
    fn check_capability_missing() {
        let caps = CapabilitySet::empty();
        let err = check_capability(&caps, Capability::FileRead).unwrap_err();
        match err {
            CapabilityError::Missing {
                required,
                available,
            } => {
                assert_eq!(required, Capability::FileRead);
                assert!(available.is_empty());
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    #[test]
    fn check_capability_missing_lists_available() {
        let mut caps = CapabilitySet::empty();
        caps.insert(Capability::ApiCall);
        caps.insert(Capability::LlmInfer);
        let err = check_capability(&caps, Capability::FileRead).unwrap_err();
        match err {
            CapabilityError::Missing { available, .. } => {
                assert_eq!(available.len(), 2);
                assert!(available.contains(&"API_CALL".to_string()));
                assert!(available.contains(&"LLM_INFER".to_string()));
            }
            other => panic!("expected Missing, got {other:?}"),
        }
    }

    // -- check_delegation tests ---------------------------------------------

    #[test]
    fn delegation_succeeds_when_caller_has_delegate() {
        let mut caller = CapabilitySet::empty();
        caller.insert(Capability::Delegate);

        let mut callee = CapabilitySet::empty();
        callee.insert(Capability::LlmInfer);

        assert!(check_delegation(&caller, &callee).is_ok());
    }

    #[test]
    fn delegation_fails_without_delegate() {
        let caller = CapabilitySet::empty(); // no DELEGATE
        let callee = CapabilitySet::empty();
        let err = check_delegation(&caller, &callee).unwrap_err();
        assert_eq!(err, CapabilityError::DelegationNotPermitted);
    }

    #[test]
    fn delegation_callee_caps_are_independent() {
        // Caller has DELEGATE + FILE_READ.
        let mut caller = CapabilitySet::empty();
        caller.insert(Capability::Delegate);
        caller.insert(Capability::FileRead);

        // Callee has LLM_INFER only (no FILE_READ).
        let mut callee = CapabilitySet::empty();
        callee.insert(Capability::LlmInfer);

        // Delegation itself succeeds.
        assert!(check_delegation(&caller, &callee).is_ok());

        // The callee should NOT inherit FILE_READ from caller.
        assert!(!callee.contains(&Capability::FileRead));

        // The callee should be checked against its own set only.
        assert!(check_capability(&callee, Capability::LlmInfer).is_ok());
        assert!(check_capability(&callee, Capability::FileRead).is_err());
    }

    // -- CapabilityError diagnostic conversion tests ------------------------

    #[test]
    fn error_to_diagnostic_missing() {
        let err = CapabilityError::Missing {
            required: Capability::FileRead,
            available: vec!["API_CALL".to_string()],
        };
        let diag = err.to_diagnostic();
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("FILE_READ"));
        assert_eq!(diag.profile, "mvp-0.1");
    }

    #[test]
    fn error_to_diagnostic_delegation() {
        let err = CapabilityError::DelegationNotPermitted;
        let diag = err.to_diagnostic();
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("DELEGATE"));
    }

    #[test]
    fn error_to_diagnostic_deprecated() {
        let err = CapabilityError::DeprecatedAlias {
            alias: "read capability".to_string(),
            canonical: Capability::FileRead,
            warning: "Use FILE_READ".to_string(),
        };
        let diag = err.to_diagnostic();
        assert_eq!(diag.severity, Severity::Warning);
        assert!(diag.message.contains("read capability"));
    }

    #[test]
    fn error_to_diagnostic_unknown() {
        let err = CapabilityError::Unknown {
            name: "BOGUS".to_string(),
        };
        let diag = err.to_diagnostic();
        assert_eq!(diag.severity, Severity::Error);
        assert!(diag.message.contains("BOGUS"));
    }

    #[test]
    fn error_to_diagnostic_at_span() {
        let err = CapabilityError::Missing {
            required: Capability::FileRead,
            available: vec![],
        };
        let span = Span::new(10, 2, 5, 9);
        let diag = err.to_diagnostic_at(span);
        assert_eq!(diag.span, span);
    }

    // -- Display / Error trait sanity checks ---------------------------------

    #[test]
    fn capability_error_display_does_not_panic() {
        let errors = vec![
            CapabilityError::Missing {
                required: Capability::FileRead,
                available: vec!["API_CALL".to_string()],
            },
            CapabilityError::DelegationNotPermitted,
            CapabilityError::DeprecatedAlias {
                alias: "read capability".to_string(),
                canonical: Capability::FileRead,
                warning: "Use FILE_READ".to_string(),
            },
            CapabilityError::Unknown {
                name: "BOGUS".to_string(),
            },
        ];
        for err in &errors {
            let _ = format!("{err}");
        }
    }

    #[test]
    fn default_capability_set_is_empty() {
        let set = CapabilitySet::default();
        assert!(set.is_empty());
    }
}
