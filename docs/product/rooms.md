# Salas de trabajo

> **Estado:** planificado (Fase 3). Comportamiento portado del producto Rails, donde ya estaba implementado.

**Problema:** el usuario tiene agentes, pero no existe un lugar donde reunirlos.
**Objetivo:** que el usuario cree salas, asigne agentes y converse en un historial persistente. La sala es la unidad de contexto de trabajo: los equipos organizan agentes; las salas los reúnen para trabajar juntos. Los agentes asignados responden cuando el usuario les dirige una instrucción ([agent-execution.md](agent-execution.md)).
**Historia de usuario:** Como responsable de un workspace, quiero crear una sala, asignarle agentes y conversar en ella, para dejar listo el contexto de trabajo antes de que los agentes ejecuten.

## Alcance y exclusiones

**Incluye**
- Tabla `rooms` perteneciente a `workspaces`, con nombre obligatorio.
- Participación de agentes (`room_agents`): un agente puede estar en varias salas y una sala puede tener varios agentes, de cualquier equipo del mismo workspace.
- Crear, ver y eliminar salas. Los participantes se eligen al crear con un buscador por nombre; los elegidos quedan como fichas que se quitan con su cruz o con Retroceso; una sala creada no se edita.
- Se permite una sala sin agentes.
- Mensajes (`messages`) del usuario y de agentes, con historial persistente por sala. La entrega en vivo usa **eventos Tauri** emitidos por el núcleo. Los mensajes son inmutables: sin edición ni eliminación individual.
- Pantalla de la sala como reunión: una tarjeta por participante (el usuario y cada agente con rol y estado: disponible o pensando), la conversación como transcripción en un panel lateral con el compositor, y una barra inferior con el número de agentes y la acción de eliminar.
- Listado de salas con tarjetas del tema oscuro Grafito.
- Selector de agente en el compositor para pedirle trabajo.

**Excluye**
- Archivado y edición de salas, edición o borrado de mensajes, paginación del historial (se cargan los últimos 100).
- Miembros humanos adicionales.

## Flujo funcional
1. El usuario abre **Salas** y ve sus salas o un estado vacío. El listado va ordenado por actividad reciente (último mensaje; la creación si no tiene mensajes): la primera se destaca con su estado, sus agentes y una vista previa de sus últimos 3 mensajes, y el resto aparece como tarjetas con estado y última actividad.
2. Crea una sala: nombre y, opcionalmente, agentes.
3. Entra a la sala y ve a los participantes como tarjetas y la conversación a un costado, junto a un panel de pantalla compartida vacío (futuro panel de tarea). Con agentes, el compositor ofrece «Pedir a»; sin agentes no muestra selector y la sala sugiere asignarlos.
4. Escribe un mensaje: se guarda y aparece en la conversación.
5. Elimina la sala con confirmación; se eliminan también sus mensajes y participaciones.

## Criterios de aceptación
- [ ] Crear y eliminar salas del workspace actual.
- [ ] Asignar agentes de cualquier equipo al crear la sala; un agente no se duplica en una sala.
- [ ] Un agente de otro workspace no puede asignarse, ni manipulando los argumentos del comando.
- [ ] Una sala de otro workspace no se puede leer ni escribir (error `not_found` del núcleo).
- [ ] El autor de un mensaje del usuario lo fija el núcleo, nunca el argumento del comando.
- [ ] Los mensajes persisten y se emiten a la UI mediante `message:created`.
- [ ] Estados vacíos, errores y confirmaciones, en `es` y `en`.

## Reglas de negocio
- `name` obligatorio, máximo 80 caracteres.
- `body` del mensaje obligatorio, máximo 4000 caracteres.
- Sala y agente de una participación deben pertenecer al mismo workspace; se valida en la capa de servicios del núcleo.
- Eliminar una sala elimina sus mensajes y participaciones. Eliminar un agente elimina sus participaciones, salvo que tenga mensajes o ejecuciones: entonces no se puede eliminar.
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
