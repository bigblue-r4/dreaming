# Memory admission

Validation requires a completed evidence experiment and a nonempty operator evidence note. Source provenance remains immutable; evidence may reference a different held-out run. Operator notes are assertions, not automatic verification of semantic correctness or novelty.

| Current status | Allowed next status |
| --- | --- |
| candidate | validated, rejected, retired |
| validated | rejected, retired |
| rejected | retired; validated only with explicit override |
| retired | rollback to candidate first |
| any | rollback to a prior content revision as candidate |

Known-answer failures are initially rejected. `memory validate --override-known-answer` explicitly permits overriding rejection, including a failed known-answer check, and records that override. Rollback never restores validation or a validation timestamp; revalidation requires a separate operation. All changes append transactional revisions.

Retrieval uses validated entries only, excludes the source task identifier and identical source prompt even under another identifier, ranks concept overlap then revision recency, and takes at most eight entries. The memory branch records injected IDs. This exact-prompt filter cannot detect paraphrases or shared answers; dataset separation remains an operator responsibility. Memory remains data, never policy. SQLite schema 1 is migrated transactionally to schema 2, adding evidence-run and override provenance. No automatic weight changes.
