# Code principles audit: DRY, KISS, YAGNI, Tell Don't Ask

> Last updated: 2026-11-27 · Scope: everything under `app/` and `config/routes.rb` (~2,000 lines), plus usages in `test/`. Nothing was changed; this is the list of findings to act on.

## Summary

| Principle | Score | Verdict |
| --- | --- | --- |
| DRY | 6/10 | Models are clean. Repetition lives in the older views and in the repository section list. |
| KISS | 7/10 | Thin controllers, simple models. The heaviest piece is the `Workspace` placeholder layer. |
| YAGNI | 5/10 | Weakest area: components, variants and methods only exercised by tests. |
| Tell, Don't Ask | 7/10 | Models guard their own state (`revoke!`, `archive!`, `touch_last_used`). Some views and controllers ask and decide outside. |

## DRY

1. **Repository sections are listed three times**: the route constraint regex (`config/routes.rb`), `Repositories::SectionsController::PHASES` and `Molecules::SidebarRepository::Component::SECTIONS`. Adding a section means touching all three. Keep one source.
2. **Checklist logic is duplicated**: `Organisms::SetupChecklist::Component#done_count` and `#current` repeat `Workspace::State#completed_steps` and `#current_step` (which the app never calls). `Workspace::Build#first_run?` computes the same thing again.
3. **Older views bypass the components.** `access_tokens/*` and `repositories/*` hand-write the `nb-card mx-auto ...` container, the `error_explanation` block (`Molecules::Notice` exists), the flash notice, the `nb-btn nb-btn-raised bg-brand-600 ...` button classes (4 times) and the cancel link. The auth views already use `FormField`, `Notice` and `SubmitButton`, so two styles coexist.
4. **The `rate_limit` line is near-identical** in the three auth controllers, including a hard-coded "Try again later." (also a breach of the i18n rule, like the other strings in `Auth::PasswordsController` and `passwords_mailer/*`).
5. **Repeated row and tray structures**: `Organisms::InProgress` hand-builds a row almost identical to `Molecules::InboxRow` (list item, avatar, title, chip). `settings/show` repeats the tray styling. `Molecules::PhaseNotice` and `settings/show` share the same 32px heading.
6. `Atoms::SubmitButton` repeats the classes of `Atoms::LinkButton` variant `primary` with size `block`.

## KISS

1. **`Workspace` is heavy for what it does**: seven `Data` classes plus `Build`, with per-scenario `case` branches (`Build#repositories`, `Build#problems`), for placeholders only visible in development. Acceptable as temporary scaffolding (see `navigation-and-inbox-pending.md`), but the hardest part to follow.
2. **The session is looked up twice per request**: `AuthenticatedConstraint` in `config/routes.rb` queries the database to pick the root, then the `Authentication` concern looks it up again.
3. **Wrappers that add nothing**: `Organisms::Sidebar::Component#current_page_in?` only calls `current_page?`. `SidebarRepository::MIRROR_TONES = { ok: :ok, failing: :bad, none: :none }` maps two of three values to themselves.
4. **`Organisms::Navbar` takes an `authenticated` boolean** and branches on it four times in its template: effectively two components in one.
5. `Atoms::LinkButton` has ten variants, and `secondary` and `graphite` are identical.

## YAGNI

Unused in `app/`, only referenced by their own tests:

- **Components**: `Molecules::Panel`, `Molecules::SectionHeading`, `Atoms::PillButton`, and `Atoms::Chip` (only used by `SectionHeading`).
- **`Atoms::LinkButton` variants**: `signal`, `deep`, `inverse`, `danger`, `graphite_soft`, `inverse_outline`. Sizes `sm`, `md`, `lg`.
- **Tones and sizes**: `Atoms::Link` `light`, `deep`, `plain`, `signal`. `Atoms::Button` `light`, `deep`, `pill`. `Molecules::Brand` tones `light`, `deep` and size `md`. `Atoms::AgentAvatar` sizes `sm`, `md`.
- **Model methods**: `Repository#archive!`, `#unarchive!`, `.archived`, `#touch_pushed` (no UI or push flow calls them). `AccessToken.authenticate` and `#active?` (no git or API endpoint yet).
- **`Workspace::State`**: `completed_steps`, `current_step`, `preview?`.
- `ApplicationHelper::CONFIG` is a one-key hash. i18n or `Rails.application.name` would do.

`AccessToken.authenticate` and `Repository#touch_pushed` are likely groundwork for git push over HTTP. Keep them only if that phase is confirmed; the rest looks left over from earlier designs.

## Tell, Don't Ask

1. **`home/show.html.erb`** asks `workspace.assign_issue_repository ? section_path : repositories_path` inline. The workspace or the component should hand over the resolved link.
2. **`access_tokens/index.html.erb`** branches `revoked? / expired? / else`. An `AccessToken#status` (`:revoked`, `:expired`, `:active`) lets the view just render it.
3. **`Navigation::ExpansionsController#update`** validates the slug outside the model, reading `Repository::SLUG_FORMAT` and checking the length. That is a domain rule: something like `Repository.valid_slug?(slug)`.
4. **`AccessToken.authenticate`** does `token&.scopes&.include?(scope)`. Prefer `token.allows?(scope)`.
5. **`Previewable#preview_scenario`** does `SCENARIOS.map(&:to_s).include?(scenario)`. Prefer `PlaceholderData.scenario_for(param)`.
6. **Decisions inside templates**: `Molecules::InboxRow` checks `item.kind == :failed` in ERB instead of carrying the icon colour in `KINDS`. `Organisms::SetupChecklist` calls `t(..., default: "").present?` in ERB.

Done well: `revoke!`, `archive!` and `touch_last_used` guard their own state; controllers tell the model what to do and never decide for it; every record is scoped through `Current.user`.

## Suggested order

1. **YAGNI**: remove unused components, variants and methods. Highest gain for the least effort. Confirm first whether `AccessToken.authenticate` and `Repository#archive!` stay for upcoming phases.
2. **DRY**: move `access_tokens` and `repositories` views onto the existing components; keep the section list in one place.
3. **Tell, Don't Ask**: add `AccessToken#status`, move the remaining decisions out of views (`home/show`, `InboxRow`, `SetupChecklist`).
4. **i18n**: translate the hard-coded strings in the auth controllers and the password mailer.
