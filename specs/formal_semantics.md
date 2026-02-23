# AgentLang Formal Semantics and Type System (v1.0)

Status: Normative draft for production runtime/compiler implementations.

This document formalizes the dynamic semantics, static semantics, capability discipline, probabilistic reasoning, memory model, and concurrency model of AgentLang v1.0 (`AgentLang_Specification_v1.0.md`).

## 1. Scope and Conformance

An implementation conforms iff:

1. It accepts all well-typed programs defined by these rules.
2. Its execution relation refines the small-step transition system below.
3. It enforces capability and memory axioms at least as strictly as specified.
4. It preserves the stated soundness properties (up to explicitly named dynamic checks).

## 2. Core Calculus

We formalize a core language `AL_core` that conservatively embeds v1.0 constructs.

### 2.1 Syntax (Core)

Let:

- Variables: `x,y,z`.
- Hashes: `h`.
- Capabilities: `cap ∈ Cap`.
- Confidence values: `c ∈ [0,1]`.

Types:

- Primitive: `ι ::= Int | Float | Bool | Str | Bytes | Duration | Timestamp | Hash | Confidence | AgentId | TaskId`
- Composite: `τ ::= ι | List[τ] | Set[τ] | Map[τ,τ] | Schema S | Result[τ] | Probable[τ] | τ1 ∪ τ2 | {x:τ | φ}`
- Function-like op type: `Op(τin, τout, Pre, Post, Caps)`

Terms/expressions:

- `e ::= v | x | let x=e1 in e2 | if e0 then e1 else e2`
- `| match e with {pi -> ei}_i`
- `| op(e)`
- `| e1 -> e2` (pipeline stage application)
- `| probable(v,c,p,d)` (`value`, `confidence`, `provenance`, `degradation`)
- `| use e min_conf c0 else policy`
- `| delegate a op e with pol`
- `| store x = e` (named reference binding)
- `| mutable x = e | assign x = e`
- `| acquire r in e | checkpoint e | resume h`
- `| fork {bi=ei}_i join J`

Values:

- `v ::= n | b | s | hash(h) | [] | [v1..vn] | {k1=v1..kn=vn} | inl v | inr v | probable(v,c,p,d) | success(v) | FAILURE(ε,m,d)`

Canonical failure shape:

- `FailureDetails ::= Map[Str, JsonValue] | NONE`
- `Result[T] ::= SUCCESS(T) | FAILURE(ErrorCode, message: Str, details: FailureDetails)`

### 2.2 Runtime State

A machine state is:

`Σ = (H, R, M, K, A, Q, L)` where:

- `H : Hash ⇀ Value` is CAS heap (immutable by key).
- `R : Name ⇀ Hash` is named reference map (`STORE`).
- `M : Name ⇀ Value` is mutable cell map (`MUTABLE`).
- `K : AgentId → P(Cap)` is granted capabilities.
- `A` current agent id.
- `Q` scheduler queues / runnable DAG nodes.
- `L` lock table (`resource ↦ owner/queue`).

Evaluation configuration: `⟨e, ρ, Σ⟩` where `ρ` is lexical environment.

## 3. Static Semantics

Typing judgment:

`Γ; Ψ; C ⊢ e : τ ▷ Ω`

- `Γ`: term typing env.
- `Ψ`: refinement/dependent assumptions.
- `C`: available capabilities in current agent scope.
- `Ω`: proof obligations (SMT/runtime obligations from `REQUIRE/ENSURE/INVARIANT`).

### 3.1 Selected Typing Rules

Variable:

`(T-Var)`

`x:τ ∈ Γ`
`────────────`
`Γ;Ψ;C ⊢ x : τ ▷ ∅`

Let:

`(T-Let)`

`Γ;Ψ;C ⊢ e1 : τ1 ▷ Ω1`
`Γ,x:τ1;Ψ;C ⊢ e2 : τ2 ▷ Ω2`
`────────────────────────────────`
`Γ;Ψ;C ⊢ let x=e1 in e2 : τ2 ▷ Ω1∪Ω2`

