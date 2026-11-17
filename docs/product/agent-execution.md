# Ejecución de agentes

> **Estado:** planificado (Fases 3 y 5). Comportamiento portado del producto Rails y ampliado con streaming, proveedor local, cancelación, tareas con worktree y herramientas.

**Problema:** el usuario tiene agentes y salas, pero los agentes no hacen nada.
**Objetivo:** que el usuario dirija una instrucción a un agente de una sala y reciba su respuesta, generada por un proveedor de IA (en la nube o local), de forma asíncrona, persistente, cancelable y observable en vivo. En una segunda etapa, que el agente trabaje sobre un repositorio real dentro de un worktree aislado.
**Historia de usuario:** Como responsable de un workspace, quiero pedirle una tarea a un agente dentro de una sala, para obtener un resultado y verlo llegar mientras se genera.

## Alcance y exclusiones

**Incluye (Fase 3)**
- Selector de agente en el compositor («Pedir a»): vacío es un mensaje normal; con un agente es una instrucción dirigida.
- Ejecución asíncrona como tarea `tokio`, con estados visibles (`queued`, `running`, `succeeded`, `failed`) y respuesta del agente como mensaje persistente.
- **Streaming de tokens** a la UI mediante eventos Tauri.
- **Cinco proveedores:** OpenAI (Responses API), Anthropic (Messages API), DeepSeek (Chat Completions), MiniMax (protocolo Anthropic-compatible) y **local** (Chat Completions compatible con OpenAI: Ollama en `localhost:11434`, LM Studio en `localhost:1234`). Cada agente usa el proveedor y modelo que se le configuran.
- Catálogo de proveedores y modelos en `llm.yml`; las claves se guardan en el keychain del SO.
- Errores, reintentos acotados, timeouts, idempotencia, **cancelación** y registro de tokens.
- Límites diarios opcionales por workspace.

