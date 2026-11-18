# AgentRun: propuesta técnica de la Fase 3

> **Estado:** propuesta, **pendiente de aprobación**. Nada de este documento está implementado y ninguna migración está aprobada. Producto: [agent-execution.md](../product/agent-execution.md). Decisiones: [decisions.md](../product/decisions.md) (D9–D13 aprobadas; las de este documento se marcan **A1–A12** hasta su aprobación). Relación con las reuniones: [conversation-engine.md](conversation-engine.md).
>
> **Existe hoy:** salas, participantes, mensajes del usuario y el evento `message:created`; el catálogo `llm.yml` (proveedores, ubicación, modelos y `base_url` de los locales); el keychain; el spike `stream_completion` (Chat Completions con SSE, sin reintentos ni persistencia).

## 1. Alcance

**Incluye:** pedirle una respuesta a un agente de la sala desde el compositor («Pedir a»), ejecución asíncrona con estados persistidos, streaming de tokens, cancelación, reintentos, timeouts, idempotencia, expiración al arrancar y adaptadores para los cinco proveedores del catálogo (D9).

**Excluye** (cada punto tiene su propia entrega o decisión):
- Pantalla «Claves de IA» y verificación de claves: entrega separada. Mientras tanto, los agentes con proveedor en la nube fallan con `provider_not_configured` si no hay clave; los locales funcionan sin clave.
- Tabla `llm_providers_configured`: no se crea; el núcleo consulta el keychain al ejecutar (A11).
- Límites diarios: necesitan la tabla `settings`, que no existe (A10).
- Coordinación multiagente, menciones, «pasar», sesión de reunión (Pista R) y voz (D13).
- Cambiar el proveedor o el modelo de un agente (D12).
- Herramientas, tareas y worktrees (Fases 4–5).

## 2. Modelo de datos: migración `create_agent_runs`

Propuesta para aprobación (A1). Una sola migración nueva, `20261119000001_create_agent_runs.sql`. No toca `messages`, `rooms` ni `agents`.

```sql
CREATE TABLE agent_runs (
    id TEXT PRIMARY KEY NOT NULL,
    room_id TEXT NOT NULL REFERENCES rooms (id) ON DELETE CASCADE,
    agent_id TEXT NOT NULL,
    request_message_id TEXT NOT NULL REFERENCES messages (id) ON DELETE NO ACTION,
    response_message_id TEXT UNIQUE REFERENCES messages (id) ON DELETE NO ACTION,
    client_request_id TEXT NOT NULL CHECK (length(client_request_id) = 36),
    status TEXT NOT NULL CHECK (status IN ('queued', 'running', 'succeeded', 'failed')),
    error_kind TEXT CHECK (error_kind IN (
        'rate_limited', 'timeout', 'provider_unavailable',
        'provider_not_configured', 'model_unavailable', 'provider_auth_failed',
        'provider_billing', 'provider_rejected', 'content_filtered',
        'output_truncated', 'invalid_response', 'cancelled', 'expired'
    )),
    attempts INTEGER NOT NULL DEFAULT 0 CHECK (attempts BETWEEN 0 AND 3),
    provider TEXT NOT NULL CHECK (length(trim(provider)) > 0),
    model TEXT NOT NULL CHECK (length(trim(model)) > 0),
    input_tokens INTEGER CHECK (input_tokens >= 0),
    output_tokens INTEGER CHECK (output_tokens >= 0),
    created_at TEXT NOT NULL,
    started_at TEXT,
    finished_at TEXT,
    CHECK ((status = 'failed') = (error_kind IS NOT NULL)),
    CHECK ((status = 'succeeded') = (response_message_id IS NOT NULL)),
    CHECK ((status IN ('succeeded', 'failed')) = (finished_at IS NOT NULL)),
    CHECK (status <> 'running' OR started_at IS NOT NULL),
    CHECK (response_message_id IS NULL OR response_message_id <> request_message_id),
    FOREIGN KEY (room_id, agent_id) REFERENCES room_agents (room_id, agent_id) ON DELETE NO ACTION
) STRICT;

CREATE UNIQUE INDEX agent_runs_room_id_client_request_id ON agent_runs (room_id, client_request_id);
CREATE UNIQUE INDEX agent_runs_active_room_id_agent_id ON agent_runs (room_id, agent_id)
    WHERE status IN ('queued', 'running');
CREATE INDEX agent_runs_room_id_created_at ON agent_runs (room_id, created_at, id);
CREATE INDEX agent_runs_request_message_id ON agent_runs (request_message_id);
```

