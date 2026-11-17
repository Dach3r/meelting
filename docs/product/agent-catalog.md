# Catálogo de agentes IA

> **Estado:** en curso (Fase 2). Ya existen: listar equipos y agentes, crear equipos (solo nombre) y crear agentes personalizados con proveedor y modelo escritos a mano. Pendiente: catálogo de plantillas, contratar, editar, eliminar, mover, organigrama, `llm.yml` y el equipo «General» inicial. Comportamiento portado del producto Rails, donde ya estaba implementado.

**Problema:** el workspace nace vacío; el usuario no puede construir su equipo, que es la base de la oficina virtual.
**Objetivo:** que el usuario contrate agentes desde un catálogo de plantillas, los personalice y los gestione como el equipo persistente de su workspace. Cada agente tiene su propio proveedor y modelo de IA y trabaja cuando el usuario le dirige una instrucción en una sala ([agent-execution.md](agent-execution.md)).
**Historias de usuario:**
- Como responsable de un workspace, quiero explorar un catálogo de agentes especializados, para entender qué roles puedo sumar.
- Como responsable, quiero contratar un agente y ajustar su nombre, rol, descripción e instrucciones.
- Como responsable, quiero ver, editar y eliminar mis agentes.
- Como usuario nuevo, quiero un primer paso claro en Equipos, para saber cómo empezar.

## Alcance y exclusiones

**Incluye**
- Catálogo con 5 plantillas (Product Owner, Software Developer, QA Engineer, DevOps Engineer, Project Manager) como archivos YAML embebidos en la app.
- Tabla `agents` perteneciente a un workspace, con copia de los valores de la plantilla.
- Secciones separadas: **Catálogo de agentes** y **Equipos**.
- Contratar desde el catálogo, crear agentes personalizados sin plantilla, ver, editar y eliminar.
- Equipos como pantalla de inicio, con organigrama arrastrable: equipos que agrupan agentes bajo "Tú" (todo agente pertenece a un equipo), mover agentes entre equipos arrastrando su tarjeta.
- Selector de proveedor y modelo por agente, incluido el proveedor `local`.

**Excluye**
- Colaboración entre agentes, métricas, consumo de tokens y estadísticas en las tarjetas.
- Límite de agentes por workspace.
- Cambio automático del contenido de un agente al cambiar el idioma.

## Flujo funcional

1. El usuario elige "Equipos" en el riel.
2. Ve el organigrama con "Tú" en la raíz, equipos y agentes; clic en un equipo lo resalta y clic en la raíz vuelve a todos. "Nuevo equipo" abre un diálogo para nombrarlo.
3. Con un equipo seleccionado pulsa "Contratar agente" y llega al **Catálogo de agentes** de ese equipo.
4. "Ver detalles" muestra el detalle de la plantilla (avatar, rol, descripción e instrucciones por secciones). "Contratar" abre el asistente de contratación precargado: una pregunta a la vez (nombre, rol, descripción, instrucciones, proveedor y modelo), con indicador de paso, Atrás y Continuar, y el atajo «Contratar ahora con los datos de la plantilla». Si el núcleo rechaza los datos, el asistente reabre en el primer paso con el error.
5. Al confirmar se crea el agente en el workspace activo con el `template_key` de origen y se muestra su ficha con un toast.
6. Desde Equipos puede ver, editar o eliminar (con confirmación) cada agente.
7. La ficha del agente lista las salas en las que participa o un estado vacío.

**Equipos:** se crean, se renombran y se eliminan (solo sin agentes). Cada workspace nace con un equipo «General». Un agente se mueve de equipo desde su edición o arrastrando su tarjeta.

**Agente personalizado:** desde Equipos, «Crear agente personalizado» abre el formulario vacío y crea un agente con `template_key` nulo.

**Idioma:** al contratar se copian los valores del idioma activo; el agente guardado no cambia si luego cambia el idioma de la interfaz.

Se puede contratar la misma plantilla varias veces.

## Reglas de negocio
- Un agente pertenece a un único workspace; todas las consultas parten del workspace actual.
- La contratación copia los valores de la plantilla. El agente no vuelve a leer el YAML.
- `template_key` es solo trazabilidad: se fija al crear, no se cambia al editar y no condiciona el rol. Se valida contra el catálogo solo al crear.
- Un agente solo puede estar en un equipo de su mismo workspace; un equipo con agentes no se puede eliminar.
- Al eliminar un workspace se eliminan sus equipos y agentes.
- Un agente con mensajes o ejecuciones no se puede eliminar.
- Ninguna pantalla sugiere actividad que no exista.

