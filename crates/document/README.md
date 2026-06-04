# open-ot-document

Experimental document-format proposal for resolved OpenOT records.

This crate sits above `open-ot-carriage` and `open-ot-definition`. It does not re-parse the
wire record or re-run schema validation. It serializes definition-layer resolution outputs and
carriage loss events into deterministic JSON documents that preserve provenance, loss ranges,
placeholders, and private extension slots.

This is not a normative standard. It is an executable proposal with golden fixtures so the working
group can inspect and revise the document contract.
