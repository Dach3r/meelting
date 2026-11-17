# Arquitectura

> **Estado:** Fase 0. Hoy existe el scaffold Tauri 2 (React 19 + TypeScript + Vite), el módulo `keychain` con los comandos `save_secret`, `secret_last_four` y `delete_secret`, y el spike `run_in_worktree` (git worktree + PTY con salida en vivo). La organización de carpetas está en [project-structure.md](project-structure.md). El resto de este documento describe la arquitectura **objetivo**; cada sección marca lo que ya existe.
>
> Este producto reemplaza al SaaS Rails (repositorio `meelting`), congelado como referencia de diseño: modelo de dominio, estados, errores, textos, plantillas y specs como casos de prueba.

## 1. Contexto y decisión

**Necesidad:** que los agentes completen tareas reales de código. Cada tarea necesita su propio `git worktree`, ejecutar comandos, levantar contenedores Docker y usar modelos de IA locales.

**Requisitos**
- Funciona **offline** y soporta **modelos locales** (Ollama, LM Studio, llama.cpp).
- Corre en **macOS, Windows y Linux**.
- Arranque rápido y poca memoria.
- **Sin API remota ni cuenta**: todo en la máquina del usuario.

**Alternativas evaluadas**

| Opción | Resultado |
|---|---|
| Rails en servidor + runner local | Descartada: no funciona offline y deja el cerebro fuera de la máquina |
| Rails empaquetado en Tauri/Electron (sidecar) | Descartada: distribuir Ruby y gemas nativas en tres SO, migrar a SQLite y gestionar procesos, sin ganar nada sin servidor |
| Cliente Tauri + API Rails remota | Descartada: sin servidor remoto no aporta; añade sync y dos codebases |
| **App de escritorio nueva, 100 % local** | **Elegida** |

## 2. Arquitectura

```
┌────────────────────────── Aplicación Tauri 2 ──────────────────────────┐
│  UI (webview)                       Núcleo (Rust)                       │
│  React + TypeScript           ←IPC→ Comandos y eventos Tauri            │
│  Tailwind (tokens Grafito)          ├─ Repositorios SQLite (sqlx)       │
│  Estado: stores + eventos           ├─ Motor de ejecución (AgentRun)    │
│                                     ├─ Proveedores LLM (reqwest + SSE)  │
│                                     ├─ Herramientas (fs, git, shell)    │
│                                     ├─ Terminal (PTY portable)          │
│                                     ├─ Docker (CLI/bollard, opcional)   │
│                                     └─ Secretos (keychain del SO)       │
└────────────────────────────────────────────────────────────────────────┘
        │ HTTP local            │ procesos hijo            │ socket Docker
   Ollama / LM Studio       git, shell, PTY             Docker Engine
```

### Stack

| Capa | Elección | Estado |
|---|---|---|
| Shell | Tauri 2 | ✅ scaffold |
| Núcleo | Rust (edición 2024) | ✅ scaffold |
| UI | React 19 + TypeScript + Vite; Tailwind v4 con los tokens Grafito | React/Vite ✅; Tailwind pendiente |
| Secretos | crate `keyring` 3 (Keychain, Credential Manager, Secret Service) | ✅ spike |
| Datos | SQLite con `sqlx`, modo WAL, migraciones `sqlx migrate` | pendiente |
| HTTP/streaming | `reqwest` + SSE (`eventsource-stream`) | pendiente |
| Terminal | `portable-pty` (ConPTY en Windows) | ✅ spike |
| Git | CLI `git` por `tokio::process` | ✅ spike |
| Docker | CLI `docker` por `tokio::process` (o `bollard`); opcional | pendiente |
| Tests | `cargo test`, Vitest, WebDriver (`tauri-driver`) | pendiente |

**Git y Docker se invocan como procesos, no con librerías:** `git2` (libgit2) no soporta `worktree` completo ni los hooks del usuario, y la CLI es lo que el usuario espera.

## 3. Capas del núcleo