**Decisiones del esquema:**

| Tema | Propuesta | Motivo |
|---|---|---|
| Aislamiento por workspace | Sin `workspace_id`: la clave foránea `(room_id, agent_id) → room_agents` ya garantiza que sala y agente son del mismo workspace y que el agente participa | Mismo patrón que `messages`; sin columnas redundantes |
| Borrado | Borrar la sala borra sus ejecuciones (cascada desde `rooms`). Un agente con ejecuciones no se puede eliminar ni quitar de la sala (`NO ACTION`), como con sus mensajes | Igual que `messages`; la comprobación al final de la sentencia permite borrar sala, participaciones, mensajes y ejecuciones juntos |
| Mensajes | `request_message_id` y `response_message_id` apuntan a `messages` sin cascada: los mensajes no se borran uno a uno | Inmutabilidad; ningún mensaje queda huérfano de su ejecución |
| Idempotencia (C1) | Única `(room_id, client_request_id)`. Cuando R2 necesite varias ejecuciones por intervención, se sustituye el índice por `(room_id, client_request_id, agent_id)` con una migración de solo índices, sin copiar la tabla | No se diseña para R2 por adelantado; cambiar un índice en SQLite es barato |
| Una ejecución activa por agente y sala | Índice parcial único | Regla aprobada (`agent_busy`) reflejada en la base |
| `succeeded` ⇔ respuesta | `CHECK` estricto. Si en R2 se aprueba «pasar» (P7), habrá que reconstruir la tabla | No se anticipa una decisión pendiente |
| `attempts` | 0–3; `claim` lo incrementa | Máximo 3 intentos |
| Tokens | `input_tokens` y `output_tokens`, nulos si el proveedor no los informa | Se muestran en el pie del mensaje del agente junto al proveedor y el modelo |
| Fechas | `created_at`, `started_at` (primer `claim`), `finished_at` | `started_at` fija además el corte del historial del prompt (§6) |

**Columnas de Rails que no se portan** (sin consumidor en la app): `user_id` (no hay usuarios), `updated_at` (lo cubren `started_at` y `finished_at`), `finish_reason` (se deriva: `stop` = `succeeded`, longitud = `output_truncated`, filtro = `content_filtered`), `response_model`, `cached_input_tokens` y `provider_request_id`. Si una pantalla los necesita, se proponen con su migración `add_<columna>_to_agent_runs`.

**Sin cambios en `messages`:** la respuesta del agente es un mensaje normal (`agent_id` del agente, `body` completo), y el `CHECK` actual ya no limita su longitud (D10). El proveedor y el modelo del mensaje se obtienen de la ejecución que lo produjo (`response_message_id`).

## 3. Estados y transiciones

```mermaid
stateDiagram-v2
    [*] --> queued: send_instruction (TX con el mensaje del usuario)
    queued --> running: claim (attempts + 1)
    running --> queued: requeue · error transitorio con attempts < 3
    running --> succeeded: succeed (TX con el mensaje del agente)
    running --> failed: error permanente · intentos agotados · cancelled
    queued --> failed: cancelled · expired
    running --> failed: expired (al arrancar)
    succeeded --> [*]
    failed --> [*]
```

- Cada transición es una función del repositorio con guarda de estado (`UPDATE … WHERE id = ? AND status = ?`). Si no actualiza ninguna fila, la transición no ocurrió y el llamador se detiene sin efectos. Así, un resultado, un token o un error que llega tarde **nunca reactiva** una ejecución cancelada o expirada.
- `queued` con `attempts > 0` significa «esperando para reintentar»; la UI lo muestra como «Reintentando…».
- **Al arrancar** (A7), toda ejecución `queued` o `running` pasa a `failed` / `expired`: la cola vive en memoria y no sobrevive al cierre. Rails solo expiraba `running`; aquí `queued` también queda huérfana.
- Los fallos previos a la llamada (`provider_not_configured`, `model_unavailable`) ocurren después de `claim`, como `running → failed`, para tener un único camino.

