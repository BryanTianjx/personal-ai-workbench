# ADR-002: Adopt a Modular Monolith

- **Status:** Accepted for V1 planning
- **Date:** 2026-11-04
- **Decision owners:** Product and engineering

## Context

The product spans workspace, projects, tasks, Agent execution, knowledge references, creator workflows, and security. These domains need clear boundaries, but microservices would add deployment, observability, networking, and consistency costs before independent scaling needs are demonstrated.

## Decision

Use a modular monolith for V1. Keep domain/application modules explicit, enforce inward dependency direction, and isolate external systems behind adapters. A single desktop application process is the default runtime.

## Consequences

**Positive:** simpler offline installation, local transactions, debugging, packaging, and support.

**Negative:** requires discipline to prevent cross-module table coupling; limits independent scaling and fault isolation.

## Revisit when

A module needs independent deployment/scaling, a separate trust boundary, or independent lifecycle for a demonstrated product need. Extraction must be justified by measured constraints and preserve domain contracts.