- **Comandos Tauri** (`#[tauri::command]`, en `commands/`): delgados. Validan argumentos, llaman a un servicio y devuelven el resultado. Sin reglas de negocio ni SQL.
- **Servicios** (`services/`): acciones de negocio (verbos): contratar agente, crear sala, enviar instrucción, crear tarea. Reciben el workspace actual y validan la integridad entre tablas.
- **Repositorios**: acceso a SQLite; las transiciones de `AgentRun` son funciones atómicas en una transacción.
- **Catálogos**: colecciones de solo lectura embebidas en la app (`agent_templates/*.yml`, `llm.yml`), leídas con `serde_yaml`. Un catálogo nunca orquesta: la acción que lo usa es un servicio.
- **Integración con el SO** (`system/`): procesos hijo (`git`), PTY y keychain; sin reglas de negocio.
- **Dominio puro** (modelos, `BuildPrompt`, normalización de proveedores, path jail) separado del código de integración con el SO, para poder testearlo sin I/O.
- **Eventos** (`emit`) del núcleo a la UI: `agent_run:token`, `agent_run:status`, `message:created`, `tool_call:output`.

El webview **no recibe acceso directo a `fs` ni `shell`**: las capabilities de Tauri se declaran mínimas, solo los comandos propios.

Estructura de carpetas, responsabilidades y dónde ubicar código nuevo: [project-structure.md](project-structure.md).

## 4. Mapa desde el producto Rails

| Rails | Destino |
|---|---|
| `User`, Devise, onboarding de cuenta | Se elimina. Un `profile` local (nombre, idioma) y primer arranque |
| `Workspace`, `Team`, `Agent`, `Room`, `RoomAgent`, `Message` | Se mantienen |
| `AgentRun` y su máquina de estados | Se mantiene como tarea async en Rust |
| `LlmCredential` (Postgres, cifrada) | Keychain del SO; la tabla solo registra qué proveedores tienen clave |
| `AgentTemplate`, `LlmProvider`, `LlmModel` | Archivos embebidos `agent_templates/*.yml` y `llm.yml` |
| `Llm::Complete` + adaptadores | Trait `Provider` en Rust |
| `Llm::Error` (`Transient`/`Permanent`) | Enum `LlmError` con los mismos `kind` |
| Solid Queue, `AgentRunJob`, `ExpireStaleAgentRunsJob` | Tareas `tokio`; al arrancar, `running` huérfanas pasan a `failed` |
| Solid Cable, Turbo Streams, `RoomChannel` | Eventos Tauri |
| Controladores | Comandos Tauri |
| Rate limiting por IP/usuario | Se elimina; límites diarios opcionales |
| `config/locales/{es,en}.yml` | Base del JSON de i18n del frontend |
| Componentes ViewComponent (atomic design) | Componentes React con la misma jerarquía y nombres |
| Controladores Stimulus | Componentes y hooks (ver §8) |

## 5. Convenciones

- Nombres internos en **inglés**, `snake_case` en tablas y columnas.
- Texto visible traducido, nunca escrito en el código. Español usa «correo electrónico», no «email».
- Sin comentarios en el código; que se explique solo.
- **Sin código muerto:** todo método, comando, componente o constante tiene un uso real.
- Sin sobre-ingeniería: con dos o tres casos, un condicional directo.
- Validaciones reflejadas en la base de datos (índices únicos, claves foráneas, `CHECK`).
- Commits gitmoji + Conventional Commits, un cambio lógico por commit.
- Linters: `clippy -D warnings`, `rustfmt --check`, ESLint, `tsc`.
- No crear migraciones ni cambiar columnas sin presentar la propuesta y recibir aprobación.

## Modelo de datos

> **Propuesta.** Se presenta para aprobación antes de crear ninguna migración.

Reglas generales:
- IDs `TEXT` con **UUID v7** generados en la aplicación (ordenables, listos para un sync futuro).
- Fechas `TEXT` ISO-8601 UTC (`created_at`, `updated_at`).
- `PRAGMA foreign_keys = ON`, `journal_mode = WAL`.

