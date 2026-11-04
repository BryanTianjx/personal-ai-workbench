# ADR-002: Modular Monolith

- **Status:** Accepted
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering
- **Implementation Status:** Not Implemented
- **Verification Status:** Not Verified

## Context
The initial desktop product needs clear domain boundaries without the operational and deployment cost of distributed services.

## Decision
Use a modular monolith for Alpha through V1.0. Modules own their domain concepts and writes, expose application contracts, and communicate through explicit events. Do not introduce microservices in the initial desktop product.

## Consequences
**Positive:** simpler packaging and local transactions, easier offline operation, lower operational complexity.
**Negative:** module boundaries require architecture tests and discipline; a monolith can become coupled if internal repositories leak.

## Validation
Architecture dependency tests, module ownership tests, and cross-module transaction tests are required before implementation is considered verified.

## Revisit when
Independent deployment/scaling or a separately operated service boundary is demonstrated by product requirements.
