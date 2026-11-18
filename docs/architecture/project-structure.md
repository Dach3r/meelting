# Estructura del proyecto

> **Estado:** Fases 0–2 completadas; salas, mensajes y el núcleo de `AgentRun` (ejecución con streaming, reintentos y cancelación) de la Fase 3 construidos. Solo existen los módulos listados aquí como «✅». Lo marcado «pendiente» es la ubicación **prevista**; no se crea la carpeta hasta que haya código que la use (sin código muerto).

Reglas de capas y convenciones: [overview.md](overview.md) §3 y §5.

## Vista general

```
meelting-desktop/
├── CLAUDE.md              Instrucciones del proyecto (inglés)
├── LICENSE                FSL-1.1-MIT
├── README.md
├── docs/
│   ├── product/           Visión, roadmap, usuarios, oficina virtual, salas, agentes, reuniones y registro de decisiones
│   └── architecture/      overview.md, este documento, AgentRun, motor conversacional y motor de voz (propuestas)
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
├── lib.rs                 Builder de Tauri, plugin de estado de ventana, expiración de ejecuciones al arrancar, motor de ejecución como estado y registro de comandos
├── commands/              Capa 1: comandos Tauri (delgados)
│   ├── keychain.rs        ✅ save_secret, secret_last_four, delete_secret
│   ├── worktree.rs        ✅ run_in_worktree
│   ├── agent_runs.rs      ✅ send_intervention, resolve_recipient, cancel_agent_run y el emisor de eventos del motor (agent_run:status, agent_run:token, message:created)
│   ├── rooms.rs           ✅ list_rooms, create_room, get_room (con ejecuciones e instantáneas), delete_room
│   └── session.rs         ✅ load_session, complete_first_launch, create_workspace, switch_workspace, list_teams, create_team, create_agent, get_agent, update_agent, delete_agent, save_canvas_positions, reset_canvas_positions, list_agent_templates, list_agent_roles, list_llm_providers, detect_local_models, change_locale, change_theme
├── services/              Capa 2: acciones de negocio (verbos)
│   ├── worktree_run.rs    ✅ crea un worktree y ejecuta un comando en él
│   ├── session_load.rs    ✅ carga el perfil y los workspaces; define SessionError
│   ├── first_launch.rs    ✅ crea el perfil y el primer workspace en una transacción
│   ├── workspace_create.rs ✅ crea un workspace y lo deja como actual
│   ├── workspace_switch.rs ✅ cambia el workspace actual
│   ├── teams_list.rs      ✅ equipos del workspace actual con sus agentes
│   ├── locale_change.rs   ✅ guarda el idioma del perfil
│   ├── theme_change.rs    ✅ guarda el tema del perfil
│   ├── team_create.rs     ✅ crea un equipo y devuelve la lista actualizada
│   ├── agent_create.rs    ✅ valida y crea un agente (personalizado o desde plantilla) y devuelve la lista actualizada
│   ├── agent_get.rs       ✅ detalle completo de un agente del workspace actual
│   ├── agent_update.rs    ✅ valida y edita nombre, rol, descripción, instrucciones, equipo, proveedor y modelo de un agente (el par se valida contra el catálogo solo si cambia)
│   ├── agent_delete.rs    ✅ elimina un agente del workspace actual y devuelve la lista actualizada; con mensajes o ejecuciones falla con agent_has_activity
│   ├── rooms_list.rs      ✅ salas del workspace actual por actividad reciente, con sus agentes y los últimos 3 mensajes de la primera
│   ├── recipient_resolve.rs ✅ resolve_recipient: vista previa del destinatario con las mismas reglas que send_intervention, sin guardar nada
│   ├── room_create.rs     ✅ valida el nombre, deduplica los agentes y crea la sala con sus participantes en una transacción
│   ├── room_get.rs        ✅ sala del workspace actual con sus agentes, sus últimos 100 mensajes, sus ejecuciones, las instantáneas del buffer (tomadas antes de leer la base) y el destinatario propuesto si sigue participando
│   ├── room_delete.rs     ✅ elimina una sala (participaciones, mensajes y ejecuciones en cascada), cancela sus ejecuciones activas y devuelve la lista actualizada
│   ├── message_send.rs    ✅ guarda un mensaje del usuario sin ejecución y actualiza la actividad de la sala en una transacción (lo usa intervention_send)
│   ├── intervention_send.rs ✅ send_intervention: valida, deduplica por client_request_id, resuelve el destinatario con domain/recipient dentro de la transacción y guarda el mensaje del usuario (con la ejecución queued si hay agente) o devuelve los candidatos para elegir sin guardar nada
│   ├── agent_run_engine.rs ✅ motor de ejecución: pool, registro, catálogo, límites, semáforos local (1) y nube (4) y emisor de eventos; lanza cada ejecución como tarea tokio
│   ├── agent_run_execute.rs ✅ tarea de una ejecución: espera de plaza, claim, contexto congelado, llamada al adaptador, tokens agrupados cada 50 ms, reintentos con plaza liberada, duración máxima, éxito atómico y fallos
│   ├── agent_run_registry.rs ✅ registro en memoria de ejecuciones aceptadas (texto del intento acotado por L1, secuencia, prompt congelado y token de cancelación) y último agente que respondió en cada sala (D19)
│   ├── agent_run_cancel.rs ✅ cancelación idempotente filtrada por workspace
│   ├── agent_runs_expire.rs ✅ expira las ejecuciones queued y running al arrancar
│   ├── canvas_positions_save.rs ✅ guarda las posiciones del organigrama de equipos y agentes del workspace actual
│   ├── canvas_positions_reset.rs ✅ borra las posiciones del organigrama del workspace actual y devuelve la lista actualizada
│   ├── agent_roles_list.rs ✅ roles predefinidos del catálogo, agrupados por área
│   ├── agent_templates_list.rs ✅ plantillas del catálogo en el idioma del perfil
│   ├── llm_providers_list.rs ✅ proveedores y modelos del catálogo
│   └── local_models_detect.rs ✅ consulta los proveedores locales del catálogo y devuelve si están en ejecución y sus modelos
├── system/                Capa 3: integración con el SO, sin reglas de negocio
│   ├── git.rs             ✅ git por tokio::process (worktree add)
│   ├── terminal.rs        ✅ PTY portable y lectura de salida
│   ├── llm_stream.rs      ✅ HTTP + SSE común a los adaptadores: conexión, inactividad, errores HTTP por estado y código, retry-after
│   ├── anthropic_messages.rs ✅ adaptador de la Messages API de Anthropic
│   ├── openai_responses.rs ✅ adaptador de la Responses API de OpenAI
│   ├── chat_completions.rs ✅ adaptador de Chat Completions (Groq, Ollama, LM Studio)
│   ├── local_models.rs    ✅ lista los modelos de un proveedor local (GET /models)
│   └── keychain.rs        ✅ keychain del SO (crate keyring)
├── repositories/          Acceso a SQLite (sqlx); las migraciones viven en src-tauri/migrations
│   ├── database.rs        ✅ abre el pool (WAL, claves foráneas) y aplica migraciones
│   ├── profile.rs         ✅ consultas de la tabla profile
│   ├── workspaces.rs      ✅ consultas de la tabla workspaces
│   ├── canvas_positions.rs ✅ consultas de la tabla canvas_positions
│   ├── teams.rs           ✅ consultas de la tabla teams
│   ├── agents.rs          ✅ consultas de la tabla agents
│   ├── rooms.rs           ✅ consultas de la tabla rooms
│   ├── room_agents.rs     ✅ consultas de la tabla room_agents
│   ├── messages.rs        ✅ consultas de la tabla messages
│   └── agent_runs.rs      ✅ consultas y transiciones con guarda de agent_runs, y las pruebas de integridad del esquema
├── domain/                Modelos y lógica pura, sin I/O
│   ├── agent_run.rs       ✅ estados, error_kind, ejecución, instantánea, límites internos inyectables y política de reintentos
│   ├── recipient.rs       ✅ resolución del destinatario: selección (automática, nadie, agente, confirmada), menciones @ por nombre o rol sin mayúsculas ni tildes, vocativo inicial, ambigüedad, varios agentes y conflicto
│   ├── prompt.rs          ✅ BuildPrompt: instrucciones, marco, historial etiquetado y presupuesto; context_too_large si el mínimo no cabe
│   ├── provider_error.rs  ✅ normalización de errores del proveedor por estado HTTP y código
│   ├── local_provider.rs  ✅ proveedor local detectado (clave, en ejecución, modelos)
│   ├── locale.rs          ✅ idioma es/en
│   ├── theme.rs           ✅ tema system/light/dark
│   ├── profile.rs         ✅ perfil local
│   ├── room.rs            ✅ sala, participante, mensaje y sus listas; validación del cuerpo y deduplicación de agentes
│   ├── session.rs         ✅ perfil + workspaces que recibe la UI
│   ├── team.rs            ✅ equipo y agente (resumen) que recibe la UI; AgentDraft y su validación
│   └── workspace.rs       ✅ workspace y validación del nombre
└── catalogs/              Colecciones de solo lectura embebidas en la app
    ├── agent_roles.rs     ✅ carga los roles predefinidos de agente (YAML) y valida que un rol exista
    ├── agent_roles.yml    ✅ roles agrupados por área
    ├── agent_templates.rs ✅ carga las plantillas de agente (YAML es/en) y valida claves
    ├── agent_templates/   ✅ una plantilla por rol predefinido (25 .yml, p. ej. product_owner, qa_engineer, legal_advisor)
    ├── llm.rs             ✅ carga proveedores (protocolo, base_url, parámetro de tokens de salida) y modelos con su máximo de salida, y valida que un par proveedor/modelo exista; la UI solo recibe los nombres
    └── llm.yml            ✅ proveedores en la nube y locales con sus modelos
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
- **Errores hacia la UI como código:** `SessionError` se serializa como un código estable (`invalid_name`, `invalid_agent`, `invalid_message`, `workspace_not_found`, `team_not_found`, `agent_not_found`, `room_not_found`, `agent_has_activity`, `invalid_client_request_id`, `agent_busy`, `client_request_conflict`, `not_set_up`, `database`); la UI lo traduce. Los fallos de una ejecución no son errores de comando: viajan como `error_kind` en la ejecución. No viaja el detalle de la base de datos.
- **Base de datos:** el pool se abre en `setup` (`app_data_dir()/meelting.db`) y se registra como estado de Tauri; los comandos lo reciben con `State<SqlitePool>`. Los repositorios aceptan cualquier `SqliteExecutor` para poder usarse dentro de transacciones. Las escrituras que leen y luego escriben (crear sala, enviar mensaje) abren la transacción con `pool.begin_with("BEGIN IMMEDIATE")` sobre una conexión nueva del pool (nunca anidada): toman el bloqueo de escritura al empezar y esperan hasta el `busy_timeout` de sqlx (5 s); si se agota, el error llega como `database`. Las lecturas de varias tablas usan una transacción normal para leer una instantánea coherente.
- **Servicios sin Tauri:** reciben `impl Fn` para salida y fin, de modo que se prueban sin ventana ni `AppHandle`.
- **Eventos hacia la UI:** los emite la capa de comandos. Nombres actuales: `terminal-output` (texto), `terminal-exit` (código de salida), `message:created` (mensaje completo con su `room_id`, después del commit), `agent_run:status` (la ejecución completa, después del commit de cada transición) y `agent_run:token` (`run_id`, `room_id`, `attempt`, `seq`, `text`). El motor de ejecución recibe un *callback* (`Emit`) que `commands/agent_runs.rs` traduce a eventos Tauri, así que los servicios siguen sin conocer Tauri.
- **Claves:** solo la tarea de ejecución las lee del keychain (`<workspace_id>/<provider>`) justo antes de cada intento; nunca viajan a la UI, eventos ni errores. Los errores del proveedor se reducen a un `error_kind`; su texto nunca sale del adaptador.
- **Proveedores sin nombres en el código:** el adaptador se elige con un `match` sobre el `protocol` del catálogo y recibe `base_url`, `model` y el tope de salida; ningún modelo se nombra en el código de producción.
- **Pruebas junto al código** (`#[cfg(test)] mod tests`), con repositorios git temporales reales (`tempfile`) y sin APIs externas.
- Nombres de archivo en `snake_case`; el servicio se nombra por la acción (`worktree_run`), no por el recurso.

