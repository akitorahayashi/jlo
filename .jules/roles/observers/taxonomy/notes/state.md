# Taxonomy Role State

**Last Updated:** 2026-02-01

## Executive Summary
The repository exhibits several naming inconsistencies and terminology collisions, particularly in the `setup` and `scaffold` domains. While the core domain language is forming, strict boundaries between concepts like "Scaffold" vs "Template" and "Tools" vs "Setup" are currently blurred.

## Active Patterns
- **Primitive Obsession:** Frequent use of `String` for domain identifiers (`Component`, `Role`).
- **Generic Service Naming:** Services like `Resolver` and `Generator` claim global namespace terms without domain qualification.
- **Config/Type Mismatch:** Configuration files (`tools.yml`) often do not match their internal representation struct names (`SetupConfig`).

## Anti-Patterns Identified
- **Term Overloading:** "Template" is used for both file structure (scaffold) and content patterns.
- **Ambiguous Verbs:** "Setup" is used for both the command (`jlo setup`) and the artifact generation (`install.sh`), confusing the action with the output.
- **Vague Naming:** "Managed Defaults" fails to communicate "Integrity/Checksums".

## Vocabulary Glossary (Proposed)
| Concept | Current Term(s) | Proposed Canonical Term | Rationale |
| :--- | :--- | :--- | :--- |
| **Project Structure** | Scaffold, Template | **Scaffold** | "Scaffold" implies static structure. |
| **Reusable Patterns** | Template | **Template** | "Template" implies dynamic content. |
| **Component ID** | String | **ComponentId** | Type safety and domain clarity. |
| **Tools Configuration** | SetupConfig, tools.yml | **ToolsConfig** | Align with filename. |
| **Integrity File** | ManagedDefaults | **ScaffoldManifest** | Clarity of purpose. |
