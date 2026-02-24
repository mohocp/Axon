# AgentLang MVP Round 4 Summary (2026-02-24)

## Scope Implemented (VC pipeline core)

- Added VC generation from `REQUIRE`, `ENSURE`, and explicit `ASSERT` statements in `al-vc`.
- Added unique `vc_id` generation (`vc_000001`, `vc_000002`, ...).
- Replaced fixed Unknown solver with configurable stub solver:
  - `AlwaysValid`
  - `AlwaysInvalid { counterexample }`
  - `AlwaysUnknown { reason }` (default MVP mode)
- Implemented Unknown-result plumbing:
  - Produces synthetic runtime-assert rewrite metadata (`operation`, `vc_id`, `solver_reason`).
  - Injects synthetic `HirStatement::Assert` nodes into operation bodies (`meta.synthetic = true`).
- Implemented Invalid-result behavior:
  - Emits `VC_INVALID` diagnostics with VC id, operation name, and counterexample details.
- Wired VC pass into `al-types::TypeChecker` as Pass 8:
  - Generation -> solve -> apply results.
  - Stores `vc_results`, `synthetic_asserts`, and post-VC HIR for downstream/runtime plumbing.

## Tests Added/Extended

- `al-vc` unit tests:
  - VC generation from `REQUIRE`/`ENSURE`/`ASSERT` with unique IDs.
  - Configurable stub solver returns Valid/Invalid/Unknown.
  - Unknown => synthetic assert injection.
  - Invalid => `VC_INVALID` diagnostic emission.
- `al-types` unit tests:
  - Integrated VC pass generates expected VCs.
  - Unknown flow produces synthetic rewrites and synthetic HIR assert.
  - Invalid flow surfaces compile error (`VC_INVALID`).
- `al-conformance` integration tests:
  - VC generation count/ID coverage.
  - Unknown rewrite behavior coverage.
  - Invalid-to-diagnostic behavior coverage.

## MVP Boundaries Preserved

- No broad refactors outside VC-core slice.
- Capability/delegation behavior left unchanged.
- INVARIANT VC generation intentionally deferred (remaining Round 4 item).
