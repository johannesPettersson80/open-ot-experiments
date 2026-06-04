# Roadmap

This is exploratory work to help the OpenOT working group shape the standard; once it
stabilizes, a clean implementation follows from what these experiments establish.

OpenOT has three standards-facing contracts, plus supporting registry and authoring work. This
workspace implements the three standards-facing contracts as experimental prototypes and keeps the
provisional canonical registry inside `carriage`. The remaining planned work is engineer-facing
authoring and controller-language experiments.

## Status

| Contract / layer | Crate | Status |
| --- | --- | --- |
| Wire + carriage (records, ring buffer, loss accounting, epochs, concurrency) | `carriage` | Implemented + tested (experimental prototype) |
| Definition file (hash-bound map from ids to meaning) | `definition` | Content model, canonical hash, schema validation, and resolver implemented |
| Document format (resolved consumer-facing output) | `document` | Proposed JSON shape, serializer, and golden fixtures implemented |
| Canonical registry (event / key / enum / type ids) | `carriage::registry` | Implemented as provisional tables |
| Engineer-facing authoring workflow | `authoring` (planned) | Not started |

## Planned crates

### Registry

Canonical event-type, value-key, enum, and type identifiers live in `carriage::registry` as one
provisional table. A later standalone crate is still possible if the workspace needs to share the
table without depending on carriage internals.

### `definition`

The hash-bound definition file: a schema mapping ids to names, types, units, and enums; a
parser; canonical serialization and the hash that binds a definition to a record stream; and
the resolver that turns a decoded record into typed, named fields. The current crate implements
the typed content model, duplicate-key/no-float JSON guardrails, canonical bytes for hashing,
the SHA-256/8-byte binding, schema validation with placeholder outcomes, and record resolution with
current/prior epoch hash selection.

### `document`

The consumer-facing document: the output schema for resolved events, loss ranges, and placeholders,
with exact JSON fixtures. This is a proposed third contract because doc-format is undefined upstream.

### `authoring`

The engineer-facing workflow: generating and maintaining a definition file from the control
program's symbol table, so the id-to-meaning mapping is derived from code rather than kept by
hand. This is the usability layer — the hardest part to get right and the one that decides
whether the standard is actually adopted in the field.

## Sequencing

Build the `authoring` workflow next, then controller-language experiments. Each layer lands with its
own tests and conformance vectors, and the whole workspace stays green at every step.
