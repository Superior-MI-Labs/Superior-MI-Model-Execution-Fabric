# ADR-0002: Pin the Qualified Builder R1 Commit

Status: accepted for R0.

MEF depends on Builder's qualified R1 source commit:

```text
82a5ed814a42dc9ca4e8c1540227f18d4f97e11e
```

This is the commit referenced by the annotated `builder-r1` tag. Pinning the exact commit prevents post-release documentation changes on `main` from silently changing MEF's structural dependency.
