# Navigation sidebar and Inbox home: what is still pending

> Last updated: 2026-11-27 · Context: Phase 1 task "Navigation sidebar and Inbox home with placeholder data" (see "Authenticated home and navigation" in `product.md`).

The shell and the home are built. Everything below is deliberately left for the phase that brings the real data, or needs a decision first.

## Replace placeholder data

All placeholder data lives in `app/services/workspace/placeholder_data.rb` and is only reachable through `?preview=inbox|empty|first_run` in development. `Workspace::Build` is the single place that mixes it with real records.

| When | What to do |
| --- | --- |
| Phase 2 | Inbox "Merge" items come from open agent pull requests, and the empty-state "Assign an issue" link points to the repository's real open issues. Remove the `:merge` entry from `PlaceholderData::ITEMS`. |
| Phase 3 | Inbox "Approve" and "Failed" items, "In progress" runs and the sidebar counters (Inbox badge, Runs dot) come from runs, updated live with Turbo Streams. Remove `ITEMS`, `RUNS` and the `items` and `runs` methods. |
| Phase 1 (mirror) | The mirror dot per repository and the "mirror failing" problem come from the real mirror status. Today every real repository reports `:none`. |
| Model keys | Done: the step is checked by a `valid` DeepSeek `ProviderCredential`. A rejected (`invalid`) key shows the Settings dot even during onboarding. An `unverified` key (timeout) is retried only by saving it again; there is no background re-check yet. |
| Phase 1 (hosting) | The push handler must call `Repository#touch_pushed`, or the step "create a repository and push to it" is never checked. The overview step reads `.project/overview.md` from the bare repo at `Repository#git_path` with `git cat-file` on each home request; move that check into the push handler, next to `touch_pushed`, and store the result on the repository. |
| Phase 2–3 | The step "assign your first issue to an agent" is shown disabled until issues and runs exist; enable it and check it on assignment. Until then, onboarding cannot complete with real data. |
| Phase 3 | Below the model key step, an automatic check that the Docker daemon is reachable. There is no Docker integration yet, so it is left out. |
| Last placeholder removed | Delete `PlaceholderData`, `Previewable#preview_scenario`, the `?preview` tests and the "Datos de ejemplo" labels. |

## Links that still point to stand-ins

- Inbox row actions go to `/runs` (Approve, Failed) or `/repositories` (Merge). They should open the run page and the pull request page when those exist.
- Repository sections (Code, Issues, Pull requests, Runs, Memory), `/runs` and `/settings` are minimal pages that only say which phase builds them.
- Repositories and sections of placeholder repositories (preview only) link to `/repositories`.

## Configuration problems

- The "Docker is not responding" banner has its copy and its place, but nothing detects the problem yet (Phase 3, with the sandbox).
- The banner action always goes to `/settings`. Model keys are managed there; mirrors are not yet.
- While the checklist is shown, the missing model key does not show the Settings dot or the banner: the checklist already covers it. Once onboarding is complete (`users.onboarding_completed_at`), it shows both.

## Tests

- There is no browser test setup (no Capybara or Selenium). The three states, the sidebar, the expansion and the ignored `?preview` are covered by integration and component tests. Stimulus behavior (one repository expanded at a time, small-screen menu) was checked by hand.
- Decision needed: add Capybara with Selenium for real system tests. It is a new dependency, so it needs approval first.

## Decisions to confirm

- Account name and sign out live in the navbar user menu, not in the sidebar as the spec lists.
- The sidebar also appears on the other signed-in pages (repositories, access tokens).
- Copy is in Spanish (default locale) with English in sync. Which locales ship is still an open decision.
- Other signed-in pages (repositories, tokens) keep their old card style and are not aligned with the new look yet.

## Development gotcha

Running `bin/rails test` regenerates `public/assets` (git-ignored). While that folder exists, `bin/dev` serves stale CSS and JS. If the UI does not reflect a change, delete `public/assets` and restart the server.
