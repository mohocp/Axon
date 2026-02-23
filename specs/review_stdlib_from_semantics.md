# Formal Semantics Review of `stdlib_spec.md`

## Summary of Soundness Findings

Overall, the standard library spec is directionally aligned with AgentLang's formal model (explicit `REQUIRE`/`ENSURE`, capability-gated ops, and `Probable[T]` for LLM APIs), but it is **not yet type-sound as written** against the current formal semantics.

Key blockers are:
- inconsistent error/result typing (operations list `FAILURE` behaviors but mostly return plain `T`),
- use of types not present in the formal calculus (`Any`, tuple `(A,B)`),
- several ill-typed defaults for `encoder`/`decoder` identities,
- capability/error taxonomy mismatch (`UNAUTHORIZED` vs formal `CAPABILITY_DENIED`),
- generic/subtyping inconsistencies that force runtime checks where static guarantees are expected.

These issues would break preservation/progress guarantees unless additional implicit typing rules are introduced.

## Potential Issues (Severity-Ranked)

### Critical

1. **Result typing is inconsistent with failure semantics**  
   - Evidence: Most signatures return plain values (`-> T`, `-> List[T]`, etc.) while also declaring `FAILURE` outcomes (`stdlib_spec.md:40`, `stdlib_spec.md:152`, `stdlib_spec.md:417`, many others).  
   - Formal conflict: language-level error model is `Result[T] = SUCCESS(T) | FAILURE(ErrorCode, Str)` (`AgentLang_Specification_v1.0.md:278`), and semantics reduces failed preconditions to `failure(...)` values (`formal_semantics.md` section 4.1, `E-Op`).  
   - Why unsound: a function typed as `T` that may produce `FAILURE` violates canonical forms and preservation unless every such return is modeled as `Result[T]` (or equivalent union).

2. **`Any` appears widely, but formal type grammar has no top type**  
   - Evidence: `List[Any]`, `Map[Str,Any]`, `Schema[Any]`, etc. (`stdlib_spec.md:117`, `stdlib_spec.md:244`, `stdlib_spec.md:559`, `stdlib_spec.md:631`, `stdlib_spec.md:668`).  
   - Formal conflict: core types do not include `Any` (`formal_semantics.md:31-33`).  
   - Why unsound: typing derivations cannot be constructed for these signatures without adding a new top type and subtyping axioms.

3. **Tuple return type in `ZIP` is not in formal type constructors**  
   - Evidence: `ZIP ... -> List[(A,B)]` (`stdlib_spec.md:139`).  
   - Formal conflict: no tuple/product type in formal syntax (`formal_semantics.md:31-33`).  
   - Why unsound: this type is currently unexpressible in the formal system.

4. **Ill-typed default function values (`identity`) in encoder/decoder signatures**  
   - Evidence: `WRITE[T](..., encoder: (T)->Bytes=identity)` (`stdlib_spec.md:163`), `POST[B,...](..., encoder: (B)->Bytes=identity)` (`stdlib_spec.md:428`), `PUT` same (`stdlib_spec.md:439`).  
   - Why unsound: `identity` has type `(X)->X`, not `(T)->Bytes` unless `T=Bytes`; these defaults are not polymorphically valid for arbitrary `T`/`B`.

### Major

5. **Failure constructor arity and match patterns conflict with base spec**  
   - Evidence: stdlib convention uses `FAILURE(code, message, details)` (`stdlib_spec.md:15`, `stdlib_spec.md:734-736`).  
   - Formal/base spec uses `FAILURE(ErrorCode, Str)` (`AgentLang_Specification_v1.0.md:278`) and semantics examples use `failure(code,msg)` (`formal_semantics.md` values/rules).  
   - Impact: pattern typing and interop for failure handling are ambiguous.

6. **Capability failure taxonomy mismatch (`UNAUTHORIZED` vs `CAPABILITY_DENIED`)**  
   - Evidence: shared code list omits `CAPABILITY_DENIED` and uses `UNAUTHORIZED` (`stdlib_spec.md:19-33`).  
   - Formal conflict: runtime capability violation transitions to `failure(CAPABILITY_DENIED, cap)` (`formal_semantics.md:428`).  
   - Impact: capability theorem/audit semantics cannot be applied uniformly.

7. **Capability requirements are declared but missing corresponding failure paths in some ops**  
   - Evidence: `STREAM` requires read capability but no authorization/capability failure (`stdlib_spec.md:186-188`); `TOOLS.INVOKE` and `agent.reflection.*` require capabilities but omit auth failure (`stdlib_spec.md:547-550`, `stdlib_spec.md:584-587`, `stdlib_spec.md:595-598`).  
   - Impact: violates effect-boundary failure consistency.