| Tabla | Columnas | Notas |
|---|---|---|
| `profile` (una fila) | `id`, `first_name`, `last_name`, `locale` (`es`/`en`), `current_workspace_id` | Reemplaza `users` |
| `workspaces` | `id`, `name` (1–80), `canvas_x`, `canvas_y`, timestamps | |
| `teams` | `id`, `workspace_id`, `name` (1–80), `canvas_x`, `canvas_y`, timestamps | |
| `agents` | `id`, `workspace_id`, `team_id`, `template_key`, `name` (1–80), `role` (1–80), `description`, `instructions`, `provider`, `model`, `canvas_x`, `canvas_y`, timestamps | `team_id` `ON DELETE RESTRICT`; `provider`/`model` no vacíos |
| `rooms` | `id`, `workspace_id`, `name` (1–80), timestamps | |
| `room_agents` | `id`, `room_id`, `agent_id`, timestamps | Única `(room_id, agent_id)` |
| `messages` | `id`, `room_id`, `agent_id` (NULL = usuario), `body`, timestamps | `body` no vacío |
| `agent_runs` | `id`, `room_id`, `agent_id`, `request_message_id`, `response_message_id`, `client_request_id`, `status`, `error_kind`, `attempts`, `provider`, `model`, `response_model`, `input_tokens`, `output_tokens`, `cached_input_tokens`, `finish_reason`, `provider_request_id`, `started_at`, `finished_at`, timestamps | Los 7 `CHECK` del producto Rails. Únicas: `client_request_id` y una ejecución activa por `(agent_id, room_id)` (índice parcial `WHERE status IN ('queued','running')`) |
| `llm_providers_configured` | `provider`, `base_url`, `updated_at` | Qué proveedores tienen clave; la clave vive en el keychain |
| `tasks` | `id`, `workspace_id`, `room_id`, `agent_id`, `title`, `repo_path`, `base_branch`, `branch`, `worktree_path`, `status`, `created_at`, `updated_at`, `finished_at` | Fase 4. `status`: `preparing`, `ready`, `running`, `waiting_approval`, `done`, `failed`, `discarded` |
| `tool_calls` | `id`, `agent_run_id`, `task_id`, `tool`, `arguments` (JSON), `status`, `output`, `exit_code`, `created_at`, `finished_at` | Fase 5 |
| `approvals` | `id`, `tool_call_id`, `decision` (`pending`/`approved`/`denied`), `decided_at`, `remember` (`never`/`task`/`workspace`) | Fase 5 |
| `settings` | `key`, `value` | Límites diarios, ruta de Docker, shell preferido |

**Integridad entre tablas:** el aislamiento por workspace (un equipo, agente o participante debe pertenecer al mismo workspace) se valida en la capa de servicios y se prueba; donde sea posible se refuerza con claves foráneas compuestas `(id, workspace_id)`.

## 6. Motor de agentes y proveedores

Trait común:

```rust
#[async_trait]
trait Provider {
    async fn complete(&self, req: CompletionRequest, sink: &dyn TokenSink)
        -> Result<Completion, LlmError>;
}
```

| Adaptador | Protocolo | Notas |
|---|---|---|
| `openai` | Responses API, SSE | `instructions` + `input`; `reasoning.effort` solo si el modelo lo declara; `store: false`; `insufficient_quota` → `BillingFailed` |
| `anthropic` | Messages API, SSE | `system`, `messages`, `max_tokens`; `output_config.effort` solo si el modelo lo declara; texto = bloques `text` |
| `deepseek` | Chat Completions (`https://api.deepseek.com`) | Instrucciones como `system`; descartar `reasoning_content` |
| `minimax` | Anthropic-compatible (`https://api.minimax.io/anthropic`) | Mismo contrato que `anthropic`; nunca enviar `thinking`; `Authorization: Bearer` |
| `local` | Chat Completions compatible con OpenAI (Ollama `:11434`, LM Studio `:1234`) | Sin API key; modelos con `GET /v1/models`; sin `effort`; timeout largo |