## Modelo de datos (SQLite)

Detalle completo en [../architecture/overview.md](../architecture/overview.md#modelo-de-datos).

**`agents`:** `id` (UUID v7), `workspace_id`, `team_id` (FK `ON DELETE RESTRICT`), `template_key` (NULL si es personalizado), `name` (1–80), `role` (1–80), `description` (≤ 500), `instructions` (≤ 10 000), `provider` y `model` (no vacíos), timestamps. `(team_id, workspace_id)` es clave foránea compuesta hacia `teams`.

**`teams`:** `id`, `workspace_id`, `name` (1–80), timestamps.

**Validaciones:** se reflejan en la base con `CHECK`; `provider` y `model` se validan contra `llm.yml` solo al crear o cuando cambian (un agente cuyo modelo salió del catálogo sigue siendo editable y avisa que debe elegir otro).

**Avatar:** los agentes no tienen imagen elegible; `AgentAvatar` dibuja un círculo con las iniciales (máx. 2) de las dos primeras palabras del nombre.

**Plantillas (sin tabla):** `agent_templates/<key>.yml`, un archivo por plantilla, embebido en la app y leído con `serde_yaml`:

```yaml
key: product_owner
es:
  name: Product Owner
  role: Product Owner
  description: ...
  instructions: ...
en:
  ...
```

El contenido localizado vive solo en el YAML; los textos de interfaz van en los archivos i18n del frontend. Catálogo y agentes son conceptos independientes: el agente solo conoce la clave.

## Comandos Tauri (planificado)

| Comando | Efecto |
|---|---|
| `list_agent_templates`, `get_agent_template(key)` | Catálogo en el idioma activo |
| `list_teams`, `create_team`, `update_team`, `delete_team` | Equipos del workspace actual |
| `list_agents`, `get_agent`, `create_agent`, `update_agent`, `delete_agent` | Agentes del workspace actual |
| `move_agent(agent_id, team_id)`, `save_canvas_positions` | Organigrama |

El frontend no accede a SQLite directamente; los comandos son delgados y llaman a servicios del núcleo.

## Criterios de aceptación
- [ ] «Equipos» del riel abre el organigrama y aparece como activo solo allí.
- [ ] El catálogo muestra las 5 plantillas con rol y descripción; su detalle muestra las instrucciones.
- [ ] Contratar crea un agente en el workspace activo con los valores editados y `template_key` de origen.
- [ ] Se puede contratar la misma plantilla más de una vez y crear un agente personalizado.
- [ ] Cambiar el idioma no modifica agentes ya contratados.
- [ ] `name` y `role` compuestos solo de espacios son inválidos.
- [ ] Un agente se edita (incluidos proveedor y modelo) y se elimina con confirmación.
- [ ] Cada agente usa su proveedor y modelo, elegidos de un selector dependiente alimentado por `llm.yml`. Un proveedor sin clave se puede elegir con una advertencia.
- [ ] Un agente de otro workspace no aparece ni se puede modificar al cambiar de workspace.
- [ ] Errores de validación visibles en el formulario; textos completos en `es` y `en`.

## Pruebas
- Núcleo (`cargo test`): repositorios y `CHECK`, validaciones, `template_key` solo al crear, integridad por workspace.
- Plantillas: carga de las 5, clave inexistente, `es` y `en`, sin claves duplicadas.
- Componentes (Vitest): `AgentRow`, `AgentAvatar`, `AgentTile`, `TeamNode`, `Rail`/`NavItem` con estado activo.

## Implicaciones futuras
- **Salas:** `room_agents` ya contempla la relación sin cambiar `agents`.
- **Ejecución:** estado y consumo viven en `agent_runs`, no en columnas de `agents`.
- **Plantillas del usuario o marketplace:** se migraría el catálogo a tabla sin cambiar `agents`.

**Prioridad:** Must Have (Fase 2).
**Métrica de éxito:** % de workspaces con al menos 1 agente contratado tras el primer arranque; agentes por workspace; % de agentes personalizados.
