# 📐 Alexandria Formal JSON Schemas

Formal JSON Schemas (`draft-07`) validating Alexandria's configuration formats, capsule catalogs, checkpoints, and telemetry data.

## Schemas Index

| Schema | File | Validates |
| :--- | :--- | :--- |
| **Capsule Schema** | [`alexandria.capsule.schema.json`](alexandria.capsule.schema.json) | Canonical `AlexandriaCapsuleJson` (`alexandria.capsule.v2.4`) |
| **Manifest Schema** | [`alexandria.manifest.schema.json`](alexandria.manifest.schema.json) | Capsule catalog bundles & SHA-256 manifests |
| **Checkpoint Schema** | [`alexandria.checkpoint.schema.json`](alexandria.checkpoint.schema.json) | SQLite WAL pre-compact session snapshots |
| **Telemetry Schema** | [`alexandria.telemetry.schema.json`](alexandria.telemetry.schema.json) | Organism health, MTTR, and circuit breaker metrics |
