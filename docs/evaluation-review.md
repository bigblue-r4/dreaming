# Evaluation review fixes

Lucid hypothesis generation no longer receives the expected answer. Intervention prompts, hypotheses, and repairs are refused if their trimmed, lowercased text contains the known answer; repairs also check all supplied regression answers. This conservative substring check may reject benign text and cannot prove semantic non-leakage. The contradiction fixture no longer supplies the expected answer as a distractor.

`intervention_repeats` defaults to 1 (range 1–10); `support_threshold` defaults to 1.0 (range 0.5–1.0). Single-sample support is labeled anecdotal. Repeated successes support a hypothesis without establishing unique causality. Clean-context failure makes context-contamination attribution inconclusive. Existing call and timeout budgets still bound repetitions.

Comparison and aggregation use the same final answer: synthesis, correction, baseline, or the last diversity sample. Branch correctness is reported separately. Best-of-N improvement is distinct from final-answer improvement. Total calls include witnesses; comparable-call-budget excludes witness calls. Witness usage remains charged to the resource budget. Aggregation also requires matching input, not merely matching task IDs.

`configuration_unchanged` describes immutable engine configuration preservation; it is not session-reset verification or a memory admission gate. Historical `reset_verified` and `known_answer_improvements` fields remain readable through deserialization aliases. Historical observation records default to no retrieved memory IDs; new call counters default to zero when absent.

Completed work wins over a cancellation signal arriving after the last response. Credential screening failures at final persistence retain a redacted failed record and spent-call metrics. Screening is conservative and incomplete; it is not a guarantee that arbitrary sensitive material can be detected.

All fixes are local to Dreaming. No live-model research claim, generated-code sandbox, SGAIL integration change, or automatic novelty validation is introduced.
