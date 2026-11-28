# Code principles audit: DRY, KISS, YAGNI, Tell Don't Ask

> Last updated: 2026-11-28 · Scope: everything under `app/` and `config/routes.rb`, plus usages in `test/`. Re-checked against `main` after the forest/lime design migration, the onboarding checklist and the repository wizard. The YAGNI findings are done; the rest is the list of findings to act on.

## Summary

| Principle | Score | Verdict |
| --- | --- | --- |
| DRY | 7/10 | Models are clean and most views use the components now. Repetition remains in the repository section list and a few hand-styled buttons. |
| KISS | 7/10 | Thin controllers, simple models. The heaviest piece is the `Workspace` placeholder layer. |
| YAGNI | 9/10 | Unused components, tones, variants and model methods removed. Only intentional Phase 1 groundwork remains. |
| Tell, Don't Ask | 7/10 | Models guard their own state (`revoke!`, `touch_last_used`). Some views and controllers still ask and decide outside. |

## DRY

1. **Repository sections are listed three times**: the route constraint regex (`config/routes.rb`), `Repositories::SectionsController::PHASES` and `Molecules::SidebarRepository::Component::SECTIONS`. Adding a section means touching all three. Keep one source.
2. **Hand-styled buttons and alerts next to existing components**:
   - `repositories/new`: the `error_explanation` box is hand-written (`Molecules::Notice` with `tone: :alert` does this, as in `access_tokens/new` and `provider_credentials/new`), and the "back" button repeats the `secondary` + `block` classes of `Atoms::LinkButton`.
   - `access_tokens/new`: the submit uses `form.submit` with hand-written primary classes instead of `Atoms::SubmitButton`.
   - `access_tokens/index`: the revoke `button_to` hand-writes the `danger` styling.
3. **The `rate_limit` line is near-identical** in `Auth::SessionsController`, `Auth::RegistrationsController` and `Auth::PasswordsController`, each with a hard-coded "Try again later." (also a breach of the i18n rule, like the other strings in `Auth::PasswordsController` and `passwords_mailer/*`). `ProviderCredentialsController` already uses a translated message and is the pattern to follow.
4. `Atoms::SubmitButton` hard-codes the primary/block look instead of reusing the `nb-btn` base and the `LinkButton` `primary` and `block` classes, so the two can drift.

## KISS

1. **`Workspace` is heavy for what it does**: six `Data` classes plus `Build` and `PlaceholderData`, with per-scenario branches, for placeholders only visible in development. Acceptable as temporary scaffolding (see `navigation-and-inbox-pending.md`), but the hardest part to follow.
2. **The session is looked up twice per request**: `AuthenticatedConstraint` in `config/routes.rb` queries the database to pick the root, then the `Authentication` concern looks it up again.
3. **Wrappers that add nothing**: `Organisms::Sidebar::Component#current_page_in?` only calls `current_page?`. `SidebarRepository::MIRROR_TONES = { ok: :ok, failing: :bad, none: :none }` maps two of three values to themselves.
4. **`Organisms::Navbar` takes an `authenticated` boolean** and branches on it in its template: effectively two components in one.

## YAGNI

Done on 2026-11-28: removed `Atoms::PillButton`, `Atoms::AgentAvatar`, `LinkButton` `danger`, `Link` `light` (`tone:` is now required), `Repository#archive!`/`#unarchive!`/`#archived?`/`.archived` (the `archived_at` column and `.active` stay), `StatusChip` tones `ok` and `run`, `LinkButton` `accent`, `AccessToken#active?`, `Workspace::State#preview?`, `#completed_steps` and `#current_step` (they duplicated `Organisms::SetupChecklist`), and replaced `ApplicationHelper::CONFIG` with `ApplicationHelper::APP_NAME`. `Button` `pill` stays: `Molecules::PushCommands` uses it.

**Not YAGNI, keep**: `Repository#touch_pushed` and `AccessToken.authenticate` have no caller yet, but they are Phase 1 groundwork. The repository page reads `pushed_at` for its empty state, and `docs/product.md` requires the push handler to call `touch_pushed`. `Repositories::PushTokensController` already issues `git_read`/`git_write` tokens that `authenticate` will check.

## Tell, Don't Ask

1. **`home/show.html.erb`** asks `workspace.assign_issue_repository ? repository_section_path(...) : repositories_path` inline. The workspace or `InboxList` should hand over the resolved link.
2. **`access_tokens/index.html.erb`** branches `revoked? / expired? / else`. An `AccessToken#status` (`:revoked`, `:expired`, `:active`) lets the view render it.
3. **`Navigation::ExpansionsController#update`** validates the slug outside the model, reading `Repository::SLUG_FORMAT` and checking the length. That is a domain rule: something like `Repository.valid_slug?(slug)`.
4. **`AccessToken.authenticate`** does `token&.scopes&.include?(scope)`. Prefer `token.allows?(scope)`.
5. **`Previewable#preview_scenario`** does `SCENARIOS.map(&:to_s).include?(scenario)`. Prefer `PlaceholderData.scenario_for(param)`.

Done well: `revoke!` and `touch_last_used` guard their own state; controllers tell the model what to do and never decide for it; every record is scoped through `Current.user`; `Molecules::InboxRow` carries icon, tone and button per kind in `KINDS` instead of branching in the template.

## Suggested order

1. **DRY**: keep the section list in one place; move the remaining hand-styled buttons and the `repositories/new` alert onto the components.
2. **Tell, Don't Ask**: add `AccessToken#status`, `Repository.valid_slug?`, and resolve the assign link outside `home/show`.
3. **i18n**: translate the hard-coded strings in the auth controllers and the password mailer.