Reglas:
- La clave solo se lee del keychain en el núcleo; **nunca llega a la UI, a logs ni a eventos**. No se registran prompts ni razonamiento.
- Un run **nunca cambia de proveedor ni de modelo** si falla.
- `max_retries = 0` en el cliente HTTP; solo el motor reintenta.
- Los errores no incluyen texto del proveedor: solo `provider`, `status` y `request_id`.
- Una respuesta que no es JSON o SSE válido es `EmptyResponse`.
- Modelos y proveedores viven **solo** en `llm.yml`; el código no nombra ningún modelo.

**Errores (`LlmError`):** transitorios `rate_limited`, `timeout`, `provider_unavailable`; permanentes `provider_not_configured`, `model_unavailable`, `provider_auth_failed`, `provider_billing`, `provider_rejected`, `content_filtered`, `output_truncated`, `invalid_response`. HTTP: 401/403 → auth, 402 → billing, 408 → timeout, 429 → rate limit (respetando `retry-after`), 5xx → unavailable, resto → rejected.

**Máquina de estados de `AgentRun`:** `queued → running → succeeded | failed`. Transiciones solo por funciones del repositorio, atómicas: `claim`, `requeue` (suma `attempts`), `succeed`, `fail_with(kind)`, `expire`. Hasta 3 intentos con espera creciente. Idempotencia por `client_request_id` único e índice parcial. Al iniciar la app, toda ejecución `running` pasa a `failed` con `error_kind = expired`. La cancelación usa `CancellationToken` y termina en `failed` con `error_kind = cancelled`. El texto parcial se acumula en memoria; solo se persiste el mensaje final.

**Contexto del prompt (`BuildPrompt`):** instrucciones del agente, objetivo de la sala y mensajes previos hasta 24 000 caracteres y 100 mensajes; los de otros se envían con su nombre.

## 7. Tareas, worktrees y herramientas

Ciclo de vida: el usuario crea una tarea desde una sala (agente, repositorio, rama base) → `preparing` valida `git` → se crea la rama `meelting/<slug>-<id corto>` y el worktree en `<datos de la app>/worktrees/<workspace>/<tarea>` con `git worktree add -b` → el agente trabaja solo dentro de `worktree_path` → el usuario revisa `git diff <base>...<rama>` y conserva la rama, descarta (`git worktree remove --force` + `git branch -D`) o abre la carpeta en su editor.

Reglas: varias tareas del mismo repo corren en paralelo en worktrees distintos; concurrencia por defecto 2 tareas activas por workspace; un reconciliador al arrancar compara `git worktree list` con `tasks` y marca huérfanos sin borrar nada; **nunca** `push`, `merge` ni modificar la rama base.

| Herramienta | Efecto | Aprobación |
|---|---|---|
| `read_file`, `list_dir`, `search` | Lectura dentro del worktree | No |
| `write_file`, `edit_file` | Escritura dentro del worktree | No (reversible con git) |
| `run_command` | Comando con cwd en el worktree | **Sí**, salvo lista permitida |
| `git_status`, `git_diff`, `git_commit` | Git acotado al worktree | `git_commit` no; nada de `push` |

**Seguridad (riesgo principal del producto):**
- **Path jail:** toda ruta se canonicaliza y debe quedar dentro del worktree; se rechazan `..`, symlinks que escapen y rutas absolutas externas (incluidas UNC y letras de unidad).
- **Aprobación humana** de `run_command` con el comando exacto.
- **Entorno saneado:** el proceso hijo no hereda `*_API_KEY`, tokens ni `SSH_AUTH_SOCK`.
- **Timeout y límite de salida** por comando; se mata el árbol de procesos.
- **Docker opcional** como aislamiento fuerte (worktree montado, sin red, usuario no root); sin Docker la app avisa que la ejecución no está aislada.
- Capabilities de Tauri mínimas.

La salida en vivo se emite como `tool_call:output` con límite de tamaño y se muestra en un panel por tarea.

**Compatibilidad entre sistemas**

| Tema | macOS | Linux | Windows |
|---|---|---|---|
| Shell de `run_command` | `zsh`/`bash` | `bash`/`sh` | `PowerShell` (configurable) |
| Keychain | Keychain | Secret Service (libsecret) | Credential Manager |
| Docker | Docker Desktop | Docker Engine | Docker Desktop (WSL2) |
| PTY | PTY | PTY | ConPTY |
| Webview | WKWebView | WebKitGTK | WebView2 |
| Git en Windows | — | — | `core.longpaths`, respetar `core.autocrlf` |

