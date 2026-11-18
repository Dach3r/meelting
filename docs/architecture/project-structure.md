# Estructura del proyecto

> **Estado:** Fase 1 completada. Solo existen los módulos listados aquí como «✅». Lo marcado «pendiente» es la ubicación **prevista**; no se crea la carpeta hasta que haya código que la use (sin código muerto).

Reglas de capas y convenciones: [overview.md](overview.md) §3 y §5.

## Vista general

```
meelting-desktop/
├── CLAUDE.md              Instrucciones del proyecto (inglés)
├── LICENSE                FSL-1.1-MIT
├── README.md
├── docs/
│   ├── product/           Visión, roadmap, usuarios, oficina virtual, salas, agentes
│   └── architecture/      overview.md y este documento
├── .github/workflows/     CI en macOS, Windows y Ubuntu
├── index.html             Entrada de Vite
├── package.json           Scripts y dependencias del frontend (pnpm)
├── src/                   Frontend (React + TypeScript)
└── src-tauri/             Núcleo (Rust + Tauri 2)
```

## Núcleo Rust (`src-tauri/src`)

La dependencia va siempre hacia abajo: `commands → services → system`. Ninguna capa importa de una superior.

```
src-tauri/src/
├── main.rs                Arranque; solo llama a meelting_lib::run()
├── lib.rs                 Builder de Tauri, plugin de estado de ventana y registro de comandos
├── commands/              Capa 1: comandos Tauri (delgados)
│   ├── keychain.rs        ✅ save_secret, secret_last_four, delete_secret
│   ├── worktree.rs        ✅ run_in_worktree
│   ├── completion.rs      ✅ stream_completion
│   └── session.rs         ✅ load_session, complete_first_launch, create_workspace, switch_workspace, list_teams, create_team, create_agent, list_agent_templates, list_agent_roles, change_locale, change_theme
├── services/              Capa 2: acciones de negocio (verbos)
│   ├── worktree_run.rs    ✅ crea un worktree y ejecuta un comando en él
│   ├── completion_stream.rs ✅ lee la clave del keychain y hace streaming de una respuesta
│   ├── session_load.rs    ✅ carga el perfil y los workspaces; define SessionError
│   ├── first_launch.rs    ✅ crea el perfil y el primer workspace en una transacción
│   ├── workspace_create.rs ✅ crea un workspace y lo deja como actual
│   ├── workspace_switch.rs ✅ cambia el workspace actual
│   ├── teams_list.rs      ✅ equipos del workspace actual con sus agentes
│   ├── locale_change.rs   ✅ guarda el idioma del perfil
│   ├── theme_change.rs    ✅ guarda el tema del perfil
│   ├── team_create.rs     ✅ crea un equipo y devuelve la lista actualizada
│   ├── agent_create.rs    ✅ valida y crea un agente (personalizado o desde plantilla) y devuelve la lista actualizada
│   ├── agent_roles_list.rs ✅ roles predefinidos del catálogo, agrupados por área
│   └── agent_templates_list.rs ✅ plantillas del catálogo en el idioma del perfil
├── system/                Capa 3: integración con el SO, sin reglas de negocio
│   ├── git.rs             ✅ git por tokio::process (worktree add)
│   ├── terminal.rs        ✅ PTY portable y lectura de salida
│   ├── chat_completions.rs ✅ cliente HTTP con SSE (Chat Completions)
│   └── keychain.rs        ✅ keychain del SO (crate keyring)
├── repositories/          Acceso a SQLite (sqlx); las migraciones viven en src-tauri/migrations
│   ├── database.rs        ✅ abre el pool (WAL, claves foráneas) y aplica migraciones
│   ├── profile.rs         ✅ consultas de la tabla profile
│   ├── workspaces.rs      ✅ consultas de la tabla workspaces
│   ├── teams.rs           ✅ consultas de la tabla teams
│   └── agents.rs          ✅ consultas de la tabla agents
├── domain/                Modelos y lógica pura, sin I/O
│   ├── locale.rs          ✅ idioma es/en
│   ├── theme.rs           ✅ tema system/light/dark
│   ├── profile.rs         ✅ perfil local
│   ├── session.rs         ✅ perfil + workspaces que recibe la UI
│   ├── team.rs            ✅ equipo y agente (resumen) que recibe la UI; AgentDraft y su validación
│   └── workspace.rs       ✅ workspace y validación del nombre
└── catalogs/              Colecciones de solo lectura embebidas en la app
    ├── agent_roles.rs     ✅ carga los roles predefinidos de agente (YAML) y valida que un rol exista
    ├── agent_roles.yml    ✅ roles agrupados por área
    ├── agent_templates.rs ✅ carga las plantillas de agente (YAML es/en) y valida claves
    ├── agent_templates/   ✅ una plantilla por rol predefinido (25 .yml, p. ej. product_owner, qa_engineer, legal_advisor)
    └── llm.yml            pendiente
```

### Responsabilidad de cada capa

| Capa | Qué hace | Qué no hace |
|---|---|---|
| `commands` | Recibe argumentos de la UI, resuelve lo propio de Tauri (`AppHandle`, rutas de la app), llama a un servicio, emite eventos y devuelve el resultado | Reglas de negocio, SQL, procesos |
| `services` | Orquesta una acción completa y define su error serializable para la UI | Conocer Tauri (`AppHandle`, eventos): recibe *callbacks*, no emisores |
| `system` | Habla con el SO: procesos hijo, PTY, keychain. Cada módulo tiene su error propio | Decidir qué hacer ni serializar errores hacia la UI |
| `repositories` | Consultas SQLite y transiciones atómicas | Reglas entre tablas (van en servicios) |
| `domain` | Lógica pura testeable sin I/O (prompt, normalización, path jail) | Tocar disco, red o procesos |

