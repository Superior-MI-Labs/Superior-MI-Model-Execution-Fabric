# Environment Snapshot V1

`EnvironmentSnapshot` is immutable planning evidence. It is not System IR and is not a mutable machine registry.

Schema 1 contains:

- host OS and architecture;
- logical CPU count;
- optional total memory bytes;
- zero or more GPU observations;
- runtime observations with adapter name, optional executable path, and configured endpoint candidates;
- model-root observations with existence/directory flags;
- sorted diagnostic warnings.

Collections are canonicalized before serialization and hashing. SHA-256 identifies exact canonical snapshot bytes.

The snapshot deliberately does not claim that an endpoint is reachable or that a runtime can satisfy a capability. Those claims require provider-specific qualification evidence in later waves.
