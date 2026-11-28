# Navigation sidebar and Inbox home: what is still pending

> Last updated: 2026-11-27 · Context: Phase 1 task "Navigation sidebar and Inbox home with placeholder data" (see "Authenticated home and navigation" in `product.md`).

The shell and the home are built. Everything below is deliberately left for the phase that brings the real data, or needs a decision first.

## Replace placeholder data

All placeholder data lives in `app/services/workspace/placeholder_data.rb` and is only reachable through `?preview=inbox|empty|no_repositories` in development. `Workspace::Build` is the single place that mixes it with real records.

| When | What to do |
| --- | --- |
| Phase 2 | Inbox "Merge" items come from open agent pull requests, and the empty-state "Assign an issue" link points to the repository's real open issues. Remove the `:merge` entry from `PlaceholderData::ITEMS`. |
| Phase 3 | Inbox "Approve" and "Failed" items, "In progress" runs and the sidebar counters (Inbox badge, Runs dot) come from runs, updated live with Turbo Streams. Remove `ITEMS`, `RUNS` and the `items` and `runs` methods. |
| Phase 1 (mirror) | The mirror dot per repository and the "mirror failing" problem come from the real mirror status. Today every real repository reports `:none`. |
| Phase 3 | The model key only shows up when it is needed: "Assign to agent" is disabled with a link to Settings while there is none. No banner and no Settings dot for the model key until then. |
| Phase 1 (hosting) | The push handler must call `Repository#touch_pushed`, or the repository page keeps showing its empty state (push URL, commands and "Generate token"). |
| Phase 3 | The "Docker is not responding" banner, once the sandbox can detect it. |
| Last placeholder removed | Delete `PlaceholderData`, `Previewable#preview_scenario`, the `?preview` tests and the "Datos de ejemplo" labels. |

## Links that still point to stand-ins

- Inbox row actions go to `/runs` (Approve, Failed) or `/repositories` (Merge). They should open the run page and the pull request page when those exist.
- Repository sections (Code, Issues, Pull requests, Runs, Memory), `/runs` and `/settings` are minimal pages that only say which phase builds them.
- Repositories and sections of placeholder repositories (preview only) link to `/repositories`.

## Configuration problems

- The "Docker is not responding" banner has its copy and its place, but nothing detects the problem yet (Phase 3, with the sandbox).
- The banner action always goes to `/settings`. Model keys are managed there; mirrors are not yet.

## Tests

- There is no browser test setup (no Capybara or Selenium). The home states, the sidebar, the expansion and the ignored `?preview` are covered by integration and component tests. Stimulus behavior (one repository expanded at a time, small-screen menu) was checked by hand.
- Decision needed: add Capybara with Selenium for real system tests. It is a new dependency, so it needs approval first.

## Decisions to confirm

- Account name and sign out live in the navbar user menu, not in the sidebar as the spec lists.
- The sidebar also appears on the other signed-in pages (repositories, access tokens).
- Copy is in Spanish (default locale) with English in sync. Which locales ship is still an open decision.
- Other signed-in pages (repositories, tokens) keep their old card style and are not aligned with the new look yet.

## Development gotcha

Running `bin/rails test` regenerates `public/assets` (git-ignored). While that folder exists, `bin/dev` serves stale CSS and JS. If the UI does not reflect a change, delete `public/assets` and restart the server.
