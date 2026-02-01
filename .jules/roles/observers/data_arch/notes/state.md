# Data Architecture State

## Overview
The project currently exhibits moderate coupling between domain and infrastructure concerns, particularly in the `setup` module and the public API surface. While `ports` and `services` abstractions exist, they are not strictly enforced at the library boundary or within the domain models themselves.

## Key Findings
- **Mixed Concerns**: `src/domain/setup.rs` conflates domain entities with DTOs and configuration parsing logic.
- **Inefficiency**: Dependency resolution involves deep cloning of potentially large script data.
- **Leaky Abstractions**: The top-level `lib.rs` exposes internal implementation details (`app`, `services`, `ports`).
- **Primitive Obsession**: Error handling and domain identifiers rely heavily on raw `String` types, bypassing type system safety.

## Domain Purity
The `setup` domain is the most significant violator of boundary sovereignty. The `Component` entity is tightly coupled to the `ComponentMeta` DTO structure (via shared sub-structs like `EnvSpec` and co-location).

## Data Flow Efficiency
The `Resolver` service performs unnecessary work by carrying full script content during graph traversal. This suggests a missing "lightweight" metadata representation of components for the resolution phase.

## Public Surface
The library architecture (Hexagonal/Ports & Adapters) is compromised by `pub mod` exports in `lib.rs`, effectively flattening the architecture for consumers.
