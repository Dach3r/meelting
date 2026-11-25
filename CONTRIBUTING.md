# Contributing to Meelting

Meelting is an early open source experiment. Scope is deliberately closed while the first milestone is built (see [`docs/product.md`](docs/product.md)), so please open an issue and discuss a change before sending a pull request.

## Development setup

Requirements: Ruby (see `.ruby-version`), PostgreSQL.

```sh
bin/setup
bin/dev
```

Run the checks before opening a pull request:

```sh
bin/rails test
bin/rubocop
bin/brakeman -q
bin/bundler-audit
```

Or run them all with `bin/ci`.

Technical conventions (architecture layers, i18n, migrations, naming) live in [`CLAUDE.md`](CLAUDE.md). Product decisions and what is in or out of scope live in [`docs/product.md`](docs/product.md).

## Commits

Commits use gitmoji plus Conventional Commits:

```
<emoji> <type>(<scope>): <subject>
```

Example: `✨ feat(inbox): add unread counter`. The full emoji and type table is in `CLAUDE.md`.

## Sign off your commits (DCO)

Meelting uses the [Developer Certificate of Origin](DCO) instead of a CLA. Every commit must carry a `Signed-off-by` line with your real name and email, which certifies that you wrote the change or have the right to submit it under the project license.

Add it automatically with:

```sh
git commit -s
```

Which produces:

```
Signed-off-by: Jane Doe <jane@example.com>
```

If you forgot, amend the last commit with `git commit --amend -s --no-edit`, or sign a whole branch with `git rebase --signoff main`.

## License

Meelting is licensed under [AGPL-3.0](LICENSE). By contributing you agree that your contribution is licensed under the same terms. There is no CLA, so relicensing later would need consent from every contributor.
