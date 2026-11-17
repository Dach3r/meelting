# Meelting Desktop

Espacio de trabajo con IA: el usuario arma una oficina virtual con agentes especializados que colaboran y ejecutan tareas reales de código en `git worktree` aislados, bajo su supervisión. Aplicación de escritorio 100 % local (sin servidor ni cuenta) para macOS, Windows y Linux.

Tauri 2 (núcleo Rust) + React 19 + TypeScript + Vite. Gestor de paquetes: `pnpm`.

## Desarrollo

```
pnpm install
pnpm tauri dev     # app completa
pnpm dev           # solo frontend en el navegador (sin APIs de Tauri)
pnpm tauri build   # instalador
```

Requiere Node, el toolchain de Rust y, en macOS, las Xcode Command Line Tools. Node y Rust deben coincidir con la arquitectura de la máquina.

Antes de commitear, desde `src-tauri`: `cargo fmt --check`, `cargo clippy --all-targets -- -D warnings` y `cargo test`; desde la raíz: `pnpm build`.

## Documentación

- [Producto](docs/product/README.md): visión, roadmap, usuarios, oficina virtual.
- [Arquitectura](docs/architecture/overview.md) y [estructura del proyecto](docs/architecture/project-structure.md).

## Licencia

[FSL-1.1-MIT](LICENSE): código disponible; cada versión pasa a MIT dos años después de su publicación.
