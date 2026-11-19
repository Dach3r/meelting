# Ejecución de agentes

> **Estado:** Fase 3 construida en su primer corte (`AgentRun`: «Pedir a», ejecución con estados persistidos, streaming, reintentos, cancelación y expiración al arrancar, para los cinco proveedores del catálogo); la pantalla «Claves de IA», los límites diarios y la Fase 5 (tareas con worktree y herramientas) siguen planificados. Comportamiento portado del producto Rails y ampliado con streaming, proveedores locales y cancelación. Decisiones en [decisions.md](decisions.md); diseño técnico aprobado en [agent-run.md](../architecture/agent-run.md).

**Problema:** el usuario tiene agentes y salas, pero los agentes no hacen nada.
**Objetivo:** que el usuario dirija una instrucción a un agente de una sala y reciba su respuesta, generada por un proveedor de IA (en la nube o local), de forma asíncrona, persistente, cancelable y observable en vivo. En una segunda etapa, que el agente trabaje sobre un repositorio real dentro de un worktree aislado.
**Historia de usuario:** Como responsable de un workspace, quiero pedirle una tarea a un agente dentro de una sala, para obtener un resultado y verlo llegar mientras se genera.

## Alcance y exclusiones

**Incluye (Fase 3)**
- Selector de agente en el compositor («Pedir a»): vacío es un mensaje normal; con un agente es una instrucción dirigida.
- Ejecución asíncrona como tarea `tokio`, con estados visibles (`queued`, `running`, `succeeded`, `failed`) y respuesta del agente como mensaje persistente.
- **Streaming de tokens** a la UI mediante eventos Tauri.
- **Cinco proveedores, los del catálogo real** (decisión aprobada D9): Anthropic (Messages API), OpenAI (Responses API), Groq (Chat Completions compatible con OpenAI) en la nube, y Ollama (`localhost:11434`) y LM Studio (`localhost:1234`) en local, ambos con Chat Completions compatible con OpenAI. Cada agente usa el proveedor y modelo que se le configuran. Sin DeepSeek, MiniMax ni un `local` genérico en esta fase.
- Catálogo de proveedores y modelos en `llm.yml`; las claves se guardan en el keychain del SO.
- Errores, reintentos acotados, timeouts, idempotencia, **cancelación** y registro de tokens.
- Límites diarios opcionales por workspace (propuestos para después del primer corte de `AgentRun`, A10).