Pipeline stage composition (`->`):

`(T-Pipe)`

`Γ;Ψ;C ⊢ e1 : τa ▷ Ω1`
`Γ;Ψ;C ⊢ e2 : Op(τa,τb,Pre,Post,Creq) ▷ Ω2`
`Creq ⊆ C`
`Ψ ⊨ Pre`
`────────────────────────────────────────────────`
`Γ;Ψ;C ⊢ e1 -> e2 : τb ▷ Ω1∪Ω2∪{Post}`

Probable introduction:

`(T-Prob-I)`

`Γ;Ψ;C ⊢ e : τ ▷ Ω    Ψ ⊨ 0≤c≤1`
`────────────────────────────────────`
`Γ;Ψ;C ⊢ probable(e,c,p,d) : Probable[τ] ▷ Ω`

Probable elimination (explicit handling requirement):

`(T-Prob-E)`

`Γ;Ψ;C ⊢ e : Probable[τ] ▷ Ω`
`Γ;Ψ∧(conf(e)≥c0);C ⊢ e_ok : τ' ▷ Ω1`
`Γ;Ψ∧(conf(e)<c0);C ⊢ e_bad : τ' ▷ Ω2`
`────────────────────────────────────────────────────────`
`Γ;Ψ;C ⊢ use e min_conf c0 else e_bad : τ' ▷ Ω∪Ω1∪Ω2`

No implicit cast from `Probable[τ]` to `τ` is admissible.

Capability-gated operation:

`(T-Cap)`

`Γ;Ψ;C ⊢ e : τin ▷ Ω`
`op : Op(τin,τout,Pre,Post,Creq)`
`Creq ⊆ C`
`────────────────────────────────────────`
`Γ;Ψ;C ⊢ op(e) : τout ▷ Ω∪{Pre,Post}`

Mutable assignment requires mutability witness:

`(T-Assign)`

`x:Mut[τ] ∈ Γ    Γ;Ψ;C ⊢ e:τ ▷ Ω`
`────────────────────────────────────`
`Γ;Ψ;C ⊢ assign x=e : Unit ▷ Ω`

## 4. Operational Semantics (Small-Step)

Transition relation:

`⟨e,ρ,Σ⟩ → ⟨e',ρ',Σ'⟩`

Evaluation contexts omitted for brevity; rules are call-by-value.

### 4.1 Core Reduction Rules

Let-binding:

`(E-Let1)`

`⟨e1,ρ,Σ⟩ → ⟨e1',ρ',Σ'⟩`
`────────────────────────────────────`
`⟨let x=e1 in e2,ρ,Σ⟩ → ⟨let x=e1' in e2,ρ',Σ'⟩`

`(E-LetV)`

`────────────────────────────────────────`
`⟨let x=v in e2,ρ,Σ⟩ → ⟨e2,ρ[x↦v],Σ⟩`

Conditional:

`(E-IfTrue)` `⟨if true then e1 else e2,ρ,Σ⟩ → ⟨e1,ρ,Σ⟩`

`(E-IfFalse)` `⟨if false then e1 else e2,ρ,Σ⟩ → ⟨e2,ρ,Σ⟩`

Operation call (pure core):

`(E-Op)`

`op(v) ⇓ v'` and side conditions (`Pre`, capabilities) hold
`────────────────────────────────────────`
`⟨op(v),ρ,Σ⟩ → ⟨v',ρ,Σ⟩`

If side conditions fail, step to `FAILURE(code,msg,details)` and mark obligation breach.

Pipeline:

`(E-Pipe)`

`────────────────────────────────`
`⟨v -> op,ρ,Σ⟩ → ⟨op(v),ρ,Σ⟩`

Probable use:

`(E-UseHigh)`

`c ≥ c0`
`────────────────────────────────────────────────────────────────`
`⟨use probable(v,c,p,d) min_conf c0 else ebad,ρ,Σ⟩ → ⟨v,ρ,Σ⟩`

`(E-UseLow)`