8. **`agent.memory.RECALL` default parameter is ill-typed**  
   - Evidence: `default: V=NONE` with return `V|NONE` (`stdlib_spec.md:511`).  
   - Why unsound: `NONE` is not generally inhabitant of `V`; parameter type should be `V|NONE`.

9. **Generic variable used without binder**  
   - Evidence: `SCHEDULE(task: ()->T, ...) -> TaskId` (`stdlib_spec.md:312`) has `T` not declared in `SCHEDULE[...]`.  
   - Impact: signature is not well-kinded/well-scoped.

10. **Subtyping/bounds encoded as runtime `REQUIRE` rather than type bounds**  
   - Evidence: `SORT`/`GROUP`/`DISTINCT` require comparable/hashable keys (`stdlib_spec.md:74-75`, `stdlib_spec.md:85`, `stdlib_spec.md:129`) but signatures leave `K` unconstrained.  
   - Impact: pushes avoidable type errors to runtime; weakens static soundness guarantees for library contracts.

11. **Probabilistic module consistency gap for reflection APIs**  
   - Evidence: determinism note marks "some `agent.reflection`" as probabilistic (`stdlib_spec.md:789`), but `EVALUATE`/`CRITIQUE` return deterministic types (`stdlib_spec.md:583`, `stdlib_spec.md:594`).  
   - Impact: either signatures are wrong (should be `Probable[...]`) or determinism class is wrong.

### Minor

12. **Redundant/under-specified refinement opportunities**  
   - Examples: `n: UInt64` with `REQUIRE n>=0` (`stdlib_spec.md:95-97`, `stdlib_spec.md:106-108`) is redundant; `count: UInt32` with `REQUIRE count>=1` (`stdlib_spec.md:323-325`) should be a refinement type; `CLASSIFY` could encode `labels: List[L, length: 2..]` instead of ad hoc precondition (`stdlib_spec.md:476-478`).  
   - Impact: not unsound alone, but misses static discharge opportunities supported by refinement machinery.

## Recommendations for Fixes

1. **Normalize operation result typing**  
   - Adopt one rule globally: every fallible operation returns `Result[T]` (or equivalent `SUCCESS(T) | FAILURE(...)`).  
   - Update all signatures and examples/match templates accordingly.

2. **Align failure constructor and error taxonomy with formal semantics**  
   - Either:  
     - extend formal spec to 3-ary failure (`code,message,details`) and update semantics, or  
     - collapse stdlib to 2-ary failure and move details into structured message payload.  
   - Add/standardize `CAPABILITY_DENIED` mapping (or explicitly define `UNAUTHORIZED` as an alias in the formal layer).

3. **Remove untyped placeholders from public API**  
   - Replace `Any` with one of: `JsonValue`, explicit unions, generic variables, or `Schema`-bounded existential wrappers if you decide to formally add top type support.  
   - Replace `List[(A,B)]` with a schema/product type representable in the core system (or formally add tuple type constructor).

4. **Fix polymorphic defaults**  
   - For encoder/decoder params, use defaults that are type-correct by construction:  
     - constrain type parameter (`B=Bytes`) when `identity` is used, or  
     - provide concrete default codec (`UTF8.ENCODE`, `UTF8.DECODE`, etc.) with matching types.

5. **Tighten capability discipline**  
   - Ensure every operation with capability in `REQUIRE` includes capability-denial failure in `FAILURE`.  
   - Prefer canonical capability atoms (`DB_READ`, `NET_WRITE`, `LLM_CALL`, etc.) over free-text names.

6. **Strengthen generic bounds and refinements**  
   - Promote key constraints into type bounds: `K: Comparable`, `K: Hashable`.  
   - Encode numeric/length preconditions as refinements where possible (`count: UInt32 :: range(1..MAX)`, `labels: List[L, length: 2..]`, `min_confidence: Confidence`).

7. **Clarify probabilistic API contract boundaries**  
   - Keep `GENERATE`/`CLASSIFY`/`EXTRACT` as `Probable[T]` (good), but specify elimination pattern in examples (`use ... min_conf ... else ...`) to satisfy formal explicit-handling requirement.  
   - Resolve reflection inconsistency by either returning `Probable[...]` or reclassifying as deterministic/conditionally deterministic.

## Overall Assessment

**Assessment: Not formally sound yet (requires revision before normative adoption).**

The spec has a strong structure and good intent, but current contracts are not fully compatible with AgentLang’s formal type/effect model. The most important fixes are: (1) uniform `Result` typing for fallible ops, (2) removal/formalization of `Any` and tuple types, and (3) capability/error taxonomy alignment. Once those are corrected, the remaining issues are mostly mechanical refinements rather than deep semantic redesign.