**Incluye (Fases 4–5)**
- Tareas con `git worktree` aislado, herramientas del agente acotadas al worktree y aprobaciones humanas. Ver [Herramientas y aprobaciones](#herramientas-y-aprobaciones).

**Excluye (Fase 3)**
- Delegación de tareas entre agentes, integraciones externas. La conversación coordinada entre agentes y las menciones (`@agente`) se proponen para después de esta fase, en la Pista R ([voice-meetings.md](voice-meetings.md)).
- Créditos, facturación y cálculo de costos en dinero.
- Reintento manual de una ejecución fallida.
- Cambio automático de proveedor o modelo cuando uno falla.
- `push`, `merge` o modificación de la rama base.

## Flujo funcional
1. El usuario abre una sala con agentes y se dirige a uno: lo menciona con `@`, lo nombra al inicio («Backend, …»), lo elige en «Pedir a» o sigue con el agente propuesto ([conversation-engine.md](../architecture/conversation-engine.md#r1-implementado)).
2. Envía la instrucción. Debajo aparece el agente con «En cola…».
3. El motor toma la ejecución: «Pensando…» y, cuando llegan tokens, «Escribiendo…» con el texto en vivo en el mismo bloque. Si el proveedor falla de forma temporal, se reintenta hasta 3 veces con el mismo proveedor y modelo.
4. El usuario puede **cancelar** la ejecución en curso (`failed` con `error_kind = cancelled`).
5. Al terminar con éxito, la respuesta se persiste como mensaje del agente y el bloque provisional desaparece. El proveedor, el modelo y los tokens solo se muestran en el pie de la respuesta.
6. Si falla, la línea de estado explica la razón en el idioma del usuario, resuelta en la UI a partir de `error_kind`. El usuario puede volver a pedirlo.
7. Si el usuario sale de la sala, la ejecución continúa; la respuesta queda en la conversación cuando vuelva.

## Criterios de aceptación
- [x] Solo se puede dirigir una instrucción a un agente participante de la sala; un agente ajeno o de otro workspace falla sin guardar nada.
- [x] La instrucción queda guardada aunque la ejecución falle.
- [x] La respuesta es un mensaje persistente con el agente como autor; se emite a la UI en vivo.
- [x] Los estados se ven en vivo y sobreviven a cerrar y reabrir la app.
- [x] Los errores transitorios se reintentan (máximo 3 intentos, espera creciente, respetando `retry-after`); los permanentes fallan de inmediato con un mensaje claro.
- [x] Una ejecución no se procesa dos veces ni crea dos respuestas; hay una sola ejecución activa por agente y sala (`client_request_id` único e índice parcial).
- [x] Al iniciar la app, toda ejecución `running` huérfana pasa a `failed` con `error_kind = expired`.
- [x] Cada agente usa su proveedor y modelo; cada ejecución conserva los que copió al crearse, aunque después se cambien en el agente (D18).
- [x] Si falta la clave de un proveedor, solo falla ese proveedor con `provider_not_configured`.
- [x] La clave nunca llega a la UI, a los logs ni a los eventos; no se registran prompts ni razonamiento.
- [ ] Con solo el proveedor local, la app funciona sin conexión a internet.
- [x] Cancelar una ejecución la termina y libera al agente.
- [x] Estados vacíos, errores y confirmaciones en `es` y `en`.

## Reglas de negocio
- **Proveedor y modelo:** cada agente tiene `provider` y `model` (por defecto `anthropic` / `claude-sonnet-5-5`), elegidos entre los de `llm.yml`. Se pueden elegir proveedores sin clave: el agente muestra una advertencia y la ejecución falla con `provider_not_configured`. Si un modelo sale del catálogo, el agente sigue siendo editable y sus ejecuciones fallan con `model_unavailable`. Los proveedores locales (`ollama`, `lmstudio`) no declaran modelos: se descubren con `GET /v1/models`. El modelo siempre se elige de una lista, nunca se escribe a mano: en la nube, de `llm.yml`; en local, de los modelos que el entorno devuelve en ese momento. El usuario puede cambiar el proveedor y el modelo de un agente desde su edición (pestaña «Modelo», D18); el par nuevo se valida en el núcleo contra el catálogo (nube) o contra los modelos instalados en el entorno en ejecución (local), y las ejecuciones existentes conservan los que copiaron. Un agente local cuyo modelo ya no está instalado sigue siendo editable, la edición lo avisa y su disponibilidad en la sala es «No disponible». Los agentes con un modelo retirado del catálogo (`gpt-5`, `gpt-5-mini`, `llama-3.3-70b-versatile`) conservan sus datos y sus ejecuciones fallan con `model_unavailable` sin llamar al proveedor.
- **Historia fija:** el run copia `provider` y `model` del agente al crearse y no se sobrescriben.
- **Contexto:** instrucciones del agente, nombre de la sala (no existe columna de objetivo, D11) y mensajes de esa sala anteriores a la instrucción, congelados al empezar la ejecución y reutilizados en los reintentos, hasta 100 mensajes y un tope interno de caracteres (menor para los modelos locales; es una aproximación, no la ventana en tokens). Los mensajes de otros se envían con su nombre.
- **Generación:** la extensión de la respuesta se controla con el parámetro de tokens de salida que admite cada proveedor, con el valor que declara el catálogo para el modelo; otros parámetros (p. ej. `effort`) solo si el catálogo los declara. Sin `temperature`. Solo se persiste el texto final; el texto parcial vive en memoria durante el streaming y el razonamiento se descarta. `llm.yml` declara el máximo de salida documentado de cada modelo de la nube y Meelting pide el menor entre ese máximo y su tope interno (16 000 en la nube, 2048 en local).
- **Timeouts:** conexión, inactividad (45 s en la nube como en Rails, más en local) y duración máxima de la ejecución; son límites internos de Meelting, no del catálogo. Valores propuestos en [agent-run.md](../architecture/agent-run.md).
- **Idempotencia:** el compositor genera un `client_request_id` (UUID) por envío y lo reutiliza al reintentar ese mismo envío; si ya existe una ejecución con el mismo `(sala, client_request_id)` se devuelven su mensaje y su ejecución sin crear nada ni llamar al proveedor. El id es obligatorio, y el mismo id con otro agente u otro texto devuelve `client_request_conflict`, en lugar de la regla de Rails que generaba uno nuevo ([agent-run.md §3](../architecture/agent-run.md#3-contrato-de-send_instruction-e-idempotencia-a2)). Si el agente ya tiene una ejecución activa en la sala, la instrucción se rechaza con `agent_busy` y no se guarda nada.
- **Mensajes de error:** la UI los resuelve por `error_kind` (con el nombre del proveedor); uno desconocido muestra el genérico. Nunca se muestra el texto del proveedor ni claves.
- **Reintentos:** 3 intentos con espera creciente; el cliente HTTP usa `max_retries = 0` y solo el motor reintenta.
- **Errores transitorios:** `rate_limited`, `timeout`, `provider_unavailable`. **Permanentes:** `provider_not_configured`, `model_unavailable`, `provider_auth_failed`, `provider_billing`, `provider_rejected`, `content_filtered`, `output_truncated`, `invalid_response`, `context_too_large`, además de `cancelled` y `expired`. Un `connection_refused` contra `localhost` es `provider_unavailable` y la UI muestra «Ollama no está en ejecución».
- **Límites diarios (opcionales):** 50 ejecuciones y 500 000 tokens en 24 horas, desactivados para los proveedores locales. Protección inicial, no presupuesto estricto. Fuera del primer corte de `AgentRun`: necesitan la tabla `settings` (propuesta A10 en [agent-run.md](../architecture/agent-run.md)).
- **Registro:** por ejecución se guardan proveedor, modelo, intentos, tokens de entrada y salida (si el proveedor los informa) y fechas de creación, inicio y fin. El modelo reportado, los tokens de caché, `finish_reason` y el id de solicitud del proveedor no se guardan porque nada los usa todavía.
- **Eliminar un agente** con mensajes o ejecuciones no está permitido (`agent_has_activity`). **Finalizar una sala** conserva sus ejecuciones y mensajes y cancela las ejecuciones en curso.
- **Longitud de la respuesta (decisión aprobada D10):** el límite de 4000 caracteres es solo para los mensajes del usuario. La respuesta del agente no tiene límite de caracteres; la acotan los tokens de salida del modelo y los límites de contexto. Nunca se trunca en silencio: si el proveedor corta la respuesta (`finish_reason` de longitud) la ejecución falla con `output_truncated` y no se crea mensaje; si la filtra, con `content_filtered`. El esquema de `messages` no cambia.

## Implicaciones de las reuniones conversacionales

> **Propuesta, pendiente.** Nada de esta sección reemplaza las reglas anteriores mientras no se apruebe. Detalle en [conversation-engine.md §10](../architecture/conversation-engine.md#10-adaptaciones-de-agentrun).

- `AgentRun` sigue siendo **independiente del audio**: la voz consume el mensaje persistido y la entrada por voz llega como texto transcrito.
- **Idempotencia con varias ejecuciones por intervención:** un `client_request_id` único impediría que una intervención a «Equipo» cree varias ejecuciones (C1 en [conversation-engine.md](../architecture/conversation-engine.md#12-persistencia-y-modelo-de-datos); se resuelve en [agent-run.md](../architecture/agent-run.md)).
- El prompt se construye al hacer `claim`, para que cada turno vea las respuestas anteriores.
- Los tokens llevan `run_id` y una secuencia; al reintentar se reinicia el texto provisional y la voz solo usa el texto final.
- Posible resultado «pasar» (`succeeded` sin mensaje) en el modo equipo: afecta a los `CHECK` portados de Rails.
- Cancelación por intervención y por sala, además de por ejecución.
- El siguiente turno lo crea el coordinador, nunca la ejecución anterior.
- «Cambiar de modelo» es una acción explícita del usuario en la edición del agente (D18); nunca es automática cuando un proveedor falla.

## Claves de API

- Se guardan en el keychain del SO bajo el servicio `meelting` y el nombre `<workspace>/<provider>`. La tabla `llm_providers_configured` solo registra qué proveedores tienen clave (y la URL del proveedor local).
- Se gestionan en **Claves de IA**: resumen «N de M proveedores conectados» y una fila por proveedor que se expande para pegar la clave, «Verificar y guardar» y «Eliminar clave». Antes de guardar se prueba con una solicitud mínima (primer modelo del catálogo, `max_output_tokens: 16`). Si falla, no se guarda y se muestra el motivo; al reemplazar una clave, la anterior se conserva si la nueva falla.
- La clave nunca se vuelve a mostrar: solo «conectado» y sus últimos 4 caracteres.

## Herramientas y aprobaciones

Planificado (Fases 4–5); se documenta con [feature-template.md](feature-template.md) antes de implementarlo.

- Cada tarea tiene su propia rama `meelting/<slug>-<id corto>` y `git worktree` dentro del directorio de datos de la app, nunca dentro del repositorio del usuario.
- Herramientas: `read_file`, `list_dir`, `search`, `write_file`, `edit_file`, `run_command`, `git_status`, `git_diff`, `git_commit`. Nada de `push`.
- `run_command` exige **aprobación humana** mostrando el comando exacto (aprobar una vez, para la tarea o denegar), salvo una lista corta y revisable.
- Controles obligatorios: path jail (toda ruta canonicalizada dentro del worktree), entorno saneado (sin claves ni `SSH_AUTH_SOCK`), timeout y límite de salida por comando, árbol de procesos terminado al expirar.
- Docker opcional como aislamiento fuerte; sin Docker la app avisa que la ejecución **no está aislada**.
- Cada llamada queda en `tool_calls` para auditoría.

## Dependencias
- Diseño técnico propuesto: [agent-run.md](../architecture/agent-run.md).
- Agentes, salas y mensajes ([agent-catalog.md](agent-catalog.md), [rooms.md](rooms.md)).
- Keychain del SO, SQLite y eventos Tauri ([overview](../architecture/overview.md)).

## Limitaciones conocidas
- **Una sola invocación externa no está garantizada:** un cierre de la app después de que el proveedor respondió y antes de guardar el resultado puede provocar una nueva llamada. La idempotencia es local.
- Los límites diarios no son estrictos bajo concurrencia.
- Un razonamiento que agota `max_output_tokens` falla como `output_truncated`.
- Las respuestas se muestran como texto plano, sin markdown.
- Una ejecución fallida no se puede reintentar: hay que enviar la instrucción de nuevo.
- Los parámetros de cada proveedor están fijados con pruebas `wiremock`. Con proveedor real solo se ha verificado Ollama (incluido el uso de tokens en streaming). Anthropic, OpenAI, Groq y LM Studio esperan sus pruebas reales opcionales (`#[ignore]`, con la clave en una variable de entorno): en particular, la autenticación `Bearer` y la versión `2026-10-21` de Anthropic, si Groq acepta `stream_options` y la forma del fragmento de uso de LM Studio.
- La comprobación «sin conexión a internet» con solo un proveedor local no se ha hecho todavía con la red desactivada (la app no llama a la nube con agentes locales).

**Prioridad:** Must Have
**Métrica de éxito:** porcentaje de salas con al menos una ejecución `succeeded`, tasa de ejecuciones `failed` por `error_kind` y proveedor, tokens por ejecución y proveedor.