`c < c0`
`────────────────────────────────────────────────────────────────`
`⟨use probable(v,c,p,d) min_conf c0 else ebad,ρ,Σ⟩ → ⟨ebad,ρ,Σ⟩`

Store binding (CAS write + name map update):

`(E-Store)`

`h = digest(v)` and `H' = H[h↦v]`
`────────────────────────────────────────────────────`
`⟨store x=v,ρ,(H,R,M,K,A,Q,L)⟩ → ⟨hash(h),ρ,(H',R[x↦h],M,K,A,Q,L)⟩`

Mutable assignment:

`(E-Assign)`

`────────────────────────────────────────────────`
`⟨assign x=v,ρ,(H,R,M,K,A,Q,L)⟩ → ⟨unit,ρ,(H,R,M[x↦v],K,A,Q,L)⟩`

Delegation:

`(E-Delegate)`

`DELEGATE ∈ K(A)` (caller must hold DELEGATE capability)
`a'` reachable and policy permits isolation/context transfer
`Ceff = K(a')` (callee executes with callee's own capabilities, not caller's)
`Q' = enqueue(Q, task(a',op,v,pol,Ceff))`
`────────────────────────────────────────────────────────────────────────`
`⟨delegate a' op v with pol,ρ,(H,R,M,K,A,Q,L)⟩ → ⟨task_id(t),ρ,(H,R,M,K,A,Q',L)⟩`

> **MVP v0.1 rule (normative):** Delegation executes under callee capabilities only. No implicit capability inheritance or caller-callee intersection is permitted. Caller must hold `DELEGATE` in `K(A)`. See `MVP_PROFILE.md` §8.

Locking:

`(E-AcquireFree)`

`L(r)=free`
`────────────────────────────────────────────────────`
`⟨acquire r in e,ρ,Σ⟩ → ⟨e,ρ,lock(Σ,r,A)⟩`

`(E-AcquireBusy)`

`L(r)=held(A')` and `A'≠A`
`────────────────────────────────────────────────────`
`⟨acquire r in e,ρ,Σ⟩ → ⟨blocked(r,e),ρ,enq_wait(Σ,r,A)⟩`

Checkpoint:

`(E-Checkpoint)`

`cp = snapshot(ρ,Σ,Q)`; `h = digest(cp)`; `H' = H[h↦cp]`
`────────────────────────────────────────────────────────`
`⟨checkpoint e,ρ,Σ⟩ → ⟨e,ρ,Σ[H:=H',R:=R[checkpoint↦h]]⟩`

Resume:

`(E-Resume)`

`H(h)=cp` and `verify_hash(cp,h)`
`──────────────────────────────────────────`
`⟨resume h,ρ,Σ⟩ → ⟨restore(cp),ρ,Σ⟩`

Assert (runtime verification check):

`(E-AssertTrue)`

`⟨e,ρ,Σ⟩ →* ⟨true,ρ',Σ'⟩`
`────────────────────────────────────────`
`⟨assert e,ρ,Σ⟩ → ⟨unit,ρ',Σ'⟩`

`(E-AssertFalse)`

`⟨e,ρ,Σ⟩ →* ⟨false,ρ',Σ'⟩`
`────────────────────────────────────────`
`⟨assert e,ρ,Σ⟩ → ⟨FAILURE(ASSERTION_FAILED, "Assertion violated", {expression: repr(e), vc_id: id}),ρ',Σ'⟩`

> **MVP v0.1 note:** Auto-inserted `ASSERT` checks from SMT `Unknown` results must include `vc_id` and solver reason in `details` for audit traceability (see §7.3 and `MVP_PROFILE.md` §10).

### 4.2 Match Exhaustiveness

A well-typed `match` over finite enum/union must be exhaustive in typing; runtime has no `match-fail` state for typed programs.

### 4.3 Bounded Loop Principle

Core loop desugars to primitive recursion with explicit fuel `k`:

`loop(max=k, body)` is translated into `iter(k,s0,f,halt_pred)`.

No unbounded reduction chain is typable from surface `LOOP`.

## 5. Subtyping

Subtype relation `τ1 <: τ2` is the least relation closed under rules below.