## 4. Flujo y persistencia de resultados

1. **`send_instruction(room_id, agent_id, body, client_request_id)`** (comando nuevo; `send_message` no cambia, A2). En una transacción `BEGIN IMMEDIATE`:
   - valida el cuerpo con las reglas del usuario (1–4000 puntos de código), que la sala es del workspace actual y que el agente participa;
   - si ya existe una ejecución `(room_id, client_request_id)`, devuelve su mensaje y su ejecución sin crear nada;
   - si el agente tiene una ejecución activa en la sala, falla con `agent_busy` sin guardar nada;
   - inserta el mensaje del usuario y la ejecución `queued` (copiando `provider` y `model` del agente) y actualiza `rooms.updated_at`.
   Tras el commit emite `message:created` y `agent_run:status`, y lanza la tarea de ejecución. Un `client_request_id` ausente o inválido se reemplaza por uno nuevo (sin idempotencia), como en Rails.
2. **Ejecución** (tarea `tokio`): `claim` → construir el prompt (§6) → leer la clave del keychain si el proveedor es de la nube → llamar al adaptador con streaming (§5, §9).
3. **Éxito:** en una transacción se inserta el mensaje del agente con el **texto completo** y se pasa la ejecución a `succeeded` con `response_message_id` y los tokens; se actualiza `rooms.updated_at`. Si la guarda falla (cancelada mientras tanto), se hace *rollback* y no queda mensaje. Tras el commit: `message:created` y `agent_run:status`.
4. **Respuesta vacía** (solo espacios o sin texto): `failed` / `invalid_response`.
5. **Respuesta incompleta** (D10): longitud agotada → `failed` / `output_truncated`; filtro de contenido → `failed` / `content_filtered`. Nunca se guarda el parcial ni se recorta nada.

**Propuesta separada L1 (límite técnico de memoria, opcional):** si el texto acumulado de una ejecución supera **256 KiB** (UTF-8), se corta la llamada y la ejecución falla con `output_truncated`, sin mensaje. Justificación: los tokens de salida del catálogo (≤ 16 000) dejan las respuestas válidas muy por debajo (~64 KiB), pero un servidor local que ignore `max_tokens` podría enviar texto sin fin y llenar la memoria y la base. No cambia el esquema ni afecta a respuestas normales. Sin L1, el único tope es el del proveedor. No hace falta un límite de presentación: los mensajes ya se muestran como texto plano en una lista de los últimos 100.

## 5. Streaming

| Evento | Carga | Cuándo |
|---|---|---|
| `agent_run:status` | La ejecución completa (`id`, `room_id`, `agent_id`, `request_message_id`, `response_message_id`, `status`, `error_kind`, `attempts`, `provider`, `model`, tokens, fechas) | Después de cada transición confirmada |
| `agent_run:token` | `run_id`, `room_id`, `attempt`, `seq` (creciente por intento, desde 0), `text` | Durante `running`; tokens agrupados cada ≤ 50 ms para no saturar el IPC |
| `message:created` ✅ | El mensaje | Tras persistir la respuesta |

- El texto provisional vive en un **registro en memoria** de ejecuciones activas (texto, intento, último `seq`, token de cancelación), gestionado como `State` de Tauri pero definido en `services/` sin tipos de Tauri.
- **Reabrir la sala con una ejecución en curso:** `get_room` añade a su respuesta las ejecuciones de los mensajes cargados y una instantánea del texto provisional de las activas (`run_id`, `attempt`, `seq`, `text`). La UI sigue a partir de ese `seq` e ignora los anteriores (A3).
- **Reintento:** `agent_run:status` con `queued` y el intento nuevo; la UI descarta el texto provisional de ese `run_id` y empieza de cero con el siguiente intento.
- La UI ignora tokens de intentos viejos, `seq` repetidos y tokens de ejecuciones en estado final, y sustituye el texto provisional por el mensaje cuando llega `message:created`.