El webview de Linux es el menos predecible: se prueba en CI desde la Fase 0.

## 8. Interfaz

**Diseño Grafito:** tokens `ink`, `surface`, `shell`, `rail`, `raised`, `line`, `primary`, `sand` en `@theme` de Tailwind v4 (valores copiados de `app/assets/tailwind/application.css` del repositorio Rails). Iconos: `lucide`.

**Atomic design** en `src/components/<nivel>/<Nombre>`, con los mismos nombres que en Rails:
- **atoms:** `AgentAvatar`, `Button`, `Checkbox`, `Input`, `Label`, `Link`, `Logo`, `Select`, `StatusPill`.
- **molecules:** `AgentPicker`, `AgentRow`, `AgentRunStatus`, `AgentTile`, `Alert`, `CheckboxField`, `ConfirmDialog`, `ErrorSummary`, `FieldGroup`, `FormField`, `IdentityPill`, `InstructionsDocument`, `IslandMenu`, `LinkList`, `LocaleSwitcher`, `Message`, `MessageComposer`, `ModelSelect`, `NavItem`, `ParticipantTile`, `RoomCard`, `SettingsRow`, `TeamNode`, `Toast`, `WorkspaceSwitcher`.
- **organisms:** `FeaturedRoom`, `Rail`, `RoomChat`, `TeamTree`, `ToastRegion`, `Topbar`.
- **templates:** `Workspace`, `Settings`.
- **nuevos:** `TaskCard`, `TaskPanel`, `TerminalPanel`, `ApprovalDialog`, `ToolCallRow`, `DiffViewer`.
- No se migran: auth de Devise (`AuthInput`, `AuthField`, `AuthCard`, plantilla `Auth`); `UserMenu` se simplifica.

| Stimulus (Rails) | Frontend nuevo |
|---|---|
| `canvas` (organigrama arrastrable, líneas SVG, guardado con debounce 500 ms) | `TeamTree` con React Flow o SVG propio; posiciones vía comando Tauri |
| `composer`, `scroll_bottom`, `local_time` | Hooks de `MessageComposer` y `RoomChat` |
| `agent_picker`, `agent_wizard`, `llm_model` | `AgentPicker`, asistente de contratación, `ModelSelect` |
| `confirm`, `modal`, `drawer`, `disclosure`, `sidebar`, `toast`, `password_visibility` | Componentes equivalentes |

Pantallas: equipos (organigrama), contratar y editar agentes, catálogo, salas (lista, detalle, nueva), claves de IA, ajustes (perfil, idioma, workspace, Docker, shell, proveedor local), tareas y detalle de tarea con terminal y diff. Reglas: estados vacíos, errores y confirmaciones siempre presentes; nunca mostrar como disponible lo no construido.

## 9. Internacionalización

- Idiomas `es` y `en`; el valor por defecto de código es `en`; el idioma elegido se guarda en `profile.locale`.
- Se parte de `config/locales/{es,en}.yml` del repositorio Rails, filtrando claves de Devise y de funcionalidades eliminadas, convertidas a JSON anidado.
- Los textos de error de proveedor se resuelven en la UI a partir de `error_kind`; el núcleo no los envía.
- Las plantillas de agentes conservan sus YAML con contenido `es` y `en`.
- Una prueba falla si una clave existe en un idioma y no en el otro.

## 10. Claves y privacidad

- Claves en el keychain bajo el servicio `meelting` y el nombre `<workspace>/<provider>`. Nunca en SQLite, archivos de configuración ni logs.
- Se **verifican antes de guardar** con una llamada mínima al proveedor.
- La UI solo recibe el estado «conectado» y los últimos 4 caracteres.
- Ninguna telemetría ni llamada de red salvo a los proveedores configurados; con solo el proveedor local la app no sale a internet.
- Los datos viven en el directorio de datos de la app (`~/Library/Application Support`, `%APPDATA%`, `~/.local/share`) y el usuario puede exportarlos o borrarlos desde Ajustes.

