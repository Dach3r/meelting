# Oficina virtual

> **Estado:** visión; todo está planificado. El producto Rails ya validó sala, participantes, chat y ejecución; aquí se reconstruyen en la app de escritorio (ver [../architecture/overview.md](../architecture/overview.md)).

La interfaz principal está inspirada en una sala de videoconferencia. En lugar de cámaras de personas, muestra agentes de IA mediante avatares, estados de actividad y superficies visuales que representan lo que hacen.

## Elementos

### Agentes de IA

Cada agente es un miembro del equipo con un rol específico. Plantillas planificadas para la v1 (ver [agent-catalog.md](agent-catalog.md)):

| Rol | Responsabilidad |
|---|---|
| Product Owner | Analiza requerimientos, organiza prioridades, define criterios de aceptación. |
| Software Developer | Implementa funcionalidades, revisa código, propone cambios. |
| QA Engineer | Identifica errores, define pruebas, verifica resultados. |
| DevOps Engineer | Analiza infraestructura, propone operaciones. |
| Project Manager | Coordina tareas, identifica bloqueos, mantiene seguimiento. |

### Sala de trabajo

Espacio donde el usuario observa a los agentes, participa en conversaciones y supervisa su actividad: objetivo, participantes como tarjetas con su estado en vivo y conversación persistente como transcripción de la reunión. El usuario dirige una instrucción a un agente asignado y ve su respuesta llegar token a token (ver [rooms.md](rooms.md) y [agent-execution.md](agent-execution.md)).

### Pantalla compartida

Superficie visual para inspeccionar acciones, resultados y artefactos del trabajo ejecutado. En el producto de escritorio será el **panel de tarea**: estado del worktree, terminal en vivo (`TerminalPanel`) y diff (`DiffViewer`).

**Regla:** la visualización debe corresponder a actividad real verificable. No simular acciones que nunca ocurrieron.

### Chat colaborativo

Canal donde usuario y agentes intercambian instrucciones, resultados, preguntas y actualizaciones.

### Centro de aprobaciones

Diálogo (`ApprovalDialog`) que muestra el comando exacto que un agente quiere ejecutar. El usuario aprueba una vez, aprueba para la tarea o deniega. Ver [agent-execution.md](agent-execution.md#herramientas-y-aprobaciones).

## Workspace

Espacio organizativo donde el usuario administra su equipo de agentes. Contiene:

- Nombre del workspace.
- Equipos y agentes.
- Salas de trabajo.
- Tareas con worktree.
- Claves de IA y configuración.

No hay miembros humanos adicionales: la app es de un único usuario local.

## Dashboard

Puerta de entrada al workspace. La pantalla de inicio es **Equipos** (organigrama). En la visión permitirá acceder a salas, ver agentes, consultar actividad reciente, tareas pendientes y aprobaciones.

Primer MVP: solo capacidades respaldadas por funcionalidades reales.

## Sala de trabajo con agentes — ejemplo de flujo

Solicitud del usuario: *"Añade validación al formulario de registro y ejecuta los tests."*

1. El usuario crea una tarea sobre su repositorio; la app crea una rama y un worktree.
2. El Developer lee el código y propone cambios dentro del worktree.
3. El agente pide aprobación para ejecutar `npm test`; el usuario la concede.
4. El usuario observa el progreso en vivo y revisa el diff.
5. El usuario conserva la rama o descarta el worktree.

Los agentes deben completar trabajo real, no limitarse a representar una conversación ficticia. La delegación de tareas entre agentes sigue fuera del alcance de la v1. La conversación coordinada entre agentes (preguntas al equipo y discusiones acotadas, siempre iniciadas por el usuario) y la voz se proponen en [voice-meetings.md](voice-meetings.md).
