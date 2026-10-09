# Threat model

Untrusted prompts, responses and retrieved memory may contain instructions. They remain user-role data under fixed system policy. No generated commands run. Configuration and credentials never enter run snapshots. Endpoint credentials are read only by the HTTP adapter; URLs with embedded credentials or query strings are rejected. HTTP errors omit server bodies; redirects are disabled. Response bytes, tokens, calls and wall time are bounded; cancellation discards uncommitted session state.

SQLite is a local trusted storage boundary, not protection against an attacker with OS write access. Explicit operator validation is required for promotion. Live provider requests disclose task data to the configured endpoint. This release provides orchestration isolation, not an OS process sandbox or access to hidden model internals.
