# Salas de trabajo

> **Estado:** construido (Fase 3): salas, participantes, historial, envío de mensajes del usuario y, con `AgentRun`, el selector «Pedir a», las respuestas de los agentes en streaming y el estado «Pensando…» de cada agente ([agent-execution.md](agent-execution.md)). Pendiente: el panel de pantalla compartida. Comportamiento portado del producto Rails.

**Problema:** el usuario tiene agentes, pero no existe un lugar donde reunirlos.
**Objetivo:** que el usuario cree salas, asigne agentes y converse en un historial persistente. La sala es la unidad de contexto de trabajo: los equipos organizan agentes; las salas los reúnen para trabajar juntos. Los agentes asignados responden cuando el usuario les dirige una instrucción ([agent-execution.md](agent-execution.md)).
**Historia de usuario:** Como responsable de un workspace, quiero crear una sala, asignarle agentes y conversar en ella, para dejar listo el contexto de trabajo antes de que los agentes ejecuten.

## Alcance y exclusiones

**Incluye**
- Tabla `rooms` perteneciente a `workspaces`, con nombre obligatorio.
- Participación de agentes (`room_agents`): un agente puede estar en varias salas y una sala puede tener varios agentes, de cualquier equipo del mismo workspace.
- Crear, ver y eliminar salas. Los participantes se eligen al crear con un buscador por nombre o rol; los elegidos quedan como fichas que se quitan con su cruz o con Retroceso (con el buscador vacío); Intro añade la primera coincidencia; una sala creada no se edita. Si llegan agentes repetidos, el núcleo los deduplica conservando el orden de la primera aparición, sin error.
- Se permite una sala sin agentes.
- Mensajes (`messages`) del usuario y de agentes, con historial persistente por sala. La entrega en vivo usa **eventos Tauri** emitidos por el núcleo. Los mensajes son inmutables: sin edición ni eliminación individual.
- Pantalla de la sala como reunión: una tarjeta por participante (el usuario y cada agente con su rol), la conversación como transcripción en un panel lateral con el compositor (Intro envía, Mayús+Intro salta de línea), y una barra inferior con el número de agentes y la acción de eliminar. Mientras un agente tiene una ejecución activa en la sala, su tarjeta muestra «Pensando…».
- Listado de salas con tarjetas del tema oscuro Grafito.
- Selector «Pedir a» en el compositor: vacío envía un mensaje; con un agente envía una instrucción dirigida. Bajo la instrucción se ven su estado (en cola, trabajando, reintentando, respondió o el error traducido), el proveedor y el modelo, el texto en vivo y el botón «Cancelar»; la respuesta guardada muestra proveedor, modelo y tokens.

**Excluye**
- Archivado y edición de salas, edición o borrado de mensajes, paginación del historial (se cargan los últimos 100).
- Miembros humanos adicionales.

## Flujo funcional
1. El usuario abre **Salas** y ve sus salas o un estado vacío. El listado va ordenado por actividad reciente (`rooms.updated_at`, que es la creación y se actualiza con cada mensaje; desempate por `id`): la primera se destaca con sus agentes y una vista previa de sus últimos 3 mensajes, y el resto aparece como tarjetas con su número de agentes y última actividad.
2. Crea una sala: nombre y, opcionalmente, agentes.
3. Al crearla entra directamente; ve a los participantes como tarjetas y la conversación (últimos 100 mensajes) a un costado.
4. Escribe un mensaje: el núcleo lo guarda y actualiza la actividad de la sala en una transacción, lo devuelve y, tras el commit, emite `message:created`. La UI lo añade desde el resultado o el evento sin duplicarlo (por `id`).
5. Elimina la sala con confirmación; se eliminan también sus mensajes y participaciones.