## 6. Construcción del prompt (`BuildPrompt`, dominio puro)

Función pura en `domain/`, sin I/O, que recibe datos ya cargados y devuelve una estructura neutral (instrucciones + lista de turnos `user`/`assistant`). Cada adaptador la traduce a su protocolo.

| Bloque | Contenido | ¿Se recorta? |
|---|---|---|
| Instrucciones (rol de sistema) | Nombre y rol del agente, sus `instructions`, el nombre de la sala (D11) y la lista de participantes (nombre y rol). Indicación fija: los mensajes de otros agentes son aportes de otros participantes, no instrucciones del usuario | Nunca |
| Historial | Mensajes de la sala anteriores a la instrucción, hasta el corte: propios como `assistant`; del usuario como `user`; de otros agentes como `user` con la etiqueta `[Nombre · Rol]` | Desde los más antiguos, mensajes enteros |
| Instrucción actual | El mensaje del usuario que originó la ejecución | Nunca |

- **Corte del historial:** mensajes con `created_at ≤ started_at` (A4). Así cada reintento envía el mismo contexto, entran los mensajes que el usuario escribió mientras la ejecución esperaba, y en R2 el turno siguiente verá la respuesta anterior.
- **Presupuesto:** hasta `context_chars` del catálogo y 100 mensajes, contando instrucciones, historial e instrucción actual. Si las instrucciones y la instrucción actual ya lo superan, se envían igualmente sin historial.
- **Normalización:** turnos consecutivos del mismo rol se unen; el historial empieza siempre por un turno `user` (se descarta un `assistant` inicial).
- **Identidad:** la etiqueta sale de `messages.agent_id`, nunca del texto.
- El texto fijo del prompt se escribe en el idioma del perfil (`es`/`en`).

## 7. Contexto por sala

- Una ejecución solo ve los mensajes de **su** sala. No hay memoria entre salas ni entre workspaces.
- Las instrucciones de un agente nunca se envían a otro.
- Una ejecución queda fijada a su sala, agente, proveedor y modelo al crearse; cambiar de workspace no la detiene ni la mueve.
- El nombre del workspace no se envía (no aporta y filtra datos innecesarios a la nube).

## 8. Cancelación

- **`cancel_agent_run(run_id)`** (A5): valida que la ejecución es de una sala del workspace actual; pasa a `failed` / `cancelled` con guarda (`WHERE status IN ('queued','running')`); emite `agent_run:status`; dispara el `CancellationToken` del registro. La tarea aborta la petición HTTP (`select!` entre el stream y el token) y su transición final no encuentra fila. Cancelar algo terminado o inexistente es un éxito sin efecto.
- **Eliminar la sala:** se dispara el token de sus ejecuciones activas y la cascada las borra; la tarea termina sin efectos.
- **Salir de la sala o cambiar de workspace:** la ejecución continúa (regla aprobada).
- **Cerrar la app:** las tareas mueren con el proceso; al arrancar se expiran (§3).

## 9. Reintentos y timeouts

- **Intentos:** máximo 3 por ejecución. El cliente HTTP no reintenta (`max_retries = 0`); solo el motor.
- **Transitorios** (se reintentan): `rate_limited`, `timeout`, `provider_unavailable`, incluido un corte del stream a mitad de respuesta. **Permanentes:** el resto.
- **Espera:** 2 s tras el primer intento y 8 s tras el segundo, o `retry-after` si el proveedor lo envía. Si `retry-after` supera 60 s, la ejecución falla con `rate_limited` en lugar de esperar.
- **Timeouts** (A6), todos declarados en el catálogo salvo la conexión:
  - conexión: 10 s (el del spike);
  - inactividad: tiempo máximo sin recibir datos, incluido el primer byte. Nube 45 s; local 120 s, porque cargar un modelo en memoria puede tardar (valor a medir).
  - No hay timeout total: el tope lo pone `max_output_tokens` (y L1 si se aprueba).