**Incluye (Fases 4–5)**
- Tareas con `git worktree` aislado, herramientas del agente acotadas al worktree y aprobaciones humanas. Ver [Herramientas y aprobaciones](#herramientas-y-aprobaciones).

**Excluye**
- Colaboración y delegación entre agentes, integraciones externas.
- Créditos, facturación y cálculo de costos en dinero.
- Reintento manual de una ejecución fallida y menciones (`@agente`).
- Cambio automático de proveedor o modelo cuando uno falla.
- `push`, `merge` o modificación de la rama base.

## Flujo funcional
1. El usuario abre una sala con agentes y elige uno en «Pedir a».
2. Escribe la instrucción y la envía. Aparece en la conversación con la línea de estado «Esperando a <agente>…» y el proveedor y modelo del agente.
3. El motor toma la ejecución: «<agente> está trabajando…». Los tokens aparecen en vivo. Si el proveedor falla de forma temporal, se reintenta hasta 3 veces con el mismo proveedor y modelo.
4. El usuario puede **cancelar** la ejecución en curso (`failed` con `error_kind = cancelled`).
5. Al terminar con éxito, la respuesta se persiste como mensaje del agente con su proveedor y modelo, y la línea de estado pasa a «<agente> respondió».
6. Si falla, la línea de estado explica la razón en el idioma del usuario, resuelta en la UI a partir de `error_kind`. El usuario puede volver a pedirlo.
7. Si el usuario sale de la sala, la ejecución continúa; la respuesta queda en la conversación cuando vuelva.

## Criterios de aceptación
- [ ] Solo se puede dirigir una instrucción a un agente participante de la sala; un agente ajeno o de otro workspace falla sin guardar nada.
- [ ] La instrucción queda guardada aunque la ejecución falle.
- [ ] La respuesta es un mensaje persistente con el agente como autor; se emite a la UI en vivo.
- [ ] Los estados se ven en vivo y sobreviven a cerrar y reabrir la app.
- [ ] Los errores transitorios se reintentan (máximo 3 intentos, espera creciente, respetando `retry-after`); los permanentes fallan de inmediato con un mensaje claro.
- [ ] Una ejecución no se procesa dos veces ni crea dos respuestas; hay una sola ejecución activa por agente y sala (`client_request_id` único e índice parcial).
- [ ] Al iniciar la app, toda ejecución `running` huérfana pasa a `failed` con `error_kind = expired`.
- [ ] Cada agente usa su proveedor y modelo; cambiarlos no altera ejecuciones anteriores, que conservan los suyos.
- [ ] Si falta la clave de un proveedor, solo falla ese proveedor con `provider_not_configured`.
- [ ] La clave nunca llega a la UI, a los logs ni a los eventos; no se registran prompts ni razonamiento.
- [ ] Con solo el proveedor local, la app funciona sin conexión a internet.
- [ ] Cancelar una ejecución la termina y libera al agente.
- [ ] Estados vacíos, errores y confirmaciones en `es` y `en`.

## Reglas de negocio
- **Proveedor y modelo:** cada agente tiene `provider` y `model` (por defecto `anthropic` / `claude-sonnet-5-5`), elegidos entre los de `llm.yml`. Se pueden elegir proveedores sin clave: el agente muestra una advertencia y la ejecución falla con `provider_not_configured`. Si un modelo sale del catálogo, el agente sigue siendo editable y sus ejecuciones fallan con `model_unavailable`. El proveedor `local` no declara modelos: se descubren con `GET /v1/models`.
- **Historia fija:** el run copia `provider` y `model` del agente al crearse y no se sobrescriben; `response_model` guarda lo que reportó el proveedor.
- **Contexto:** instrucciones del agente, objetivo de la sala y mensajes anteriores, hasta 24 000 caracteres y 100 mensajes (tope configurable para modelos locales con poca ventana). Los mensajes de otros se envían con su nombre.
- **Generación:** `max_output_tokens` por modelo y `effort` solo si el modelo lo declara. Sin `temperature`. Solo se persiste el texto final; el texto parcial vive en memoria durante el streaming y el razonamiento se descarta.
- **Reintentos:** 3 intentos con espera creciente; el cliente HTTP usa `max_retries = 0` y solo el motor reintenta.
- **Errores transitorios:** `rate_limited`, `timeout`, `provider_unavailable`. **Permanentes:** `provider_not_configured`, `model_unavailable`, `provider_auth_failed`, `provider_billing`, `provider_rejected`, `content_filtered`, `output_truncated`, `invalid_response`, además de `cancelled` y `expired`. Un `connection_refused` contra `localhost` es `provider_unavailable` y la UI muestra «Ollama no está en ejecución».
- **Límites diarios (opcionales):** 50 ejecuciones y 500 000 tokens en 24 horas, desactivados para `local`. Protección inicial, no presupuesto estricto.
- **Registro:** por ejecución se guardan proveedor, modelo solicitado y reportado, tokens de entrada, salida y caché, `finish_reason` normalizado (`stop`, `length`, `content_filter`, `other`) e id de solicitud del proveedor.
- **Eliminar un agente** con mensajes o ejecuciones no está permitido. **Eliminar una sala** elimina sus ejecuciones y mensajes y cancela las ejecuciones en curso.
- Respuesta limitada a 4000 caracteres, como todo mensaje.

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
- Agentes, salas y mensajes ([agent-catalog.md](agent-catalog.md), [rooms.md](rooms.md)).
- Keychain del SO, SQLite y eventos Tauri ([overview](../architecture/overview.md)).

## Limitaciones conocidas
- **Una sola invocación externa no está garantizada:** un cierre de la app después de que el proveedor respondió y antes de guardar el resultado puede provocar una nueva llamada. La idempotencia es local.
- Los límites diarios no son estrictos bajo concurrencia.
- Un razonamiento que agota `max_output_tokens` falla como `output_truncated`.
- Las respuestas se muestran como texto plano, sin markdown.
- Una ejecución fallida no se puede reintentar: hay que enviar la instrucción de nuevo.
- Las capacidades de los modelos de OpenAI y MiniMax y el formato de uso de MiniMax solo se confirman con las pruebas reales opcionales y claves válidas.

**Prioridad:** Must Have
**Métrica de éxito:** porcentaje de salas con al menos una ejecución `succeeded`, tasa de ejecuciones `failed` por `error_kind` y proveedor, tokens por ejecución y proveedor.
