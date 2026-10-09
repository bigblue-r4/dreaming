# Dreaming

A standalone Rust v0.1 research prototype for reversible exploration and evidence-based failure diagnosis. One library and CLI; no dependency on Split-Brain Harness and no model-weight changes. Read [the specification](docs/specification.md) and [implementation plan](docs/implementation-plan.md).

Dream compares a baseline with independent interpretive and remote-association branches, optional validated-memory recombination, and synthesis. Branch proposals have structured connections, rationale, evidence claims, counterevidence, tests and uncertainty. Lucid reproduces a failure, compares clean context and named interventions, proposes a reusable prompt repair, and tests it on the original and supplied regression cases. Zero or multiple supported hypotheses remain inconclusive.

The Witness has an independent known-answer interface and an optional model assessment in a separate context. Its agreement never admits memory. SQLite records configurations, prompts, outputs, lineage, token metadata, metrics and outcomes. Candidate memory requires explicit operator evidence; promotion, retirement and rollback append transactional revisions.

## Run without credentials

```bash
cargo test --all-targets
cargo run -- --script fixtures/baseline-script.json baseline --input fixtures/known-solutions/bridge-001.json
cargo run -- --script fixtures/dream-script.json run --mode dream --input fixtures/known-solutions/bridge-001.json
cargo run -- --script fixtures/lucid-script.json run --mode lucid --case fixtures/seeded-errors/contamination-001.json
cargo run -- results --run RUN_ID
cargo run -- memory candidates
cargo run -- memory validate MEMORY_ID --experiment RUN_ID --evidence 'Independent operator review evidence'
cargo run -- memory retire MEMORY_ID --experiment RUN_ID --evidence 'Held-out regression failure'
cargo run -- memory rollback MEMORY_ID --experiment RUN_ID --version 1 --evidence 'Restore candidate revision'
cargo run -- benchmark compare --baseline BASELINE_ID --run RUN_ID
cargo run -- benchmark aggregate --runs RUN_ID_1,RUN_ID_2
```

Scripted providers replay **fixture responses**. Passing fixture tests demonstrates software behavior, not improved model reasoning. Repeat real trials before making research claims. The default database is `dreaming.sqlite`; override with `--db PATH`. These commands do not execute model-generated programs.

For a live endpoint, explicitly pass `--endpoint http://127.0.0.1:11434/v1/chat/completions` and `--config experiments/baseline.toml` (set its model to your installed model). The adapter reads optional `DREAMING_API_KEY` from the environment and never stores it. No live model/server is bundled. Seed support is conservative and off unless a library caller explicitly declares it. Token usage may be unavailable; monetary cost is reported unknown. No paid/provider calls are needed to build or test.

## Research controls and limits

Use `--mode diversity` with `fixtures/diversity-script.json` for the sampling control; its per-call parameters are recorded. Use `experiments/no-witness.toml` to disable model assessment, or `experiments/with-memory.toml` and `fixtures/dream-memory-script.json` for the memory arm. CLI calls can be repeated and aggregated; Wilson intervals operate on distinct matching trials, not correlated branches. Comparisons explicitly flag unequal call budgets.

`known_answer_improvements` counts a correct answer absent from an incorrect baseline. It does **not** establish semantic novelty; `validated_discoveries` remains zero pending independent novelty evaluation. Branch evidence is labeled unverified. Whether a proposed connection was already implicit in the baseline remains unknown unless directly present; a different answer or wording alone does not establish absence. Exact-answer tasks are the initial evaluation domain. General semantic novelty, external evidence retrieval, and automated memory admission are not implemented. There are no general-purpose autonomous tools, vector stores, distributed agents or automatic training.

Lucid uses operator-supplied interventions when present; otherwise it asks the provider for two to four competing hypotheses and single-factor intervention proposals. Model-proposed causes remain untrusted and are tested, never accepted as retrospective truth. A named factor is metadata, not proof that two prompts differ in only that factor. Operator review and controlled cases are necessary. The repair is applied to all supplied regression tasks; independently expected answers measure repair success. Interventions support hypotheses, not uniquely established causes.

Credentials belong only in the adapter environment. Obvious credential patterns in inputs/outputs are refused, but screening is not comprehensive DLP: use sanitized research cases. Logs contain task data and provider outputs. Endpoint access is intentional; experiment branches have no tool or host-access interface. Isolation discards in-memory orchestration state, not hidden provider state or OS processes. See [threat model](docs/threat-model.md).

## Development and release

```bash
cargo fmt --check
cargo clippy --all-targets -- -D warnings
cargo test --all-targets --locked
cargo build --release --locked
```

The package includes CI and a lockfile. Local Git initialization creates no commits; GitHub hosting/publishing is a separate action. Optional SBH integration is `Engine::run_harness(HarnessEnvelope, ...)`, which rejects escalated requests and uses data contracts only. It does not modify or execute the existing harness.