Reflexive/transitive:

- `(S-Refl)` `τ <: τ`
- `(S-Trans)` `τ1<:τ2 ∧ τ2<:τ3 ⇒ τ1<:τ3`

Unions:

- `(S-UnionL)` `τ1 <: τ1 ∪ τ2`
- `(S-UnionR)` `τ2 <: τ1 ∪ τ2`
- `(S-UnionElim)` if `τ1<:τ ∧ τ2<:τ` then `τ1∪τ2 <: τ`

Collections:

- `List`/`Set` are covariant on immutable elements: `τ1<:τ2 ⇒ List[τ1] <: List[τ2]`.
- `Map[K,V]` is covariant in `V`, invariant in `K` (hash/equality safety).

Schemas (width + depth, immutable fields):

`(S-Schema)`

If `S1` has all fields of `S2` and for each shared field `f`, `S1.f <: S2.f`, then `Schema S1 <: Schema S2`.

Refinements:

`(S-Refine)`

`{x:τ | φ} <: τ`

`(S-Refine-Strengthen)`

`Ψ ⊨ φ1 ⇒ φ2` implies `{x:τ | φ1} <: {x:τ | φ2}`.

Probabilistic wrapper:

`(S-Prob)`

`τ1 <: τ2 ⇒ Probable[τ1] <: Probable[τ2]`.

No rule permits `Probable[τ] <: τ`.

Function/operator types (pre/post/capability aware):

`Op(τin2,τout1,Pre1,Post1,C1) <: Op(τin1,τout2,Pre2,Post2,C2)` iff

- `τin1 <: τin2` (contravariant input),
- `τout1 <: τout2` (covariant output),
- `Pre2 ⇒ Pre1`,
- `Post1 ⇒ Post2`,
- `C1 ⊆ C2` (subtype requires no additional capability beyond supertype contract).

## 6. Probabilistic Type Semantics

`Probable[τ]` denotes tuples `(v,c,p,d)` with `v∈⟦τ⟧`, `c∈[0,1]`, provenance `p`, degradation policy `d`.

Denotationally:

`⟦Probable[τ]⟧ = { (v,c,p,d) | v∈⟦τ⟧ ∧ 0≤c≤1 ∧ validProv(p) ∧ validPolicy(d) }`

Confidence ordering:

`(v,c1,_,_) ⪯ (v,c2,_,_)` iff `c1 ≤ c2`.

### 6.1 Confidence Bounds as Obligations

Using a probabilistic value where certainty is required generates obligation:

`Ω += { conf(e) ≥ θ }` or a mandatory branch handling low-confidence case.

This corresponds to language-level requirement that `Probable[T]` cannot flow into `T` unchecked.

### 6.2 Composition Laws

For independent steps with confidence `c1,c2`, conservative composition:

- Sequential conjunction: `c_seq = c1 * c2`.
- Alternative best-of retries (`n` i.i.d attempts): `c_retry = 1-(1-c)^n`.
- Delegation with trust attenuation `t_agent`: `c_eff = c * t_agent`.

Implementations may use tighter estimators if they are monotone and sound w.r.t. lower bound interpretation.

### 6.3 Probabilistic Safety Theorem (Bounded Use)

If all elimination sites of `Probable[τ]` enforce threshold `θ`, then every extracted `τ` value in successful execution paths satisfies run-time evidence `c≥θ`.

Proof sketch: by induction on reduction steps; only `(E-UseHigh)` yields `τ` from `Probable[τ]`, guarded by `c≥θ`.

## 7. Dependent Type Checking

Surface dependent forms use refinements and indexed containers.

Examples represented as:

- `List[T, length: n]  ≡  {x:List[T] | len(x)=n}`
- `Float64 :: range(a..b) ≡ {x:Float | a≤x≤b}`

### 7.1 Decidable Fragment

Type checker accepts constraints in solver-backed quantifier-light fragment:

- Linear integer/real arithmetic.
- Finite-domain enum predicates.
- Size/length/range constraints.
- Acyclic symbolic equalities over immutable fields.

