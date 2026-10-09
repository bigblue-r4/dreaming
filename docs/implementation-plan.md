# Implementation plan

1. Standalone package, specification, CI, JSON contracts and baseline tests.
2. Provider trait, OpenAI-compatible HTTP adapter, cancellation, budgets, isolated session and SQLite run records; functional CLI.
3. Dream: baseline, independent interpretive and remote branches, memory recombination, synthesis; Lucid: original and clean-context reproduction, one-factor interventions, correction and regression.
4. Independent Witness and exact known-answer evaluator; deterministic scripted fixtures, recorded raw counts and cost. Explicit unknown results when evidence is insufficient.
5. Transactional, versioned candidate memory with explicit admission and rollback. Never use model agreement alone as admission evidence.
6. Serializable optional SBH boundary without dependency on its repository; format, lint, tests, smoke checks and release documentation.

Minimum dependencies: tokio, async-trait, reqwest, serde/serde_json, rusqlite (bundled SQLite), sha2, toml and clap. tempfile is test-only.

Uncertainties: semantic novelty needs independent domain criteria; provider behavior is stochastic; intervention evidence supports but does not establish unique causality; explicit validation is the initial admission policy. No sandboxed generated-code execution or model training is included.
