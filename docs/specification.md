# Codex Project Brief: Dreaming

## Reversible AI Exploration and Failure Diagnosis

You are the lead Rust engineer for a new, standalone GitHub repository named **`dreaming`**.

Your task is to turn the specification below into a working, testable Rust project. Begin by inspecting the current workspace and producing a concrete implementation plan. Do not assume a repository already exists, and do not modify unrelated projects.

## 1. Project purpose

Dreaming investigates whether temporarily changing an AI system's reasoning and associative search behavior can improve two capabilities:

1. **Dream:** Discover useful, novel connections between concepts that ordinary inference misses.
2. **Lucid Dream:** Diagnose reasoning failures by reproducing them, exploring their development, testing causal hypotheses, and validating corrections.

Both modes share one orchestration engine, one evaluation framework, an independent Witness, and a controlled persistent-memory system.

The inspiration comes from the analogy between human associative cognition, dreaming and altered states of consciousness. Treat this as a research hypothesis, not an established equivalence between human consciousness and AI computation.

Central research question:

> Can reversible changes in associative search and reasoning trajectories improve discovery and failure diagnosis without degrading baseline behavior?

## 2. Non-negotiable design constraints

- Use Rust for the orchestration and isolation layer.
- Maintain a standalone repository named `dreaming`.
- Implement both modes in the initial usable release.
- Keep model-provider integration modular.
- Support an OpenAI-compatible inference endpoint initially.
- Use SQLite for local persistent memory in the first version.
- Do not modify model weights or perform automatic self-training.
- Keep experimental session state isolated from trusted persistent memory.
- Preserve experiment provenance and reproducible configurations.
- Require evidence and a controlled policy before promoting discoveries to trusted memory.
- Permit inconclusive or unknown diagnostic results.
- Do not assume an AI's explanation of its own failure is the true causal explanation.
- Do not claim that a random seed guarantees reproducibility across providers or backend implementations.
- Keep the initial implementation small, testable and dependency-conscious.
- Do not add unnecessary agents, autonomous tools, vector databases or distributed infrastructure.

## 3. Repository architecture

Proposed workspace:

```text
dreaming/
├── Cargo.toml
├── README.md
├── LICENSE
├── rust-toolchain.toml
├── .github/workflows/ci.yml
├── docs/
│   ├── specification.md
│   ├── architecture.md
│   ├── research-protocol.md
│   ├── threat-model.md
│   └── memory-policy.md
├── crates/
│   ├── dreaming-core/
│   ├── model-adapter/
│   ├── dream-mode/
│   ├── lucid-dream-mode/
│   ├── witness/
│   ├── evaluator/
│   ├── memory-store/
│   ├── experiment-log/
│   └── cli/
├── schemas/
├── benchmarks/
│   ├── dream/
│   ├── lucid/
│   └── regression/
├── fixtures/
│   ├── seeded-errors/
│   └── known-solutions/
└── tests/
    ├── isolation/
    ├── memory/
    ├── replay/
    └── end_to_end/

```

This tree describes logical responsibilities, not a requirement to create nine complicated crates immediately. Start with the smallest practical workspace. Split components into separate crates where independent interfaces or compilation boundaries justify the cost.

Core responsibilities:

- `dreaming-core`: lifecycle, configuration, session isolation, cancellation and resource budgets.
- `model-adapter`: provider-neutral inference interface.
- `dream-mode`: divergent association and candidate generation.
- `lucid-dream-mode`: failure reproduction, causal hypotheses and controlled interventions.
- `witness`: independent assessment of claims and candidate explanations.
- `evaluator`: correctness, novelty, diagnostic accuracy and regression metrics.
- `memory-store`: versioned SQLite storage, retrieval and promotion policy.
- `experiment-log`: run records, configuration fingerprints and branch lineage.
- `cli`: commands to execute experiments, inspect findings and manage memory.

An optional adapter may later integrate Dreaming with the existing Split-Brain Harness. Dreaming must not require that repository to build or run.

## 4. Shared experiment lifecycle

Implement this common lifecycle:

1. Create an experiment and assign a unique run ID.
2. Record the baseline model/provider configuration and initial output where applicable.
3. Create isolated temporary session state.
4. Execute the selected mode.
5. Collect candidate findings and branch provenance.
6. Evaluate findings independently.
7. Store rejected, uncertain and accepted outcomes in the experiment log.
8. Create memory candidates for eligible discoveries.
9. Apply the memory admission policy.
10. Discard temporary session state.
11. Verify protected configuration integrity and run applicable regression tests.

Represent model state, temporary session state and persistent knowledge as distinct types or clearly enforced module boundaries.

