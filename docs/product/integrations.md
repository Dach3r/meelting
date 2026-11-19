# Planned integrations

> **Status:** vision. No integration is planned for desktop v1.

The vision includes connecting Meelting to the everyday tools of technical teams, after the local flow has been validated. Roadmap: see [roadmap.md](roadmap.md), section "What NOT to build".

## GitHub

- Browse repositories, issues and pull requests.
- Analyze code changes and prepare proposals.
- Publish authorized branches or PRs. In v1 the app **never runs `push` or `merge`**: publishing is left to the user.

## Slack

- Read conversations, retrieve context and prepare replies.
- Post authorized messages.

## ClickUp

- Browse projects and tasks, create or update tasks, keep track of progress.

## Cross-cutting rule

Actions with external impact (publishing, creating, modifying, executing) require permissions and human approval. With only the local provider, the app never goes out to the internet.
