# Evaluation review fixes

Lucid hypothesis generation no longer receives the expected answer. Intervention prompts, hypotheses, and repairs are refused if their trimmed, lowercased text contains the known answer; repairs also check all supplied regression answers. This conservative substring check may reject benign text and cannot prove semantic non-leakage. The contradiction fixture no longer supplies the expected answer as a distractor.

`intervention_repeats` defaults to 1 (range 1–10); `support_threshold` defaults to 1.0 (range 0.5–1.0). Single-sample support is labeled anecdotal. Repeated successes support a hypothesis without establishing unique causality. Clean-context failure makes context-contamination attribution inconclusive. Existing call and timeout budgets still bound repetitions.

Comparison and aggregation use the same final answer: synthesis, correction, baseline, or the last diversity sample. Branch correctness is reported separately. Best-of-N improvement is distinct from final-answer improvement. Total calls include witnesses; comparable-call-budget excludes witness calls. Witness usage remains charged to the resource budget. Aggregation also requires matching input, not merely matching task IDs.

`configuration_unchanged` compares the final session configuration with the initial configuration before session disposal; it is not session-reset verification or a memory admission gate. Diversity intentionally sets session temperature to 1.0, recorded in diversity observations (the baseline retains the initial temperature), so this field is false when the initial temperature differs. The engine configuration itself remains unchanged. Historical `reset_verified` and `known_answer_improvements` fields remain readable through deserialization aliases. Historical observation records default to no retrieved memory IDs; new call counters default to zero when absent.

Completed work wins over a cancellation signal arriving after the last response. Credential screening failures at final persistence retain a redacted failed record and spent-call metrics through one direct insert, without recursive screening. Screening is conservative and incomplete; it is not a guarantee that arbitrary sensitive material can be detected.

All fixes are local to Dreaming. No live-model research claim, generated-code sandbox, SGAIL integration change, or automatic novelty validation is introduced.

## Deferred review follow-ups

- Short known-answer substring checks can reject benign instructions; refine matching without weakening leakage detection.
- Repetition budgets are enforced during execution; add upfront feasibility checks to avoid spending calls on an impossible run.
- Concurrent schema-1 migration can safely fail with duplicate-column errors; serialize migrations with an immediate transaction and re-read schema version within it.
- The explicit rejection override currently permits overriding operator rejection too; restrict or rename it in a later change.
- Retrieval loads every validated memory and its source run; optimize queries and indexing before scaling.
