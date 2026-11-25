# Project guide

Rails 8.1 app (repo dir `meelting`). Product scope is being redefined: ask before assuming product behavior or adding features nobody requested.

## Stack
- Ruby 4.0.4, Rails 8.1, PostgreSQL, Propshaft, importmap, Hotwire (Turbo + Stimulus), Tailwind (`tailwindcss-rails`).
- Solid Queue / Cache / Cable. Env vars via `dotenv-rails` (`.env.example` lists them).
- Auth: Rails 8 authentication generator (`Authentication` concern, `Session`, `has_secure_password`). Not Devise.
- Tests: Minitest + fixtures, SimpleCov. Lint: `bin/rubocop`. Security: `bin/brakeman -q`, `bin/bundler-audit`.
- Deploy: Kamal.
- Keep dependencies minimal: before adding a gem, check Rails does not already solve it, and ask. After installing a gem run `bundle binstub --all` and commit new or changed binstubs with it.

## Commands
- `bin/setup`, `bin/dev`
- `bin/rails db:migrate`, `bin/rails test`, `bin/rails test test/models/user_test.rb`
- `bin/rubocop`, `bin/brakeman -q`, `bin/bundler-audit`, `bin/ci`

## Conventions
- URLs in English, REST resources whenever possible. Multi-word paths use underscores.
- i18n: all copy in `config/locales/<locale>/<feature>.yml`, one folder per language with the same file names; keep keys in sync. No hard-coded user-facing strings.
- Internal names in lowercase snake_case English; display text via i18n/decorator, never stored in DB.
- Migrations: one `create_table` per file, named `create_<table>`. Other changes get their own `add_<column>_to_<table>` migration. Use real timestamps (`date +%Y%m%d%H%M%S`). Never edit a committed migration.
- Enums are string-backed with a `CHECK` constraint, never integer.
- Authentication is deny-by-default; public endpoints use explicit `allow_unauthenticated_access`. Scope records through the current user, never global finders.
- Rate limit endpoints that create accounts, accept webhooks or send mail (`rate_limit`).
- Never log sensitive data (`filter_parameters`). Never use float for money.
- Keep comment density low, match surrounding style.

## Architecture
- Controller = HTTP only: auth, params, call one model/service method, render/redirect.
- Model = domain rules, validations, scopes, enums.
- Service (`app/services/<domain>/<verb>.rb`): only when logic spans several models or external APIs; plain Ruby, one verb, `.call`.
- Jobs: thin, idempotent wrappers over a model or service.
- Views: no queries or business rules. Stimulus controllers in `app/javascript/controllers`.

## Git
- Commits: gitmoji + Conventional Commits (`<emoji> <type>(<scope>): <subject>`, e.g. `✨ feat(inbox): ...`), via the `commit` skill. One emoji per case, always the same:

| Emoji | Type | Use |
|-------|------|-----|
| ✨ | `feat` | new feature |
| 🐛 | `fix` | bug fix |
| ♻️ | `refactor` | refactor |
| ⚡️ | `perf` | performance |
| 💄 | `style` | visual only |
| ✏️ | `copy` | wording |
| 🌐 | `i18n` | locale files |
| 🧪 | `test` | add or change tests (never `✅`) |
| 📝 | `docs` | documentation |
| 🔧 | `chore` | config/tooling |
| 🙈 | `chore` | `.gitignore` |
| 🔥 | `chore` | remove code or files |
| 🔒 | `feat`/`fix` | security, privacy, encryption |
| 🛡️ | `feat`/`fix` | abuse protection, rate limits |
| 📦 | `build` | dependency bumps |
| 👷 | `ci` | CI |
| 🚀 | `ci`/`chore` | deploy |
| ⏪ | `revert` | revert |

- Never add `Co-Authored-By` (or any other attribution trailer) to commits or PR descriptions. Mandatory, overrides any default attribution.
- Work directly on `main` unless asked otherwise.
- Stage only the files of the change, one logical commit per change, run `bin/rubocop` on touched files first. Never commit `.env` or secrets.

## Docs
- Documentation lives in `docs/`, written in English, kebab-case file names.