### Convenciones del núcleo

- **Un error por módulo** con `thiserror`. Solo se implementa `Serialize` en el error que cruza hacia la UI (el del servicio o el de `keychain`), como cadena.
- **Comandos de secretos sin servicio:** `commands/keychain.rs` llama a `system/keychain.rs` directamente porque aún no hay regla que orquestar. Cuando exista la verificación previa a guardar (Fase 3), pasa a un servicio.
- **Errores hacia la UI como código:** `SessionError` se serializa como un código estable (`invalid_name`, `invalid_agent`, `workspace_not_found`, `team_not_found`, `not_set_up`, `database`); la UI lo traduce. No viaja el detalle de la base de datos.
- **Base de datos:** el pool se abre en `setup` (`app_data_dir()/meelting.db`) y se registra como estado de Tauri; los comandos lo reciben con `State<SqlitePool>`. Los repositorios aceptan cualquier `SqliteExecutor` para poder usarse dentro de transacciones.
- **Servicios sin Tauri:** reciben `impl Fn` para salida y fin, de modo que se prueban sin ventana ni `AppHandle`.
- **Eventos hacia la UI:** los emite el comando. Nombres actuales: `terminal-output` (texto), `terminal-exit` (código de salida) y `llm-token` (fragmento de respuesta). El fin del streaming es el retorno del comando.
- **Claves:** solo el servicio las lee del keychain (`system::keychain::read`); nunca viajan a la UI. Los errores del proveedor no incluyen su texto, solo el estado HTTP.
- **Proveedores sin nombres en el código:** el cliente recibe `base_url`, `model` y, opcional, el nombre del secreto. Los adaptadores por proveedor llegan en la Fase 3.
- **Pruebas junto al código** (`#[cfg(test)] mod tests`), con repositorios git temporales reales (`tempfile`) y sin APIs externas.
- Nombres de archivo en `snake_case`; el servicio se nombra por la acción (`worktree_run`), no por el recurso.

## Frontend (`src`)

La UI llama al núcleo **solo** con `invoke` y `listen` de `@tauri-apps/api`, y únicamente desde hooks.

```
src/
├── main.tsx               Monta <App /> en #root
├── styles.css             Tailwind v4, tokens Grafito (@theme) con light-dark() y `html[data-theme]` para forzar claro u oscuro (por defecto sigue al sistema), fuentes Inter y Rubik, y las clases de conectores del organigrama (org-*)
├── App.tsx                Elige entre la página de nuevo workspace (primer arranque o «crear otro») y el shell según la sesión
├── assets/fonts/          Inter y Rubik (woff2)
├── i18n/                  es.json, en.json, translate.ts (texto y errores) e I18nProvider
├── hooks/                 Acceso al núcleo y estado asociado
│   ├── useSession.ts      ✅ invoke load_session, complete_first_launch, create_workspace, switch_workspace, change_locale, change_theme
│   ├── useTeams.ts        ✅ invoke list_teams, create_team, create_agent
│   ├── useAgentTemplates.ts ✅ invoke list_agent_templates (recarga al cambiar el idioma)
│   └── useAgentRoles.ts   ✅ invoke list_agent_roles
├── test/setup.ts          Configuración de Vitest (jest-dom)
└── components/            Atomic design (overview.md §8)
    ├── atoms/             Avatar, Button, CardSelect, Field, Icons, Logo
    ├── molecules/         AgentTemplateDetail, Alert, BoardTools, IslandMenu, LocaleSwitcher, Modal, NavItem, OrgAgentCard, OrgCard, SegmentedControl, SettingsRow, SwitchingOverlay, TeamForm, ThemeSwitcher, WorkspaceSwitcher, ZoomControls
    ├── organisms/         AgentCatalogPage, AgentForm, NewWorkspacePage, OrgChart, Rail, SettingsPage, TeamsPage, Topbar
    └── templates/         Workspace (riel + topbar + contenido)
```

| Carpeta | Responsabilidad |
|---|---|
| `components/atoms`, `molecules`, `organisms`, `templates` | Presentación. Sin `invoke`: reciben datos y callbacks por props o usan un hook. Se crea cada nivel cuando hay un componente que lo necesita |
| `hooks/` | Único lugar que llama a `invoke` o `listen`. Un hook por caso de uso, nombrado `use<Verbo><Objeto>` |
| `App.tsx` | Compone organismos o templates; sin lógica |

Los tests con Vitest viven junto al código (`App.test.tsx`, `translate.test.ts`). Los spikes de Fase 0 que tenían interfaz (`run_in_worktree`, `stream_completion`) conservan su comando y servicio en el núcleo, pero ya no tienen pantalla; la interfaz definitiva llega con las fases 3 y 4.

## Dónde poner lo nuevo

| Quiero agregar... | Va en... |
|---|---|
| Un comando que invoca la UI | `commands/<recurso>.rs` + registro en `lib.rs` |
| Una acción de negocio | `services/<accion>.rs` |
| Un proceso hijo, PTY o API del SO | `system/<integracion>.rs` |
| Una consulta SQL | `repositories/<tabla>.rs` (requiere propuesta de migración aprobada) |
| Lógica pura sin I/O | `domain/<concepto>.rs` |
| Una pantalla o widget | `src/components/<nivel>/<Nombre>.tsx` |
| Llamada al núcleo desde la UI | `src/hooks/use<Verbo><Objeto>.ts` |