## Criterios de aceptación
- [x] Crear y eliminar salas del workspace actual.
- [x] Asignar agentes de cualquier equipo al crear la sala; un agente no se duplica en una sala.
- [x] Un agente de otro workspace no puede asignarse, ni manipulando los argumentos del comando.
- [x] Una sala de otro workspace no se puede leer ni escribir (error `room_not_found` del núcleo).
- [x] El autor de un mensaje del usuario lo fija el núcleo, nunca el argumento del comando.
- [x] Los mensajes persisten y se emiten a la UI mediante `message:created`.
- [x] Estados vacíos, errores y confirmaciones, en `es` y `en`.

## Reglas de negocio
- `name` obligatorio, máximo 80 caracteres.
- `body` del mensaje obligatorio (el núcleo recorta los espacios de los extremos). Máximo 4000 caracteres solo para los mensajes del usuario; las respuestas de agentes no tienen límite de caracteres ni se truncan: las acotan los tokens de salida del modelo (decisión aprobada D10, [agent-execution.md](agent-execution.md)). La UI, el núcleo y SQLite cuentan igual: puntos de código Unicode (`Array.from` en la UI, `chars()` en Rust, `length()` en SQLite) sobre el texto recortado; un emoji simple cuenta 1 y uno compuesto con ZWJ cuenta cada punto de código. Si el mensaje pasa de 4000, el compositor desactiva el envío y muestra el aviso, sin cortar el texto.
- Sala y agente de una participación deben pertenecer al mismo workspace; se valida en el servicio y lo refuerzan claves foráneas compuestas con `workspace_id`.
- Un agente solo puede tener mensajes en salas donde participa (clave foránea `messages (room_id, agent_id) → room_agents`).
- Eliminar una sala elimina sus mensajes y participaciones. Eliminar un agente elimina sus participaciones, salvo que tenga mensajes o ejecuciones: entonces no se puede eliminar. El núcleo devuelve el error propio `agent_has_activity`.
- Todas las consultas parten del workspace actual.
- Un `agent_ids` con agentes de otro workspace (o inexistentes) falla y no cambia ninguna participación: la asignación es atómica (una transacción SQLite).

## Dependencias
- Agentes y equipos ([agent-catalog.md](agent-catalog.md)).
- SQLite y eventos Tauri.

## Evolución prevista

✅ Con R1, `send_intervention` sustituye a `send_message` y a `send_instruction`: un mensaje sin destinatario se guarda igual que antes (mismas validaciones, `rooms.updated_at` y `message:created`), y uno dirigido a un agente crea además su `AgentRun`. El núcleo resuelve el destinatario ([conversation-engine.md](../architecture/conversation-engine.md#r1-implementado)). `list_rooms`, `create_room`, `get_room`, `delete_room` y `message:created` no cambian.

| Nivel | Estado | Qué añade | Documento |
|---|---|---|---|
| Sala actual | ✅ implementado | Sala, participantes, historial, mensajes del usuario | Este documento |
| Conversación con `AgentRun` | ✅ Construido (Fase 3) | Respuestas de un agente elegido en «Pedir a», con streaming y cancelación | [agent-execution.md](agent-execution.md) |
| Conversación dirigida y multiagente | Propuesto (R1–R2) | Menciones, destinatario por intervención, «Equipo», discusiones acotadas, sesión de reunión en memoria | [voice-meetings.md](voice-meetings.md) |
| Voz | Propuesto (R3–R5) | Voz por agente, push-to-talk, subtítulos | [voice-meetings.md](voice-meetings.md), [voice-engine.md](../architecture/voice-engine.md) |
| Trabajo real | Planificado (Fases 4–5, R6) | Tareas con worktree vinculadas a la sala (`tasks.room_id`), pantalla compartida con terminal y diff reales. Los mensajes siguen siendo solo conversación | [agent-execution.md](agent-execution.md#herramientas-y-aprobaciones) |

**Decisiones relacionadas:** la sala no tiene columna de objetivo y el prompt usa su nombre (D11); las respuestas de agentes no tienen el límite de 4000 caracteres (D10). Ver [decisions.md](decisions.md).

**Prioridad:** Must Have
**Métrica de éxito:** porcentaje de workspaces que crean una sala con al menos un agente y envían un mensaje.
