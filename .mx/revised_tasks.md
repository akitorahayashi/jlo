# jo CLI Revised Implementation Plan

## Overview

Transform the `rs-cli-tmpl` template into `jo`, a CLI tool that deploys and manages `.jules/` workspace scaffolding for organizational memory and workflow contracts.

## Commands to Implement

| Command | Alias | Description |
|---------|-------|-------------|
| `jo init` | `i` | Create `.jules/` skeleton and source-of-truth docs |
| `jo update` | `u` | Update jo-managed docs/templates under `.jules/.jo/` |
| `jo update --force` | `u -f` | Force overwrite jo-managed files |
| `jo status` | `st` | Print version info and detect local modifications |
| `jo role <role_id>` | `r` | Scaffold `.jules/roles/<role_id>/` workspace |
| `jo session <role_id> [--slug]` | `s` | Create new session file under role's sessions directory |

## Code Changes

### 1. Cargo.toml Updates
- Rename package from `rs-cli-tmpl` to `jo`
- Update description and metadata
- Add `include_dir` dependency for embedding bundle content

### 2. New Module: `src/bundle.rs`
- Embed static bundle content for `.jules/.jo/` directory
- Policy files: `contract.md`, `layout.md`, `run-bootstrap.md`, `run-output.md`, `role-boundaries.md`, `exchange.md`, `decisions.md`
- Template files: `session.md`, `decision.md`, `weekly-synthesis.md`
- `START_HERE.md` for root `.jules/` directory

### 3. Replace `src/storage.rs` with `src/workspace.rs`
- New `Workspace` struct for `.jules/` directory operations
- Methods for detecting, creating, and updating workspace structure
- Version marker management (`.jo-version`)
- Modification detection for jo-managed files

### 4. New Command Implementations

#### `src/commands/init.rs`
- Validate current directory is suitable (no existing `.jules/` or confirm overwrite)
- Create complete `.jules/` skeleton with:
  - `START_HERE.md`
  - `org/` directory with placeholder files
  - `decisions/` directory structure
  - `roles/` directory
  - `exchange/inbox/` and `exchange/threads/`
  - `synthesis/weekly/`
  - `state/` directory
  - `.jo/` with policy and templates
  - `.jo-version` marker

#### `src/commands/update.rs`
- Read current `.jo-version`
- Compare with current `jo --version`
- Detect modifications in `.jules/.jo/`
- Fail if modified without `--force`
- Update all jo-managed files under `.jules/.jo/`
- Write new `.jo-version`

#### `src/commands/status.rs`
- Print current jo version
- Print `.jo-version` from workspace (if exists)
- Detect and list modified files under `.jules/.jo/`
- Report if workspace is up-to-date or needs update

#### `src/commands/role.rs`
- Validate role_id format (alphanumeric + hyphens)
- Create `.jules/roles/<role_id>/` with:
  - `charter.md`
  - `direction.md`
  - `sessions/` directory

#### `src/commands/session.rs`
- Validate role exists
- Generate session filename: `YYYY-MM-DD/HHMMSS_<slug>.md`
- Create session file from template
- Print path to created session

### 5. Update `src/lib.rs`
- Remove old `add`, `list`, `delete` public functions
- Export new command functions: `init`, `update`, `status`, `role`, `session`
- Update module imports

### 6. Update `src/main.rs`
- Change binary name to `jo`
- Implement new CLI structure with subcommands
- Wire commands to library functions

### 7. Update `src/error.rs`
- Add error variants for jo-specific errors:
  - `WorkspaceExists`
  - `WorkspaceNotFound`
  - `ModifiedFiles(Vec<String>)`
  - `RoleNotFound(String)`
  - `InvalidRoleId(String)`

### 8. Delete Obsolete Files
- Remove `src/commands/add_item.rs`
- Remove `src/commands/list_items.rs`
- Remove `src/commands/delete_item.rs`
- Remove `src/config.rs` (replaced by workspace detection)

## Test Changes

### Integration Tests (`tests/`)

#### `tests/cli_commands.rs` (rewrite)
- `init_creates_jules_directory`
- `init_fails_if_jules_exists`
- `update_updates_jo_managed_files`
- `update_fails_if_modified_without_force`
- `update_force_overwrites_modified_files`
- `status_shows_versions`
- `status_detects_modifications`
- `role_creates_role_directory`
- `role_fails_for_invalid_id`
- `session_creates_session_file`
- `session_fails_for_nonexistent_role`
- `version_flag_works`
- `help_lists_visible_aliases`

#### `tests/cli_flow.rs` (rewrite)
- `user_can_init_create_role_and_session`
- `user_can_update_after_jo_upgrade`

#### `tests/commands_api.rs` (rewrite)
- Library API tests for each command

#### `tests/commands_core.rs` (rewrite)
- Error handling tests for edge cases

#### `tests/common/mod.rs` (update)
- Update binary name from `rs-cli-tmpl` to `jo`
- Add helpers for `.jules/` structure assertions
- Add version marker helpers

### Unit Tests in Commands
- Each command module includes `#[cfg(test)]` module
- Tests use mock workspace/filesystem as needed

## Documentation Updates

### `README.md` (complete rewrite)
- Project description for jo
- Installation instructions
- Command reference with examples
- `.jules/` directory structure documentation
- Development commands

### `AGENTS.md` (complete rewrite)
- Update project summary for jo
- Update tech stack
- Update testing strategy
- Update project structure

## Bundle Content to Embed

### Policy Files (`.jules/.jo/policy/`)

1. `contract.md` - Core workspace contract
2. `layout.md` - Directory structure reference
3. `run-bootstrap.md` - Agent bootstrap instructions
4. `run-output.md` - Output file conventions
5. `role-boundaries.md` - Role model guidance
6. `exchange.md` - Inter-role communication
7. `decisions.md` - Decision logging conventions

### Templates (`.jules/.jo/templates/`)

1. `session.md` - Session file template
2. `decision.md` - Decision record template
3. `weekly-synthesis.md` - Weekly synthesis template

### Root Files

1. `START_HERE.md` - Entry point for the `.jules/` directory

## Implementation Order

1. Update Cargo.toml
2. Create bundle content module with embedded files
3. Rewrite error.rs with jo-specific errors
4. Create workspace.rs for filesystem operations
5. Implement init command
6. Implement update command
7. Implement status command
8. Implement role command
9. Implement session command
10. Update mod.rs with new command exports
11. Update lib.rs with public API
12. Update main.rs with CLI structure
13. Delete obsolete files
14. Rewrite test files
15. Update documentation
16. Run tests and verify
