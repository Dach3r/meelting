# Salas de trabajo

> **Estado:** construido (Fase 3): salas, participantes, historial, envío de mensajes del usuario y, con `AgentRun`, el selector «Pedir a», las respuestas de los agentes en streaming y el estado «Pensando…» de cada agente ([agent-execution.md](agent-execution.md)). Los agentes pueden entrar y salir de una sala ya creada (D24). Pendiente: el panel de pantalla compartida. Comportamiento portado del producto Rails.

**Problema:** el usuario tiene agentes, pero no existe un lugar donde reunirlos.
**Objetivo:** que el usuario cree salas, asigne agentes y converse en un historial persistente. La sala es la unidad de contexto de trabajo: los equipos organizan agentes; las salas los reúnen para trabajar juntos. Los agentes asignados responden cuando el usuario les dirige una instrucción ([agent-execution.md](agent-execution.md)).
**Historia de usuario:** Como responsable de un workspace, quiero crear una sala, asignarle agentes y conversar en ella, para dejar listo el contexto de trabajo antes de que los agentes ejecuten.

## Alcance y exclusiones

**Incluye**
- Tabla `rooms` perteneciente a `workspaces`, con nombre obligatorio.
- Participación de agentes (`room_agents`): un agente puede estar en varias salas y una sala puede tener varios agentes, de cualquier equipo del mismo workspace.
- Crear, ver y eliminar salas. Los participantes se eligen al crear con un buscador por nombre o rol; los elegidos quedan como fichas que se quitan con su cruz o con Retroceso (con el buscador vacío); Intro añade la primera coincidencia. El nombre de una sala creada no se edita, pero sus agentes sí cambian: entran y salen como en una reunión real (D24). Si llegan agentes repetidos, el núcleo los deduplica conservando el orden de la primera aparición, sin error.
- Se permite una sala sin agentes.
- Invitar y quitar agentes de una sala existente. «Invitar agentes» abre el mismo buscador con los agentes del workspace que no están presentes; la cruz de cada mosaico lo quita sin confirmación (se puede volver a invitar). Quien sale deja de ser participante, pero su historial se conserva con su nombre; si estaba respondiendo, su ejecución se cancela.
- Mensajes (`messages`) del usuario y de agentes, con historial persistente por sala. La entrega en vivo usa **eventos Tauri** emitidos por el núcleo. Los mensajes son inmutables: sin edición ni eliminación individual.
- Pantalla de la sala como videollamada, a pantalla completa (cubre el riel y la barra superior; «Salas» vuelve al listado): un mosaico por participante (el usuario y cada agente con su rol), la conversación como transcripción en un panel lateral con el compositor (Intro envía, Mayús+Intro salta de línea), el número de agentes en la cabecera y una barra de controles flotante con «Invitar agentes», «Comprobar disponibilidad» y «Eliminar sala». Mientras un agente tiene una ejecución activa en la sala, su mosaico muestra «Pensando…» y un anillo.
- Lobby de participantes: listado de salas como cuadrícula de tarjetas del tema Grafito, cada una con nombre, fecha de actividad, avatares solapados de hasta tres agentes con contador del resto y botón para entrar a todo el ancho.
- Compositor con menciones `@`, selector «Pedir a» («Según el mensaje», «Nadie» o un agente) y destinatario propuesto ([conversation-engine.md](../architecture/conversation-engine.md#r1-implementado)). Bajo la instrucción aparece el agente con su estado (en cola, pensando, escribiendo o reintentando), el texto en vivo y «Cancelar», o el error traducido. La respuesta guardada muestra proveedor, modelo y tokens solo en su pie.
- Cada mensaje muestra la hora, y al pasar el cursor, la fecha completa. Si el rol de un agente coincide con su nombre, no se repite.

**Excluye**
- Archivado y cambio de nombre de salas, edición o borrado de mensajes, paginación del historial (se cargan los últimos 100).
- Miembros humanos adicionales.

## Flujo funcional
1. El usuario abre **Salas** y ve sus salas o un estado vacío. El listado va ordenado por actividad reciente (`rooms.updated_at`, que es la creación y se actualiza con cada mensaje; desempate por `id`): la primera se destaca como una mesa de reunión (una tarjeta por participante, el usuario incluido, y la vista previa de sus últimos 3 mensajes) y el resto aparece sobre un plano punteado como salas con sus agentes sentados (hasta 3, con «+N» para el resto) y su última actividad, más un hueco discontinuo «Nueva sala».
2. Crea una sala: nombre y, opcionalmente, agentes.
3. Al crearla entra directamente; ve a los participantes como mosaicos y la conversación (últimos 100 mensajes) a un costado.
4. Escribe un mensaje: el núcleo lo guarda y actualiza la actividad de la sala en una transacción, lo devuelve y, tras el commit, emite `message:created`. La UI lo añade desde el resultado o el evento sin duplicarlo (por `id`).
5. Invita agentes desde el control «Invitar agentes» o quita uno con la cruz de su mosaico (`add_room_agents`, `remove_room_agent`; ambos devuelven los presentes y los que salieron).
6. Elimina la sala con confirmación; se eliminan también sus mensajes y participaciones.

## Criterios de aceptación
- [x] Crear y eliminar salas del workspace actual.
- [x] Asignar agentes de cualquier equipo al crear la sala; un agente no se duplica en una sala.
- [x] Invitar agentes a una sala existente y quitarlos; quien sale conserva su historial con su nombre, no recibe instrucciones y su ejecución activa se cancela; volver a invitarlo lo reincorpora.
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
- Quitar un agente marca `room_agents.left_at` (no borra la fila) y cancela su ejecución activa en la sala en la misma transacción. Solo los participantes sin `left_at` aparecen en la sala, el listado, la disponibilidad y la resolución del destinatario; los que salieron se usan para nombrar sus mensajes en la UI y en el historial del prompt. Quitar a quien no está presente falla con `agent_not_found`; invitar a quien ya está no cambia nada.
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

**Decisiones relacionadas:** los agentes entran y salen con salida suave (D24); la sala no tiene columna de objetivo y el prompt usa su nombre (D11); las respuestas de agentes no tienen el límite de 4000 caracteres (D10). Ver [decisions.md](decisions.md).

**Prioridad:** Must Have
**Métrica de éxito:** porcentaje de workspaces que crean una sala con al menos un agente y envían un mensaje.
