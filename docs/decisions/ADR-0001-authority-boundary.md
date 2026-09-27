# ADR-0001: Builder Remains Structural Authority

Status: accepted for R0.

MEF may observe, qualify, plan, and execute. It may not create a second SystemGraph, capability registry, or hidden structural mutation path.

Any deployment realization that affects system structure must eventually be represented through ordinary qualified Builder definitions and `GraphDelta`.
