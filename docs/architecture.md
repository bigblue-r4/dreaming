# Architecture

One library and CLI, with modules for types, provider, engine, evaluation and store. Model configuration is immutable. Session context is local to a run and drops on completion/error/cancellation. SQLite keeps experiments and untrusted candidates separate from validated memory revisions. The engine never executes model output or grants branches tools. Provider access is the only network path.

Witness receives task criteria, observed outputs and independent evidence. Its opinion is recorded but cannot substitute for mechanical validation. SBH integration is a data-only request envelope, not execution authorization.
