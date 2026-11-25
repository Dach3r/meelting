# Meelting

An open source, self-hostable code collaboration platform where humans and AI agents understand, build and maintain software together.

Meelting keeps the project's knowledge in the repository (decisions, conventions and constraints as versioned Markdown), lets you control what each agent may do, and records everything an agent does.

> **Status:** early experiment. Phase 0 of the first milestone is in progress. Nothing here is stable yet. See [`docs/product.md`](docs/product.md) for the vision, decisions and roadmap.

## Self-hosting with Docker Compose

Requirements: Docker with Compose.

```sh
cp .env.example .env
# edit .env: set SECRET_KEY_BASE (openssl rand -hex 64) and MEELTING_DATABASE_PASSWORD
docker compose up -d
```

The app listens on `http://localhost:3000` (change it with `PORT`). Set `FORCE_SSL=true` and `APP_HOST` when a reverse proxy terminates TLS in front of it.

## Development

Requirements: Ruby (see `.ruby-version`), PostgreSQL.

```sh
bin/setup
bin/dev
```

Run the test suite and linters with `bin/ci`, or individually: `bin/rails test`, `bin/rubocop`, `bin/brakeman -q`, `bin/bundler-audit`.

## Contributing

Read [`CONTRIBUTING.md`](CONTRIBUTING.md). Commits must be signed off under the [DCO](DCO).

## License

[AGPL-3.0](LICENSE).