### 7.2 Algorithm

Given expression `e` and expected type `{x:τ|φ}`:

1. Infer base type `τe` and obligations `Ω`.
2. Check subtype `τe <: τ`.
3. Generate VC: `Ψ ∧ Ω ⇒ φ[e/x]`.
4. Query SMT.
5. If `sat-valid`, accept.
6. If `unknown`, accept compilation and auto-insert a runtime `ASSERT(VC_id, φ[e/x])` guard at the obligation boundary.
7. If `invalid`, reject.

Pseudo-judgment:

`checkDep(Γ,Ψ,e,{x:τ|φ}) = ok` iff `infer(e)=(τe,Ω)` and `τe<:τ` and `SMT(Ψ∧Ω ⇒ φ[e/x])=valid`.

### 7.3 Soundness Boundary

Dependent guarantees are static for solver-validated VCs. For SMT `unknown`, the compiler must fail-open at compile time by inserting runtime `ASSERT` checks, and execution fails-closed if any inserted check is false (with audit evidence and `FAILURE(ASSERTION_FAILED, message, details)`).

> **MVP v0.1 deterministic SMT Unknown policy (normative):**
> 1. Solver interface returns `Valid`, `Invalid(counterexample)`, or `Unknown(reason)`.
> 2. On `Unknown`: compiler auto-inserts a runtime `ASSERT` for the unresolved VC. Compilation succeeds (fail-open).
> 3. At runtime: if the auto-inserted `ASSERT` evaluates to false, execution halts with `FAILURE(ASSERTION_FAILED, message: Str, details: FailureDetails)` where `details` includes `vc_id` and `solver_reason`. This is fail-closed.
> 4. All auto-inserted `ASSERT` checks must be recorded in audit output.
> See `MVP_PROFILE.md` §10 and operational rule `(E-AssertFalse)` in §4.1.

## 8. Capability System Formalization

Capability judgment:

`Γ;Ψ;C ⊢ e : τ [uses U]`

Requirement: `U ⊆ C`.

### 8.1 Static Rules

- Pure expressions: `uses ∅`.
- `DB_READ`, `API_CALL`, etc. annotated on primitive ops; usage set accumulates by union.
- Delegation requires `DELEGATE` capability in caller's set; callee executes under callee's own capabilities (MVP v0.1 normative rule, see §4.1 E-Delegate).

`(Cap-Seq)`

If `e1` uses `U1` and `e2` uses `U2`, then `let x=e1 in e2` uses `U1∪U2`.

`(Cap-Call)`

If `op` requires `Creq`, then `op(e)` uses `Creq ∪ Ue`.

### 8.2 Dynamic Enforcement

Runtime check at effect boundary:

If step attempts effect `eff(cap)` and `cap ∉ K(A)`, transition to `FAILURE(CAPABILITY_DENIED, "Missing capability", {required_capability: cap})` and log audit entry.

### 8.3 Non-Escalation Theorem

For any reduction sequence of a well-typed program, observed successful effects use only capabilities in initial `C` unless an explicit `ESCALATE` event is approved and logged.

Proof sketch: static containment `U⊆C`; runtime denies missing caps; only escalation rule can extend `C`, and requires external authorization predicate.

## 9. Memory Model Axioms (CAS + References)

### 9.1 CAS Axioms

A1. Deterministic digest: `digest(v)=digest(v') ⇔ canonical(v)=canonical(v')` (collision-free idealization).

A2. Immutability by hash: if `H(h)=v` then forever `H(h)=v`.

A3. Content addressing: write stores by hash only; name indirection changes do not mutate prior objects.

A4. Integrity: load by hash returns value iff recomputed digest matches hash.

A5. Deduplication: writing equivalent value does not allocate a distinct semantic object.

### 9.2 Named Reference Semantics

- `STORE x = e` evaluates `e→v`, computes `h=digest(v)`, sets `R(x)=h`.
- Rebinding `STORE x = e2` changes only `R(x)`; historic hashes remain reachable by hash and audit chain.

