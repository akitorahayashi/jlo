# Issue: Stop Auto-Replacement in `jo template`

## Context
Currently, the `jo template` command automatically replaces `ROLE_NAME` in template files with the role ID provided via CLI arguments. This obscures which parts of the file are strictly coupled to the role identity and need configuration.
The goal is to prioritize clarity on "where dependencies exist" within the file. Explicit internal declarations (`role: ...`) and full paths (including role segments) must be preserved as they are critical for operation.

## Goal
Stop the automatic string replacement in `jo template` command. The command should only create the directory structure and copy the template files with placeholders intact. This ensures the user (or agent) can explicitly see and update the `ROLE_NAME` placeholders.

## Tasks
- [ ] **Modify `RoleTemplateStore`**: Remove `.replace("ROLE_NAME", ...)` logic from `generate_role_yaml` and `generate_prompt_yaml_template` in `src/services/role_template_service.rs`.
- [ ] **Verify Templates**: Ensure templates in `src/assets/templates/` use clear placeholders (currently `ROLE_NAME`) that are easy for users to identify and replace manually.