- Errores HTTP: 401/403 → `provider_auth_failed`, 402 → `provider_billing`, 404 de modelo → `model_unavailable`, 408 → `timeout`, 429 → `rate_limited`, 5xx → `provider_unavailable`, resto → `provider_rejected`. Conexión rechazada en `localhost` → `provider_unavailable` («Ollama no está en ejecución»). Errores del proveedor en el cuerpo (p. ej. cuota agotada de OpenAI) se mapean por su código, nunca por su texto.

## 10. Integración con los proveedores

**Catálogo** (A8): se amplía `llm.yml` sin cambiar la forma de `models` (sigue siendo una lista de nombres) para declarar protocolo y límites por proveedor. Extracto: se omiten `name`, `location` y `models` donde no cambian.

```yaml
- key: anthropic
  name: Anthropic
  location: cloud
  protocol: anthropic_messages
  base_url: https://api.anthropic.com/v1
  max_output_tokens: 8192
  context_chars: 24000
  timeout_seconds: 45
  models: [claude-sonnet-5-5, claude-opus-5-5, claude-haiku-5-5]
- key: openai
  protocol: openai_responses
  base_url: https://api.openai.com/v1
  max_output_tokens: 16000
  context_chars: 24000
  timeout_seconds: 45
- key: groq
  protocol: chat_completions
  base_url: https://api.groq.com/openai/v1
  max_output_tokens: 8192
  context_chars: 24000
  timeout_seconds: 45
- key: ollama
  protocol: chat_completions
  base_url: http://localhost:11434/v1
  max_output_tokens: 4096
  context_chars: 8000
  timeout_seconds: 120
- key: lmstudio
  protocol: chat_completions
  base_url: http://localhost:1234/v1
  max_output_tokens: 4096
  context_chars: 8000
  timeout_seconds: 120
```

- Límites por proveedor y no por modelo: hoy todos los modelos de un proveedor usarían el mismo valor (en Rails todos tenían 8192). Si un modelo necesita otro, se propone entonces.
- OpenAI lleva un tope mayor porque en sus modelos de razonamiento los tokens de razonamiento cuentan dentro de `max_output_tokens`.
- `context_chars` menor en local porque los servidores locales suelen trabajar con ventanas pequeñas; valor a medir con modelos reales.
- Sin `effort` ni otros parámetros opcionales: ningún comportamiento los usa todavía.
- Añadir un proveedor con un protocolo existente (p. ej. otro compatible con Chat Completions) será solo una entrada en el catálogo.

**Adaptadores:** tres protocolos para cinco proveedores, elegidos con un `match` sobre `protocol` (tres casos: sin trait ni `async_trait`). Cada uno recibe el prompt neutral, `max_output_tokens`, la clave opcional y un callback de tokens, y devuelve el texto, los tokens de uso y un final normalizado (`stop`, `length`, `content_filter`) o un `LlmError`.

| Protocolo | Proveedores | Petición | Final y uso |
|---|---|---|---|
| `anthropic_messages` | `anthropic` | `POST /messages`, cabeceras `x-api-key` y `anthropic-version`, `system`, `messages`, `max_tokens`, `stream: true` | `content_block_delta` (`text_delta`) aporta texto; `message_delta.stop_reason`: `end_turn`/`stop_sequence` → `stop`, `max_tokens` → `length`, `refusal` → `content_filter`; uso en `message_start` y `message_delta`; evento `error` → error |
| `openai_responses` | `openai` | `POST /responses`, `Authorization: Bearer`, `instructions`, `input`, `max_output_tokens`, `store: false`, `stream: true` | `response.output_text.delta` aporta texto; `response.completed` → `stop` con uso; `response.incomplete` con motivo `max_output_tokens` → `length` y `content_filter` → `content_filter`; `response.failed` y `error` → error |
| `chat_completions` | `groq`, `ollama`, `lmstudio` | `POST /chat/completions`, `messages` con el rol `system`, `stream: true`, `stream_options.include_usage`; tope con `max_completion_tokens` en `groq` y `max_tokens` en los locales; clave `Bearer` solo en `groq` | `choices[].delta.content` aporta texto; `finish_reason`: `stop` → `stop`, `length` → `length`, `content_filter` → `content_filter`; uso en el último fragmento; `[DONE]` cierra |