### 9.3 Mutability Discipline

`MUTABLE` cells are separate from CAS objects; mutating `M[x]` does not mutate any `H(h)` value.

### 9.4 Scoping Invariant

For each object label `ℓ`, visibility set `Vis(ℓ)` obeys:

`LOCAL ⊂ TASK ⊂ AGENT ⊂ SHARED ⊂ GLOBAL` (partial order by visibility/lifetime).

Cross-scope reads/writes must satisfy policy monotonicity and capability checks.

## 10. Concurrency Model (Dataflow Formalization)

A program induces DAG `G=(V,E)` where nodes are operations, edges are data dependencies.

Ready predicate:

`ready(v,σ) ⇔ ∀(u,v)∈E. done(u,σ)`.

Scheduler rule:

`(C-Ready)`

If `ready(v,σ)` and resources available, `v` may execute.

This yields nondeterministic interleavings of independent nodes.

### 10.1 Determinism up to Independence

If all nodes are pure and immutable inputs, then any topological execution order yields observationally equivalent outputs.

### 10.2 Fork/Join Semantics

`fork {bi=ei}_i join J` creates subgraph roots `ri` with shared parent dependencies.

Join readiness depends on strategy:

- `ALL_COMPLETE`: all branches done.
- `BEST_EFFORT`: proceed on timeout/partial threshold predicate.
- `PARTIAL(min=k)`: at least `k` branches done.

Formal join gate `gate_J(σ)` determines transition eligibility.

> **MVP v0.1 restriction (normative):** Only `ALL_COMPLETE` is permitted. `BEST_EFFORT` and `PARTIAL(min=k)` must be rejected at compile time with `NOT_IMPLEMENTED` diagnostic and profile tag `mvp-0.1`. See `MVP_PROFILE.md` §2.

### 10.3 Locking and Deadlock Prevention

For mutable shared resources, lock acquisition order is a strict global order `<L`.

Rule: a thread/agent may acquire sequence `(r1..rn)` only if `r1 <L ... <L rn`.

Theorem (cycle freedom): with strict total lock order and no out-of-order acquisition, wait-for graph is acyclic.

## 11. Type Soundness Sketch

We state soundness for `AL_core` with dynamic checks for capabilities/probabilities/refinements where specified.

### 11.1 Progress

Theorem (Progress):

If `∅;Ψ;C ⊢ e:τ ▷ Ω`, obligations in `Ω` are either solver-discharged or represented by explicit runtime checks in `e`, and runtime preconditions hold for initial state, then either:

1. `e` is a value, or
2. `e = FAILURE(code, message, details)`, or
3. there exists `e',Σ'` such that `⟨e,∅,Σ⟩ → ⟨e',∅,Σ'⟩`.

Proof sketch: structural induction on typing derivation. Canonical forms handle booleans/unions/probables; elimination forms (`use`, `match`) are exhaustive by typing. Effectful stuck states are prevented by capability checks becoming explicit `FAILURE` states, not stuckness.

### 11.2 Preservation

Theorem (Preservation):

If `Γ;Ψ;C ⊢ e:τ ▷ Ω` and `⟨e,ρ,Σ⟩ → ⟨e',ρ',Σ'⟩` under valid runtime invariants, then there exists `Ψ'` such that `Γ';Ψ';C' ⊢ e':τ' ▷ Ω'` with `τ' <: τ`, and all new obligations are sound refinements of prior obligations or discharged checks.

Proof sketch: induction on reduction step. Key lemmas:

- Substitution lemma for `let`.
- CAS immutability lemma for `STORE`.
- Capability monotonicity lemma (`C'` unchanged unless authorized escalation).
- Probable-elimination lemma (`use` returns `τ` only on high-confidence branch).

Corollary: Well-typed programs do not reach untyped stuck states.

## 12. Counter-Examples and Edge Cases

### 12.1 Illegal Probable Erasure

Program fragment:

`x: Str = llm_generate("hi")  // llm_generate : Probable[Str]`

Why rejected: would require forbidden subtyping `Probable[Str] <: Str`.