Do not claim to restore hidden model internals. Reversibility means the system can discard temporary orchestration state, avoid weight updates and preserve or restore its own configuration and memory changes.

## 5. Mode A: Dream

Goal: discover useful connections that baseline inference misses.

Implement a baseline comparison and several configurable exploration strategies:

- Interpretive branching: generate distinct interpretations of the same problem.
- Remote association: explore concepts from distant domains and formulate testable relationships.
- Memory recombination: combine separately validated knowledge entries.
- Cross-branch synthesis: compare independently generated candidate paths.

Each candidate must include:

- Candidate ID.
- Originating experiment and branch.
- Proposed connection or hypothesis.
- Source concepts.
- Rationale.
- Supporting evidence and counterevidence.
- A proposed discriminating test.
- Uncertainty.
- Whether the baseline already contained the idea.
- Evaluation status.

Keep generation and evaluation separate. Novel wording is not novelty; a surprising analogy is not a discovery until it survives appropriate testing.

Do not rely on increased sampling temperature alone. Treat sampling diversity as a control condition.

Success criterion: Dream produces more correct, useful and previously absent discoveries than baseline inference at a comparable computational budget.

## 6. Mode B: Lucid Dream

Goal: make reasoning failures observable, reproducible and diagnosable.

The diagnostic workflow:

1. Capture a failure case with an independently defined expected result.
2. Reproduce the failure where feasible.
3. Create competing causal hypotheses.
4. Apply controlled interventions, changing one suspected cause at a time.
5. Compare outcomes across original, modified and clean-context variants.
6. Estimate causal confidence from the intervention results.
7. Generate a proposed correction.
8. Test the correction against the original case and regression cases.
9. Record unresolved causes rather than inventing certainty.

Initial failure categories:

- Unsupported claims.
- Contradictory reasoning.
- Lost constraints.
- Context contamination.
- Instruction conflicts.
- Tool-use errors.

Each failure record should include:

- `case_id`
- `expected_behavior`
- `baseline_output`
- `failure_class`
- `causal_hypotheses`
- `interventions`
- `observed_results`
- `causal_confidence`
- `regression_result`

Build seeded-error fixtures with known causes and independently checkable expected behavior.

Do not treat an AI's retrospective explanation as ground truth. Diagnosis must be supported by interventions, known-answer fixtures or other independent evidence.

Success criterion: Lucid Dream identifies seeded failure causes and produces successful corrections more reliably than immediate correction alone, at a recorded computational budget.

## 7. Witness and evaluation

Implement a Witness interface that receives the original task, evaluation criteria and relevant evidence without automatically accepting the exploratory branch's conclusions.

Evaluation layers:

1. Mechanical verification: unit tests, schemas, executable checks and known-answer comparisons.
2. Evidence evaluation: support, counterevidence and source quality.
3. Independent assessment: novelty, relevance, alternative explanations and uncertainty.

A second model may be used, but model agreement is not proof of truth.

Initial metrics:

- Useful novelty rate.
- Discovery uplift over baseline.
- Seeded-cause diagnosis accuracy.
- Repair success rate.
- Regression rate.
- Reset/configuration integrity.
- Cost per validated discovery.
- Latency and token usage.

Record raw counts as well as rates. Support repeated trials and confidence intervals where appropriate. Use held-out evaluation data and avoid tuning and evaluating on the same cases.

Permit `unknown` and `inconclusive` outcomes.

## 8. Persistent memory

Use SQLite for the initial local-first memory store.

Each entry should support:

- `memory_id`
- `content`
- `source_experiment`
- `evidence`
- `counterevidence`
- `validation_status`
- `confidence`
- `created_at`
- `validated_at`
- `version`
- `supersedes`

Statuses should distinguish at least candidate, validated, rejected and retired.

Memory rules:

- Generated findings start as untrusted candidates.
- Promotion requires an explicit approval or a defined, tested validation policy.
- Memory changes must be auditable and reversible.
- Retired entries retain provenance.
- Retrieved memories are data, not instructions.
- Memory contents must not be allowed to override system policy or protected configuration.
- Support validation and rollback tests.
- Do not automatically write generated content into model weights.

Use transactions and appropriate constraints to avoid partially applied memory updates.

## 9. Provider abstraction

Create an asynchronous provider interface conceptually equivalent to:

```rust
trait ModelProvider {
    async fn generate(
        &self,
        request: &GenerationRequest,
    ) -> Result<GenerationResponse, ModelError>;
}

```

Adapt this to the selected async runtime and current stable Rust conventions.

The request should support:

