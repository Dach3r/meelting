# Salas de trabajo

> **Estado:** construido (Fase 3) sin ejecución de agentes: salas, participantes, historial y envío de mensajes del usuario. El estado de los agentes (disponible o pensando), el selector «Pedir a» y el panel de pantalla compartida llegan con `AgentRun` ([agent-execution.md](agent-execution.md)). Comportamiento portado del producto Rails.

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
- Pantalla de la sala como reunión: una tarjeta por participante (el usuario y cada agente con su rol), la conversación como transcripción en un panel lateral con el compositor (Intro envía, Mayús+Intro salta de línea), y una barra inferior con el número de agentes y la acción de eliminar. El estado de cada agente llega con `AgentRun`.
- Listado de salas con tarjetas del tema oscuro Grafito.
- Selector de agente en el compositor para pedirle trabajo (con `AgentRun`, aún no construido).

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
- `body` del mensaje obligatorio (el núcleo recorta los espacios de los extremos). Máximo 4000 caracteres solo para los mensajes del usuario; las respuestas de agentes no se truncan (sus límites los fijará el motor de ejecución). La UI, el núcleo y SQLite cuentan igual: puntos de código Unicode (`Array.from` en la UI, `chars()` en Rust, `length()` en SQLite) sobre el texto recortado; un emoji simple cuenta 1 y uno compuesto con ZWJ cuenta cada punto de código. Si el mensaje pasa de 4000, el compositor desactiva el envío y muestra el aviso, sin cortar el texto.
- Sala y agente de una participación deben pertenecer al mismo workspace; se valida en el servicio y lo refuerzan claves foráneas compuestas con `workspace_id`.
- Un agente solo puede tener mensajes en salas donde participa (clave foránea `messages (room_id, agent_id) → room_agents`).
- Eliminar una sala elimina sus mensajes y participaciones. Eliminar un agente elimina sus participaciones, salvo que tenga mensajes o ejecuciones: entonces no se puede eliminar. Hoy lo impide la base de datos (el núcleo devuelve `database`); el error propio `agent_has_messages` llega con `AgentRun`, cuando los agentes puedan escribir.
- Todas las consultas parten del workspace actual.
- Un `agent_ids` con agentes de otro workspace (o inexistentes) falla y no cambia ninguna participación: la asignación es atómica (una transacción SQLite).

## Dependencias
- Agentes y equipos ([agent-catalog.md](agent-catalog.md)).
- SQLite y eventos Tauri.

## Evolución prevista
- Tareas con worktree vinculadas a la sala (`tasks.room_id`); los mensajes siguen siendo solo conversación.
- Panel de pantalla compartida alimentado por actividad real verificable (terminal y diff de la tarea).

**Prioridad:** Must Have
**Métrica de éxito:** porcentaje de workspaces que crean una sala con al menos un agente y envían un mensaje.
