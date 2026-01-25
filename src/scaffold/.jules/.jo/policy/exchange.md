# Inter-Role Exchange

## Purpose

The `exchange/` directory provides asynchronous communication between roles.

## Structure

```text
exchange/
  inbox/
    <role_id>/              # Incoming messages for a role
      YYYY-MM-DD_HHMMSS_<slug>.md
  threads/
    <thread_id>/            # Multi-message conversations
      README.md             # Thread context
      YYYY-MM-DD_HHMMSS_<slug>.md
```

## Inbox Messages

Inbox messages are short, targeted requests for another role.
They include context, the question, and expected response format.

## Threads

Threads capture multi-message conversations over time.
The thread `README.md` preserves the purpose and current status.

## Message Format

```markdown
# Message: <subject>

**From:** <source_role>
**To:** <target_role>
**Date:** YYYY-MM-DD HH:MM:SS UTC

## Context

<Background for this message>

## Request

<What is needed from the recipient>

## Expected Response

<Format or type of response needed>
```
