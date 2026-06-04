# open-ot-definition

Experimental definition-file layer for the OpenOT workspace. This crate models the hash-bound
ID-to-meaning file above the carriage layer and pins the canonical serialization and hash contract.

Implemented so far:

- typed definition-file content model;
- duplicate-key and no-float JSON parsing guardrails;
- canonical JSON bytes for hashing;
- SHA-256 content hash with 8-byte carriage binding;
- schema validation with placeholder outcomes;
- record resolution with current/prior epoch hash selection.