### 12.2 Non-Exhaustive Match on Enum

`MATCH tier WHEN gold -> ...`

If `tier : ENUM(gold,silver)`, missing `silver` branch. Rejected by exhaustiveness checker; avoids runtime `match-fail`.

### 12.3 Mutable Alias to CAS Object (Disallowed)

Attempt: mutating an object reached through hash reference.

Why disallowed: violates A2 immutability; mutable updates must target `MUTABLE` cells, producing new CAS objects when persisted.

### 12.4 Capability Smuggling via Delegation

Low-cap agent tries delegating task requiring `FILE_WRITE` without permission.

Outcome: static rejection if requirement known; otherwise runtime `CAPABILITY_DENIED` at callee boundary under policy isolation.

### 12.5 Deadlock Attempt with Inverted Lock Order

Agent1 acquires `b` then `a`; Agent2 acquires `a` then `b`.

Rejected/blocked by lock-order checker requiring declared canonical order.

## 13. Relationship to Existing Type/Effect Systems

### 13.1 System F

- Similarity: parametric polymorphism foundation can encode `Result[T]`, `List[T]`.
- Difference: AgentLang adds effect/capability constraints, probabilistic wrapper semantics, and refinement obligations not present in plain System F.
- Consequence: type soundness argument follows System F style structure but requires effect and runtime-check lemmas.

### 13.2 Dependent Haskell

- Similarity: value-indexed types (`length`, `range`) and proposition-carrying terms.
- Difference: AgentLang deliberately restricts to a decidable SMT-friendly fragment plus explicit runtime proof fallback.
- Consequence: less expressive than full dependent typing, more predictable for production compilers.

### 13.3 Rust Borrow Checker

- Similarity: strong safety around mutation/concurrency; explicit discipline to prevent races.
- Difference: AgentLang achieves race safety primarily via immutable CAS + scoped mutability + lock ordering, rather than affine ownership/lifetime inference.
- Consequence: easier distributed persistence/audit integration, weaker compile-time alias precision than Rust’s borrow checker.

## 14. Open Questions and Ambiguities in v1.0 Source Spec

1. Confidence algebra is underspecified: independence assumptions and calibration method are not fixed.
2. `Probable[T]` provenance format is illustrated but not normatively typed (hash chain vs structured evidence object).
3. `PROVE_RUNTIME` evidence schema details (required fields and canonical serialization) should be specified precisely.
4. Schema subtyping and evolution (width/depth/optional fields) are implicit; compatibility rules for migrations need exact normative constraints.
5. ~~Capability interaction with `DELEGATE` is described informally.~~ **Resolved for MVP v0.1:** callee runs with callee's own capabilities; caller must hold `DELEGATE`. See §4.1 (E-Delegate) and `MVP_PROFILE.md` §8.
6. Lock semantics mention deadlock prevention but do not state global lock order source (static declaration vs runtime canonicalization).
7. DAG semantics for `BEST_EFFORT` join under failures/timeouts need precise treatment for partial outputs and compensation logic.
8. `MUTABLE` justification annotation is required syntactically but enforcement criteria are not formalized.
9. Checkpoint consistency model (snapshot isolation vs eventual consistency for shared scope) is not defined.
10. Collision model for hashes is assumed away; operational requirements under cryptographic collision events are unspecified.

## 15. Summary of Definitions in This Document

This formalization defines:

- A core typed calculus for AgentLang constructs.
- Small-step operational semantics over explicit runtime state (`CAS`, references, mutable cells, capabilities, scheduler, locks).
- Formal subtyping relation including unions, schemas, refinements, and probabilistic wrappers.
- Probabilistic type semantics with confidence-bound elimination and composition laws.
- A practical dependent type-checking algorithm with SMT obligations and runtime-proof fallback.
- Capability type/effect rules with dynamic enforcement and non-escalation guarantee.
- CAS memory axioms and scoping invariants.
- DAG-based concurrency semantics with fork/join and deadlock-freedom constraints.
- Soundness theorem statements (progress/preservation) with proof sketches and key lemmas.
