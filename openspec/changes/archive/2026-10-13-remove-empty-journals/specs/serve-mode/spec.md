# Spec Delta

## ADDED Requirements

### Requirement: Removing empty journal days
When it starts and before every rebuild, the server SHALL delete each journal day file (`journals/YYYY-MM-DD.md` with a valid date, a regular file and not a symbolic link) whose content is empty — only blank lines, or a single bullet without text (`-`) and nothing else — and that has not been modified for at least one hour. The file SHALL be deleted, not moved to the trash, and the deletion SHALL be reported on standard error. Other files SHALL NOT be deleted by this rule. The deletion SHALL be part of that rebuild and SHALL NOT cause a second one.

#### Scenario: Left empty
- **WHEN** `journals/2026-10-12.md` contains only `-`, has not changed for two hours, and the author saves another page
- **THEN** `journals/2026-10-12.md` no longer exists and the journal list and timeline do not show that day

#### Scenario: Just written elsewhere
- **WHEN** another editor has just saved `journals/2026-10-13.md` with only `-` and the server rebuilds
- **THEN** `journals/2026-10-13.md` still exists

#### Scenario: Content kept
- **WHEN** `journals/2026-10-01.md` contains `- standup` or two empty bullets, or `pages/empty.md` contains `-`
- **THEN** none of them is deleted