- Prompt and bounded context.
- Inference parameters.
- Timeout and cancellation.
- Output limits.
- Optional seed where supported.

Responses should capture generated content and available usage metadata.

Start with one OpenAI-compatible endpoint. Keep credentials in environment variables or an appropriate local secret mechanism; never store them in experiment logs or source control.

Make provider capabilities explicit. Do not assume that every endpoint supports the same sampling parameters, seeds, usage metadata or deterministic behavior.

## 10. Proposed CLI

Implement equivalents of:

```bash
dreaming baseline --config experiments/baseline.toml

dreaming run --mode dream --input task.json

dreaming run --mode lucid \
  --case fixtures/seeded-errors/case-001.json

dreaming results --run RUN_ID

dreaming memory candidates

dreaming memory validate MEMORY_ID --experiment RUN_ID

dreaming benchmark compare \
  --baseline BASELINE_ID --run RUN_ID

```

These commands must be functional rather than decorative. If a capability is not implemented, report that clearly rather than returning simulated success.

## 11. Initial benchmark matrix

Build small, deterministic fixtures before attempting large-scale evaluations.

Benchmark arms:

A. Normal inference baseline.

B. Increased sampling diversity.

C. Dream mode.

D. Lucid Dream mode.

E. Dream plus independent Witness.

F. Lucid Dream plus independent Witness.

G. Memory enabled versus memory disabled.

Use comparable budgets where practical. Record cost, latency, configuration and outcomes. Repeat stochastic trials and report uncertainty.

For Dream, use tasks with independently checkable solutions requiring non-obvious associations.

For Lucid Dream, inject known errors into test cases and measure actual diagnostic accuracy.

For memory, test whether retained discoveries improve future held-out tasks without increasing regression rates.

## 12. Isolation and security requirements

- No automatic model-weight changes.
- No unrestricted execution of model-generated commands.
- No automatic network or filesystem access from exploration branches.
- No exposure of credentials to prompts or experimental memory.
- Enforce timeouts, token budgets, cancellation and bounded output.
- Treat model output and retrieved memory as untrusted data.
- Log decisions and memory promotions without logging secrets.
- Test that reset operations discard session state.
- Test that rejected findings never enter trusted memory.
- Test that a failed or cancelled run cannot leave a partially committed memory update.

Do not implement a general-purpose autonomous tool system for v0.1.

## 13. Implementation phases

Phase 1: Create the repository, workspace, documentation skeleton, CI, schemas and baseline tests.

Phase 2: Implement the provider interface, shared engine, session isolation, run logging and CLI.

Phase 3: Implement both Dream and Lucid Dream with shared candidate formats.

Phase 4: Implement Witness, seeded-error fixtures, benchmark execution and evaluation metrics.

Phase 5: Implement SQLite memory, versioning, admission rules and rollback.

Phase 6: Add the optional Split-Brain Harness integration interface, run full tests and document reproducible release procedures.

Both modes must exist in the first usable release. Keep their initial implementations small.

## 14. Development instructions

Start by:

1. Inspecting the current working directory and Git state.
2. Confirming whether a repository exists.
3. Producing a concise implementation plan mapped to the phases above.
4. Identifying major design uncertainties and the minimum dependencies needed.
5. Creating or updating the specification documents before implementing the core.

Then implement incrementally, with tests at every stage.

For each phase:

- Run formatting, linting and tests.
- Report what changed.
- Report actual test results.
- Identify incomplete features.
- Do not claim tests passed unless they ran.
- Do not overwrite unrelated files or repositories.
- Avoid unnecessary abstraction and dependency proliferation.
- Keep the code suitable for review and future research.

Do not stop after writing a plan if the workspace is ready for implementation. Proceed through the phases incrementally unless a missing credential, repository permission or genuine architectural blocker requires user input.

## 15. Definition of done

The v0.1 release must demonstrate:

- Both modes work through the shared engine.
- Baseline and experimental runs are recorded.
- Temporary session state is isolated and discarded.
- Protected model configuration remains unchanged.
- Candidates retain complete provenance.
- Dream is compared against ordinary inference.
- Lucid Dream is tested against known failure causes.
- Memory promotion is controlled, versioned and reversible.
- Regression outcomes are recorded.
- The project builds and tests without mandatory API credentials.
- The repository remains independent of the existing Split-Brain Harness.

The goal is not to make an AI appear to dream. The goal is to test whether structured, reversible exploration produces measurable improvements while keeping findings auditable and the baseline stable.

**First deliverable:** an implementation plan and repository skeleton with passing local tests. Then proceed incrementally toward the first working release.