## 11. Pruebas

| Nivel | Herramienta | Cubre |
|---|---|---|
| Núcleo Rust | `cargo test` | Repositorios y `CHECK`, máquina de `AgentRun`, `BuildPrompt`, proveedores con servidor simulado (`wiremock`), path jail, bucle de herramientas |
| Contrato de proveedores | Suite compartida | Mismos casos para los cinco adaptadores (éxito, tokens, cada `finish_reason`, cada error) sin APIs reales |
| Git | Repos temporales reales | Worktrees, huérfanos, rutas con espacios y Unicode |
| Componentes | Vitest + Testing Library | Cada componente del atomic design |
| E2E | `tauri-driver` en CI | Crear workspace, contratar agente, enviar instrucción con proveedor simulado, aprobar un comando, descartar una tarea |
| Proveedores reales | Opcionales, con variable de entorno | Nunca en CI por defecto; incluye Ollama |

Matriz de CI: macOS, Windows y Ubuntu con `cargo test`, `clippy -D warnings`, `rustfmt --check`, lint y tests del frontend. Los specs de Rails (`spec/services/llm`, `spec/models/agent_run_spec.rb`, `spec/catalogs`, `spec/requests`) se leen como lista de casos a portar: cada criterio de aceptación de [agent-execution.md](../product/agent-execution.md) debe tener una prueba equivalente. La política de cobertura (100 % en el dominio del núcleo, alta y medida en el resto) está pendiente de decisión.

## 12. Empaquetado y distribución

- Instaladores: `.dmg`/`.app` (macOS Intel y Apple Silicon), `.msi`/`.exe` (Windows), `.AppImage` y `.deb` (Linux), generados por el bundler de Tauri en GitHub Actions.
- **Firma:** macOS con Developer ID y notarización; Windows con certificado de firma de código. Iniciar los trámites temprano.
- **Actualizaciones:** `tauri-plugin-updater` con artefactos firmados (p. ej. GitHub Releases); opcional.
- Objetivos: instalador < 30 MB, arranque en frío < 2 s en hardware medio.
- Los modelos locales **no se incluyen**: la app detecta Ollama/LM Studio y guía la instalación.

## 13. Decisiones abiertas

| # | Decisión | Recomendación |
|---|---|---|
| 1 | Tauri o Electron | Tauri; Electron si la Fase 0 muestra problemas en Linux o falta dominio de Rust |
| 2 | Framework de UI | React (ya elegido en el scaffold; librería de grafos para el organigrama) |
| 3 | Datos del Rails actual | No migrarlos, salvo que existan datos reales |
| 4 | Política de cobertura | 100 % en el dominio del núcleo, alto en el resto, medido en CI |
| 5 | Límites diarios | Opcionales y desactivados para `local` |
| 6 | Cancelar ejecución | Incluirlo (imprescindible con modelos locales lentos) |
| 7 | Autoactualización | Opcional vía GitHub Releases firmados |
| 8 | Soporte mínimo de SO | macOS 12+, Windows 10+ (WebView2), Ubuntu 22.04+ |
| 9 | Concurrencia de tareas | 2 por workspace |

## 14. Riesgos

| Riesgo | Mitigación |
|---|---|
| Un agente ejecuta un comando destructivo | Worktree aislado, path jail, aprobación obligatoria, entorno saneado, Docker opcional, nunca `push`/`merge` |
| WebKitGTK o WebView2 se comportan distinto | Matriz de CI y E2E desde la Fase 0; plan B Electron |
| Rutas, shells y fin de línea en Windows | Pruebas de git con espacios y Unicode, `core.longpaths`, shell configurable |
| Modelos locales lentos o con poco contexto | Streaming, cancelación, tope de contexto configurable |
| Firma de código | Iniciar trámites en la Fase 1 |
| Reescribir la UI desde cero | Paridad medida contra los criterios de `docs/product/` y los specs de Rails |
| Curva de Rust | Fase 0 como prueba; dominio puro separado del código de integración |
| Scope creep hacia integraciones y colaboración | Mantener la exclusión explícita del roadmap |