## Frontend (`src`)

La UI llama al núcleo **solo** con `invoke` y `listen` de `@tauri-apps/api`, y únicamente desde hooks.

```
src/
├── main.tsx               Monta <App /> en #root
├── styles.css             Tailwind v4, tokens Grafito (@theme) con light-dark() y `html[data-theme]` para forzar claro u oscuro (por defecto sigue al sistema), fuentes Inter y Rubik, y las clases de disposición del organigrama (org-*)
├── App.tsx                Elige entre la página de nuevo workspace (primer arranque o «crear otro») y el shell según la sesión
├── assets/fonts/          Inter y Rubik (woff2)
├── i18n/                  es.json, en.json, translate.ts (texto, valores con nombre, errores de comando y de ejecución, fecha y número de agentes) e I18nProvider
├── hooks/                 Acceso al núcleo y estado asociado
│   ├── useSession.ts      ✅ invoke load_session, complete_first_launch, create_workspace, switch_workspace, change_locale, change_theme
│   ├── useTeams.ts        ✅ invoke list_teams, create_team, create_agent, get_agent, update_agent, delete_agent, save_canvas_positions (con debounce de 500 ms), reset_canvas_positions
│   ├── useRooms.ts        ✅ invoke list_rooms, create_room, delete_room
│   ├── useRoom.ts         ✅ invoke get_room, send_intervention (un client_request_id por envío, reutilizado en los reintentos automáticos y tras un timeout del IPC; sin reintentos si el destinatario es «Nadie») y cancel_agent_run; listen message:created, agent_run:status y agent_run:token antes de cargar la sala, con los eventos de la carga en espera
│   ├── useRecipientPreview.ts ✅ invoke resolve_recipient 150 ms después del último cambio del borrador (IPC local, sin red); descarta respuestas obsoletas
│   ├── agentRunState.ts   ✅ estado puro de las ejecuciones: orden lógico de estados (queued n = 2n, running n = 2n − 1, final al final), tokens por (attempt, seq), instantáneas y texto en vivo
│   ├── useAgentTemplates.ts ✅ invoke list_agent_templates (recarga al cambiar el idioma)
│   └── useAgentRoles.ts   ✅ invoke list_agent_roles
├── test/setup.ts          Configuración de Vitest (jest-dom)
└── components/            Atomic design (overview.md §8)
    ├── atoms/             Avatar, Button, CardSelect, Field, Icons, Logo
    ├── molecules/         AgentPicker, AgentRunStatus, AgentTemplateDetail, Alert, ChatMessage, EnginePicker, IslandMenu, LocaleSwitcher, MentionSuggestions, MessageComposer, Modal, NavItem, OrgAgentCard, OrgCard, OrgDrawer, OrgNode, ParticipantTile, RoomCard, RoomForm, SegmentedControl, SettingsRow, SwitchingOverlay, TeamForm, TeamPicker, ThemeSwitcher, WorkspaceSwitcher, ZoomControls; mentionDraft.ts detecta la mención en curso, filtra agentes por nombre o rol e inserta la elegida
    ├── organisms/         AgentCatalogPage, AgentEditForm, AgentForm, FeaturedRoom, NewWorkspacePage, OrgChart, Rail, RoomChat, RoomView, RoomsPage, SettingsPage, TeamsPage, Topbar; authorName.ts resuelve el autor de un mensaje
    └── templates/         Workspace (riel + topbar + contenido)
```

| Carpeta | Responsabilidad |
|---|---|
| `components/atoms`, `molecules`, `organisms`, `templates` | Presentación. Sin `invoke`: reciben datos y callbacks por props o usan un hook. Se crea cada nivel cuando hay un componente que lo necesita |
| `hooks/` | Único lugar que llama a `invoke` o `listen`. Un hook por caso de uso, nombrado `use<Verbo><Objeto>` |
| `App.tsx` | Compone organismos o templates; sin lógica |

Los tests con Vitest viven junto al código (`App.test.tsx`, `translate.test.ts`, `RoomsPage.test.tsx`, `RoomView.test.tsx`, `agentRunState.test.ts`). El spike `run_in_worktree` conserva su comando y servicio en el núcleo sin pantalla; su interfaz llega con la Fase 4. El spike `stream_completion` se eliminó al llegar los adaptadores de `AgentRun`.

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
