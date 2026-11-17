# Meelting Desktop — Producto

**Categoría:** AI Workspace / Virtual AI Office / Multi-Agent Collaboration
**Plataforma:** aplicación de escritorio local-first (Tauri 2) para macOS, Windows y Linux
**Etapa:** Fase 0 — scaffold Tauri 2 con spikes de keychain, `git worktree` + PTY y streaming SSE

> **Tu equipo de inteligencia artificial trabajando contigo, en una oficina virtual.**

Meelting Desktop es una aplicación que funciona 100 % en la máquina del usuario, sin servidor ni cuenta. El usuario crea una oficina virtual con agentes de IA que trabajan de forma autónoma y ejecutan tareas reales de código en `git worktree` aislados, con modelos en la nube o locales (Ollama, LM Studio). El usuario es el responsable del equipo: supervisa, asigna objetivos, conversa y aprueba acciones.

Este producto reemplaza al SaaS Rails original (repositorio `meelting`), que queda congelado como referencia de diseño. Ver [../architecture/overview.md](../architecture/overview.md).

## Índice

| Documento | Contenido |
|---|---|
| [vision.md](vision.md) | Visión, problema, propuesta de valor, principios |
| [virtual-office.md](virtual-office.md) | Concepto de oficina virtual: agentes, salas, pantalla compartida, chat, aprobaciones |
| [agent-catalog.md](agent-catalog.md) | Catálogo de plantillas, equipos y gestión de agentes |
| [rooms.md](rooms.md) | Salas de trabajo: objetivo, participantes y conversación persistente |
| [agent-execution.md](agent-execution.md) | Ejecución de agentes: proveedores (nube y local), streaming, estados, límites y errores |
| [users.md](users.md) | Usuarios objetivo y experiencia principal (journey) |
| [roadmap.md](roadmap.md) | Fases, alcance y lo que no se construye |
| [integrations.md](integrations.md) | Integraciones previstas (GitHub, Slack, ClickUp) |
| [metrics-and-business.md](metrics-and-business.md) | Indicadores de éxito y modelo de negocio |
| [feature-template.md](feature-template.md) | Formato estándar para documentar funcionalidades y reglas de decisión |

Arquitectura técnica: [../architecture/overview.md](../architecture/overview.md).

## Estado de la información

Cada documento distingue entre:

- **Implementado:** existe hoy en el código de este repositorio.
- **Planificado:** definido en el plan de migración y aún no construido. Los documentos de funcionalidad (catálogo, salas, ejecución) describen el comportamiento **objetivo**, portado del producto Rails, donde ya fue validado.
- **Visión:** dirección del producto, sin compromiso de entrega.
- **Hipótesis:** por validar.

**Hoy está implementado:** scaffold Tauri 2 (React + TypeScript + Vite) con tres spikes: keychain del SO (guardar, leer los últimos 4 caracteres y borrar una clave), `git worktree` + PTY con salida en vivo, y streaming SSE desde un endpoint compatible con Chat Completions (Ollama o nube). Todo lo demás es planificado.

## Principio fundamental

Meelting no busca simular que una IA está trabajando. Busca permitir que IAs trabajen realmente, mientras el usuario observa, colabora y mantiene el control. La oficina virtual, los avatares y las salas son la interfaz que hace comprensible ese trabajo. El producto real es la capacidad de **delegar, coordinar, supervisar y completar tareas reales** mediante agentes de IA.