- El adaptador `chat_completions` evoluciona el spike `system/chat_completions.rs`. Al terminar, el comando `stream_completion` y el servicio `completion_stream` quedan sin uso y **se eliminan en el mismo cambio** (sin código muerto).
- Los nombres exactos de parámetros y eventos se fijan con pruebas `wiremock` y se confirman con las pruebas reales opcionales; en particular el uso en streaming de Groq, Ollama y LM Studio (I: si no lo informan, los tokens quedan nulos).
- Nunca se cambia de proveedor ni de modelo, ni se reintenta con otro (D6).

## 11. Concurrencia e idempotencia

- **Idempotencia:** índice único `(room_id, client_request_id)` y búsqueda previa en `send_instruction` (§4). Las transiciones con guarda impiden procesar dos veces una ejecución o crear dos respuestas.
- **Una activa por agente y sala:** índice parcial; `agent_busy` sin guardar nada.
- **Límite de simultaneidad** (A9, resuelve P22): como máximo **1 ejecución en curso por proveedor local** (Ollama, LM Studio), porque un modelo local satura la máquina, y **4 en total para la nube**. Las que esperan siguen en `queued` (cuenta para la UI como «En cola»). Se implementa con semáforos en memoria del registro; los valores se ajustan tras medir.
- **Estado compartido:** el registro de ejecuciones activas se protege con un `Mutex` que nunca se mantiene durante un `await`.
- **SQLite:** las escrituras usan `BEGIN IMMEDIATE`, como `send_message`, para evitar interbloqueos al promover la transacción.
- **Limitación conocida:** si la app se cierra después de que el proveedor respondió y antes de guardar, la instrucción se pierde (`expired`) y repetirla genera otra llamada; la idempotencia es local.

## 12. Seguridad y aislamiento

- **Claves:** solo el núcleo lee el keychain (`<workspace>/<provider>`), justo antes de la llamada; nunca viajan en eventos, errores, logs ni respuestas de comandos.
- **Errores:** solo `error_kind`; nunca el texto del proveedor. La UI resuelve el mensaje en `es`/`en`.
- **Sin registro de contenido:** no se registran prompts, respuestas ni razonamiento.
- **Workspace:** toda operación parte del workspace actual; las claves foráneas compuestas impiden ejecuciones con agentes de otra sala o workspace.
- **Autoría:** el autor de cada mensaje lo fija el núcleo; las etiquetas del prompt salen de `agent_id`.
- **Inyección entre agentes:** los mensajes de otros agentes van etiquetados como aportes, nunca como sistema ni como el usuario. `AgentRun` no tiene herramientas: ningún texto puede disparar acciones.
- **Red:** solo las `base_url` del catálogo (HTTPS en la nube, `localhost` en local). Con solo proveedores locales, la app no sale a internet.
- **Webview:** dos comandos nuevos (`send_instruction`, `cancel_agent_run`) y ningún permiso nuevo de `fs`, `shell` ni red.

## 13. Organización del código

| Capa | Pieza |
|---|---|
| `domain/` | Estados y `error_kind` de la ejecución, política de reintentos, `BuildPrompt` |
| `repositories/` | `agent_runs`: crear, `claim`, `requeue`, `succeed` (con el mensaje), `fail`, `expire`, listar por sala |
| `system/` | Un módulo por protocolo: `anthropic_messages`, `openai_responses`, `chat_completions` (este último sustituye al spike) |
| `services/` | `instruction_send`, `agent_run_execute` (bucle con reintentos y streaming), `agent_run_cancel`, `agent_runs_expire` (al arrancar), registro de ejecuciones activas |
| `catalogs/` | `llm.rs` lee los campos nuevos |
| `commands/` | `send_instruction`, `cancel_agent_run`; `get_room` amplía su respuesta |
| Frontend | `useRoom` (invocar y escuchar), «Pedir a» en `MessageComposer`, `AgentRunStatus`, pie del mensaje con proveedor, modelo y tokens, estado «Pensando» en `ParticipantTile`, textos de `error_kind` en `es`/`en` |

