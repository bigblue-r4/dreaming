# Local v0.1 release procedure

Run cargo fmt --check, cargo clippy --all-targets -- -D warnings, cargo test --all-targets --locked, and cargo build --release --locked. Use the fixture commands in README with a disposable database. Test live inference only against an explicitly approved endpoint and sanitized tasks. Keep generated databases out of Git. Review memory admissions and evidence; do not infer model quality from scripted tests. No remote repository or publishing is required for local use.

Known boundaries: domain-independent semantic novelty and automatic admission are deferred. Interventions are operator-defined or untrusted model proposals. One independently scoped model assessment supplements mechanical Witness checks; it does not prove truth. Monetary pricing is not assumed. Optional SBH integration remains data-only.
