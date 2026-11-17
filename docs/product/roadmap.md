# Alcance y roadmap

> Las fases son una **propuesta**, pendiente de aprobación. Estimaciones orientativas para 1–2 personas; se ajustan tras la Fase 0.

## Estado actual

Fase 0 en curso: scaffold Tauri 2 (React + TypeScript + Vite), spike de keychain y spike de `git worktree` + PTY con salida en vivo (validado en los tres SO desde CI) y spike de streaming SSE (Chat Completions, cubre Ollama y nubes compatibles; probado con servidor simulado, falta una prueba real con Ollama y con una nube). Pendientes: medir arranque en frío e instalador. Nada del producto está implementado todavía. Ver [../architecture/overview.md](../architecture/overview.md).

## Fase 0 — Spike de validación (1 semana)

**Objetivo:** probar lo más riesgoso antes de comprometerse.

- Proyecto Tauri 2 con UI mínima.
- Llamada con streaming a Ollama y a un proveedor en la nube.
- Crear un `git worktree`, ejecutar un comando con PTY y mostrar la salida en vivo.
- Guardar y leer una clave en el keychain.
- Compilar e instalar en macOS, Windows y Linux desde CI.

**Criterios de salida:** funciona en los tres SO; arranque en frío < 2 s; instalador < 30 MB; sin bloqueos en Windows (ConPTY, rutas, WebView2) ni Linux (WebKitGTK). Si Linux falla, se evalúa Electron.

## Fase 1 — Fundamentos (2 semanas)

- CI en la matriz de 3 SO, `clippy`, `rustfmt`, lint y plantilla de commits.
- SQLite, migraciones y repositorios con los `CHECK` del modelo de datos.
- Tokens de diseño Grafito, atoms y molecules base, i18n `es`/`en`.
- Primer arranque (idioma y workspace), shell (riel + topbar) y cambio de workspace.

**Resultado esperado:** crear y cambiar de workspace con persistencia y ambos idiomas.

## Fase 2 — Equipos y agentes (2 semanas)

- Organigrama con nodos arrastrables y posiciones persistidas.
- Catálogo de plantillas, contratar, crear, editar, eliminar y mover agentes.
- Selector de proveedor y modelo desde `llm.yml`.

**Resultado esperado:** paridad funcional con [agent-catalog.md](agent-catalog.md).

## Fase 3 — Salas y ejecución con streaming (3 semanas)

- Salas, participantes y chat persistente ([rooms.md](rooms.md)).
- Adaptadores `openai`, `anthropic`, `deepseek`, `minimax` y `local`.
- `AgentRun` con estados, reintentos, idempotencia, expiración al arrancar y cancelación.
- Claves de IA en keychain con verificación previa.

**Resultado esperado:** todos los criterios de [agent-execution.md](agent-execution.md), más streaming y cancelación; funciona offline con Ollama.

## Fase 4 — Tareas y worktrees (2 semanas)

- Tareas con repositorio y rama base; creación y limpieza de worktrees; reconciliación de huérfanos; límite de concurrencia.
- Panel de tarea con diff de solo lectura.

**Resultado esperado:** dos tareas en paralelo sobre el mismo repo sin interferirse; descartar limpia worktree y rama.

## Fase 5 — Herramientas y aprobaciones (3 semanas)

- Bucle de herramientas con `tool use` nativo, path jail, entorno saneado, timeouts.
- `tool_calls`, `approvals`, diálogo de aprobación y terminal en vivo.
- Antes de empezar: propuesta en formato [feature-template.md](feature-template.md) para tareas y aprobaciones.

**Resultado esperado:** el agente completa una tarea simple pidiendo aprobación; ninguna escritura sale del worktree.

## Fase 6 — Docker opcional (1–2 semanas)

- Detección, ejecución en contenedor con el worktree montado, sin red por defecto, limpieza al descartar.

## Fase 7 — Endurecimiento y lanzamiento (2 semanas)

- Firma y notarización, instaladores, actualizador opcional.
- Pruebas manuales en los tres SO, accesibilidad y rendimiento.
- Exportar y borrar datos desde Ajustes.

## Qué NO construir

- Cuentas, login, sincronización, colaboración multiusuario y backend remoto.
- Integraciones externas (GitHub, Slack, ClickUp), colaboración y delegación entre agentes.
- Facturación, créditos y marketplace.
- Videollamadas reales, avatares hiperrealistas, animaciones 3D.
- Automatizaciones sin permisos ni supervisión.

Prioridad: validar que el usuario obtiene valor real trabajando con agentes de IA sobre su propio código.
