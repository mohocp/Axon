# Review: `formal_semantics.md` from Implementation Feasibility and API Usability Perspective

## Summary of Feasibility Findings

The specification is conceptually strong and mostly coherent as a *meta-semantics*, but it is not yet executable as a production-grade checker/runtime contract without additional normalization and algorithmic constraints.

Key conclusion:
- The core typing and reduction rules are implementable.
- The current draft mixes declarative rules with underspecified side conditions (`Pre/Post`, solver validity, policy predicates, scheduler/resource model), which blocks deterministic compiler/runtime behavior.
- The memory and concurrency model captures the intended architecture (CAS + mutable cells + DAG + locks), but several axioms idealize away real-world failure modes that storage engines and distributed runtimes must handle explicitly.
- Complexity guarantees are currently descriptive rather than enforceable because cost models and parameter bounds are not normatively attached to operations.

Net: feasible with moderate redesign of formal obligations into executable algorithms, plus explicit operational contracts for failure, distribution, and solver fallbacks.

## Concerns (with Severity)

### Critical

1. **Type checker is not directly executable from current judgments**
- `Γ;Ψ;C ⊢ e : τ ▷ Ω` leaves `Ψ ⊨ Pre`, `Post` accumulation, and VC entailment semantics underdefined (solver theory, timeout policy, unknown handling are only partially described).
- Without a canonical obligation language and decision procedure contract, independent implementations will diverge.

2. **Operator contracts (`Op(..., Pre, Post, Caps)`) are not operationally grounded**
- `Pre` and `Post` are treated as logical formulas but there is no required representation, variable binding discipline, or substitution semantics for side-effectful ops.
- This prevents sound code generation for API boundaries and inhibits automated contract checking.

3. **Memory axiom A1 assumes collision-free digest equivalence**
- `digest(v)=digest(v') ⇔ canonical(v)=canonical(v')` is mathematically convenient but operationally false for real hashes.
- Production storage requires an explicit collision strategy (secondary hash, byte-compare, object IDs, or collision fault semantics).

4. **`resume`/checkpoint semantics are underspecified for mutable and external state**
- `snapshot(ρ,Σ,Q)` and `restore(cp)` do not define what happens to in-flight tasks, lock ownership, external side effects, or concurrent writes.
- This is a major correctness and API contract risk for users expecting deterministic recovery.

5. **Concurrency semantics are too abstract for scheduler interoperability**
- `resources available`, join policies (`BEST_EFFORT`, `PARTIAL`) and timeout/failure propagation lack normative state transitions.
- Implementers cannot produce equivalent behavior across runtimes, especially for partial results and cancellation.

### Major

6. **Subtyping + refinement + unions likely cause high checker complexity without normalization rules**
- Declarative least-closure definition is fine mathematically but expensive in practice without bounded algorithms, memoization strategy, and simplification order.
- Risk: exponential behavior on nested unions/refinements/operator types.

7. **Capability model does not fully specify delegation authority transfer**
- Non-escalation theorem references escalation events, but delegation semantics do not fix whether callee uses caller caps, own caps, or intersection.
- This is both a security and API usability issue.

8. **`match` exhaustiveness claim lacks pattern language formalization**
- The document asserts no runtime `match-fail` for typed programs, but there is no formal pattern-coverage algorithm for schema unions, refinements, or open enums.
- Implementability depends on a closed, decidable pattern fragment.

9. **Loop boundedness principle is not enough to guarantee practical termination/cost bounds**
- `loop(max=k, body)` desugaring is clear, but the source-level derivation of `k` and interaction with data-dependent recursion are unspecified.
- Complexity claims for stdlib ops remain unverifiable without parameterized cost contracts.

10. **Dynamic failure semantics are inconsistent across rules**
- Some violations become `failure(...)`, others become obligations, others are blocked states (`blocked(r,e)`).
- API users need a uniform effect/error model (typed failures, retryability, audit metadata).

11. **Probabilistic composition laws rely on independence assumptions not encoded in types/effects**
- `c_seq = c1*c2` is only valid under independence, yet there is no mechanism to track correlation.
- This can produce misleading confidence values in real systems.

### Minor

12. **`Ω` obligation lifecycle is not fully defined**
- Creation is clear, but persistence/discharge timing, serialization in artifacts, and runtime evidence format are unclear.

13. **Mutable aliasing boundary is policy-level, not mechanized**
- The text disallows mutating CAS-reachable objects but does not define runtime/object identity checks needed to enforce this in host VMs.

14. **Complexity section is implicit and non-normative**
- No explicit asymptotic table or worst-case bounds for core stdlib primitives (map/filter/join/hash/store/match/solve).

## Recommendations for Implementation-Friendly Adjustments

1. **Define an executable checker kernel**
- Specify syntax and normal forms for `Pre`, `Post`, and refinement formulas.
- Fix a mandatory solver fragment and deterministic `unknown` policy (e.g., reject at `PROVE_STATIC`, insert runtime guard at `PROVE_RUNTIME`).
- Add pseudocode for `infer`, `subtype`, `obligation_simplify`, and `vc_check` with complexity targets.

2. **Replace idealized hash axiom with implementable integrity semantics**
- Keep canonicalization requirement.
- Change A1 to one-way soundness (`canonical(v)=canonical(v') => digest(v)=digest(v')` for deterministic digest) and add collision-handling procedure.

3. **Make memory model storage-backend aware**
- Add required consistency levels per scope (`LOCAL/TASK/AGENT/SHARED/GLOBAL`) and define visibility/ordering guarantees (read-your-writes, monotonic reads, snapshot isolation where applicable).

4. **Formalize checkpoint/resume operational contract**
- Define whether checkpoints are quiescent-only or allow concurrent snapshots.
- Specify lock/table/task restoration behavior and side-effect replay policy (at-least-once vs exactly-once compensation model).

5. **Strengthen concurrency semantics into explicit transition rules**
- Add state machine for branch outcomes: `success | failure | timeout | cancelled`.
- Define join aggregation and error propagation per join mode.

6. **Constrain subtyping/refinement for tractable compilation**
- Require normalization (union flattening, refinement simplification) and memoized subtype checks.
- Prohibit/limit pathological nested contravariant `Op` types in public stdlib signatures.

7. **Fix capability delegation semantics normatively**
- Choose one: `callee_caps`, `caller_caps`, or `intersection` as default.
- Add explicit override policy and audit record schema.

8. **Publish a normative complexity profile for stdlib**
- For each primitive, provide expected average/worst-case complexity with assumptions (hash table quality, sorted inputs, network latency excluded/included).
- Separate compile-time complexity (typing/VC) from runtime complexity.

9. **Unify failure surface for API usability**
- Standardize typed error envelope (code, category, retry class, provenance hash, obligation id).
- Require all rule failures to map into this envelope.

10. **Add minimal conformance tests tied to rules**
- Include executable litmus tests for: capability denial, probable elimination, lock ordering, schema subtyping, collision behavior, and checkpoint resume invariants.

## Overall Assessment

The formal semantics is a strong foundation and demonstrates clear intent, but in current form it is closer to a rigorous design document than a directly implementable spec.

Assessment:
- **Implementation feasibility:** Medium (becomes high after operational tightening).
- **Type-checker executability:** Medium-low in current draft due to solver/obligation ambiguity.
- **Memory-model realism:** Medium-low due to collision idealization and consistency gaps.
- **Complexity realism for stdlib:** Low-medium until normative cost model is added.
- **API usability impact:** Significant risk unless failure semantics, delegation rules, and probabilistic evidence contracts are made precise.

Recommended release posture: treat this as **pre-normative for production conformance** until the critical items above are resolved.