**Dependencias:** `tokio` con las funciones `time`, `sync` y `macros`; `tokio-util` para `CancellationToken`. Ninguna otra.

`project-structure.md` se actualiza con los módulos reales al implementarlos.

## 14. Pruebas

- **Dominio:** `BuildPrompt` (etiquetas de autor, corte por `started_at`, recorte por caracteres y por número, instrucciones nunca recortadas, unión de turnos, primer turno `user`); política de reintentos (`retry-after`, tope de 60 s, máximo 3); mapeo de errores HTTP.
- **Repositorio con SQLite temporal:** cada `CHECK` y cada índice; transiciones con guarda; `succeed` sin fila no crea mensaje; expiración de `queued` y `running`; borrar la sala borra las ejecuciones; un agente con ejecuciones no se puede eliminar.
- **Servicios:** `send_instruction` atómico e idempotente (mismo `client_request_id` en paralelo crea una sola ejecución), `agent_busy`, agente que no participa o de otro workspace, cancelación idempotente y concurrente con el éxito, eventos tardíos sin efecto, límite de simultaneidad.
- **Adaptadores con `wiremock`:** por protocolo, éxito con tokens en orden, cada final (`stop`, `length`, `content_filter`), uso de tokens, cada código HTTP, `retry-after`, stream cortado, JSON inválido, timeout de inactividad, cancelación a mitad del stream. Nunca APIs reales en CI.
- **Pruebas reales opcionales** (`#[ignore]` con variables de entorno): Ollama o LM Studio y, con clave, cada proveedor en la nube.
- **Frontend (Vitest):** «Pedir a», texto provisional y su reemplazo, reintento que reinicia el texto, eventos fuera de orden, cancelar, reabrir la sala con una ejecución en curso, mensajes de error por `error_kind`.
- **E2E (`tauri-driver`):** instrucción a un agente con un proveedor simulado.

## 15. Criterios de aceptación

Los de [agent-execution.md](../product/agent-execution.md), más:
- [ ] Con solo Ollama o LM Studio en ejecución y sin conexión a internet, un agente local responde en vivo y la respuesta queda guardada.
- [ ] Una respuesta cortada por longitud o por filtro no crea mensaje y muestra el motivo; nunca se guarda un texto recortado.
- [ ] Una respuesta del agente de más de 4000 caracteres se guarda completa.
- [ ] Cancelar durante el streaming detiene la petición, no crea mensaje y libera al agente; cancelar dos veces no falla.
- [ ] Reabrir la sala durante una ejecución muestra el texto acumulado y sigue en vivo.
- [ ] Al reabrir la app, ninguna ejecución queda en `queued` ni `running`.
- [ ] Ninguna clave ni texto del proveedor aparece en eventos, errores ni logs.
- [ ] El spike `stream_completion` ya no existe.

## 16. Decisiones que se piden

| # | Decisión | Recomendación |
|---|---|---|
| A1 | Esquema `create_agent_runs` (§2) | Aprobar tal cual |
| A2 | Comando nuevo `send_instruction` sin tocar `send_message` | Sí: no rompe el contrato actual de las salas |
| A3 | `get_room` incluye ejecuciones y el texto provisional de las activas | Sí |
| A4 | Corte del historial por `started_at` | Sí |
| A5 | Comando `cancel_agent_run` idempotente | Sí |
| A6 | Timeouts de conexión e inactividad, sin timeout total | Sí, con los valores del catálogo |
| A7 | Expirar `queued` y `running` al arrancar | Sí |
| A8 | Ampliar `llm.yml` con `protocol`, `base_url`, `max_output_tokens`, `context_chars` y `timeout_seconds` por proveedor | Sí |
| A9 | Simultaneidad: 1 por proveedor local, 4 en la nube | Sí, a medir |
| A10 | Límites diarios fuera de este corte | Sí: requieren `settings` |
| A11 | Sin `llm_providers_configured`; la pantalla de claves es otra entrega | Sí |
| A12 | Eliminar el spike `stream_completion` al sustituirlo | Sí |
| L1 | Límite de memoria de 256 KiB por respuesta (propuesta separada) | Sí, como salvaguarda |
