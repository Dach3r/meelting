# Registro de decisiones

Registro centralizado de decisiones de producto y arquitectura que no son evidentes en el código. Las decisiones técnicas anteriores siguen en [overview.md §13](../architecture/overview.md#13-decisiones-abiertas); las nuevas se registran aquí.

**Estados:**
- **Aprobada:** la tomó el responsable del producto.
- **Pendiente:** hay una recomendación, pero falta la decisión.
- **Descartada:** se evaluó y no se sigue.

Una decisión aprobada no implica que esté implementada ni aprueba cambios de esquema: cada migración sigue necesitando su propuesta y su aprobación explícita.

## Reuniones conversacionales y voz

Contexto: [voice-meetings.md](voice-meetings.md), [conversation-engine.md](../architecture/conversation-engine.md), [voice-engine.md](../architecture/voice-engine.md).

### Aprobadas (2026-11-18)

| # | Decisión | Justificación | Alternativas descartadas |
|---|---|---|---|
| D1 | **Texto primero:** `AgentRun` → conversación dirigida por texto → coordinación multiagente por texto → voz de salida → voz de entrada | El contexto y la coordinación se validan sin depender del audio; la voz se apoya en un núcleo estable | MVP hablado temprano (coordinación diseñada con menos evidencia); reuniones solo después de tareas y herramientas |
| D2 | **Coordinador híbrido con reglas primero e IA opcional:** las reglas resuelven destinatarios, menciones, límites y disponibilidad; solo ante «Equipo» se pide una sugerencia a un modelo coordinador elegido en Ajustes. Sin modelo, o si falla, se usa la ronda fija | Predecible, testeable, funciona offline y solo cuesta donde aporta | Solo reglas; coordinador 100 % IA; usar el modelo del primer agente (coste y privacidad implícitos) |
| D3 | **Discusiones entre agentes acotadas:** solo si el usuario las pide, máximo 4 intervenciones automáticas, interrumpibles; al llegar al límite el turno vuelve al usuario | Habilita el escenario C sin bucles ni costes abiertos | Límite configurable por sala (requiere persistencia); sin discusiones en el MVP |
| D4 | **Modo por intervención:** el modo (dirigido, ronda, equipo, discusión) se deriva del destinatario de cada mensaje; no se guarda | Sin esquema nuevo; cambiar de modo es elegir otro destinatario | Conmutador por sesión; columna en `rooms` |
| D5 | **Push-to-talk** en la primera versión de entrada por voz | Simplifica el fin de turno, la interrupción y el eco; evita capturas accidentales | Escucha continua con VAD; ambos desde el inicio |
| D6 | **Operación híbrida online/offline:** motores y modelos locales o en la nube, configurables por el usuario y combinables en una misma reunión, sin preferencia obligatoria. Lo local funciona sin conexión; lo que depende de la nube informa que no está disponible. Estados: requiere configuración, configurado, disponible, no disponible, requiere conexión. **Sin fallback silencioso:** nunca se cambia de proveedor o modelo sin consentimiento o una política explícita configurada | Aprovechar la nube cuando está disponible sin perder lo local cuando no lo está | Local primero con nube opcional; calidad primero (nube por defecto); solo local |
| D7 | **Interrupción:** si la generación terminó, el mensaje completo se persiste aunque el audio se cortara; la marca «interrumpido» vive solo en memoria durante la sesión. Si la generación se canceló, no hay mensaje (`failed/cancelled`) | Mantiene la inmutabilidad y la regla aprobada de `AgentRun` sin cambiar el esquema | Marca persistida (columna nueva); guardar el parcial cancelado (cambia la regla de `AgentRun`); guardar solo lo escuchado |
| D8 | **Voz por agente en una tabla aparte** (`agent_voices`), independiente del proveedor LLM; sin fila, voz por defecto estable. **El esquema concreto sigue pendiente de aprobación** | Separa identidad y presentación, admite varios motores y no toca `agents` | Columnas en `agents`; no persistir al principio |

### Aprobadas (2026-11-18, revisión para `AgentRun`)

| # | Decisión | Justificación | Alternativas descartadas |
|---|---|---|---|
| D9 | **Proveedores de la Fase 3 = catálogo real de `llm.yml`:** Anthropic, OpenAI y Groq (nube); Ollama y LM Studio (local). Sin DeepSeek, MiniMax ni un proveedor genérico `local` en esta fase. Cada proveedor usa su protocolo propio y solo las capacidades que declara el catálogo. Añadir un proveedor después es una entrada nueva en el catálogo y, solo si su protocolo no existe aún, un adaptador nuevo con uso real (antes era P14) | El catálogo ya está en el código y en el selector de modelo de los agentes: los agentes existentes solo pueden tener estos cinco proveedores. Implementar adaptadores sin agentes que los usen sería código muerto | Ampliar el catálogo con DeepSeek y MiniMax (adaptadores sin demanda real); un único `local` (el catálogo y la UI ya distinguen Ollama y LM Studio) |
| D10 | **El límite de 4000 caracteres es solo para los mensajes escritos por el usuario.** Las respuestas de los agentes no tienen límite de caracteres: su extensión la acotan los parámetros de generación que admite cada proveedor (`max_output_tokens` o equivalente) y los límites de contexto del catálogo. Una respuesta incompleta o cortada nunca se guarda recortada ni en silencio: la ejecución falla con su `error_kind` (`output_truncated`, `content_filtered`) y no se crea mensaje. Los mensajes siguen siendo inmutables y **no cambia el esquema de `messages`** (antes era P15) | Cortar por caracteres rompería respuestas válidas (código, listas) y ocultaría que faltaba texto; el límite natural de un modelo son sus tokens de salida. Es lo que ya implementan el `CHECK` de `messages` y [rooms.md](rooms.md) | 4000 caracteres para todos (respuestas cortadas o fallidas sin motivo); truncar al guardar con una marca (exige cambiar `messages`) |
| D11 | **Sin objetivo de sala por ahora:** `rooms` no cambia. `BuildPrompt` usa el nombre de la sala y no asume ninguna columna que no exista (antes era P21) | Evita un cambio de esquema sin evidencia de que el nombre no baste | Columna `goal` en `rooms` desde ya |
| D12 | **El proveedor y el modelo de un agente no se pueden cambiar todavía:** `update_agent` mantiene su limitación actual (nombre, rol, descripción, instrucciones y equipo). Se evaluará más adelante, junto con su efecto en las ejecuciones existentes (que de todos modos conservan el proveedor y el modelo copiados al crearse) | Mantener el alcance; la ejecución no lo necesita | Habilitar el cambio dentro de `AgentRun` |
| D13 | **Voz fuera del alcance inmediato:** STT, TTS y configuración de voces no se construyen con `AgentRun`, y `AgentRun` no depende del audio | Confirma D1; la voz consume mensajes persistidos y no condiciona el motor de ejecución | Diseñar `AgentRun` con ganchos de audio |

### Pendientes

| # | Decisión | Recomendación | Bloquea |
|---|---|---|---|
| P1 | ¿Se persiste la sesión de reunión (plan de turnos, último interlocutor, cola de audio, estado del micrófono)? No confundir con C1, la propuesta de índice de `agent_runs` en [conversation-engine.md §12](../architecture/conversation-engine.md#12-persistencia-y-modelo-de-datos) | No: estado en memoria del núcleo. No bloquea `AgentRun` | R2 |
| P2 | Al salir de la reunión, ¿siguen las ejecuciones en curso? | Sí (regla aprobada de `AgentRun`), pero sin turnos nuevos ni audio | R2 |
| P3 | Una intervención nueva con ejecuciones activas de la anterior | Cancelarlas (prioridad del usuario y coste) | R2 |
| P4 | ¿Se envía sola la transcripción? | Sí si el destinatario está resuelto; si no, queda como borrador | R4 |
| P5 | Destinatario por defecto = último interlocutor | Sí, visible y modificable | R1 |
| P6 | ¿El límite de 4 también se aplica a «Equipo»? | Sí: un solo presupuesto por intervención | R2 |
| P7 | Mecanismo «pasar» y `succeeded` sin mensaje | Decidir en R2. Si se aprueba, exige reconstruir `agent_runs` (SQLite no permite cambiar un `CHECK` sin copiar la tabla) | R2 |
| P8 | Cómo se pide una discusión | Destinatario explícito «Discusión» + lista localizada de verbos | R2 |
| P9 | Alcance de `settings` (global o por workspace) para el modelo coordinador y los motores | Por workspace para lo que usa claves; global para los dispositivos | R2 |
| P10 | ¿Pueden los agentes pedir la participación de otro? | No en el MVP; después, como sugerencia que el usuario acepta | R2+ |
| P11 | Distribución de modelos de voz | Descarga bajo consentimiento con catálogo y `sha256` | R3 |
| P12 | Retención de audio | Ninguna: el micrófono en memoria hasta transcribir, el TTS sin guardar | R3 |
| P13 | Dónde se reproduce el audio | En el núcleo; confirmar con S6 | R3 |
| P16 | Generación en paralelo con reproducción ordenada (prelectura) | No al principio; medir en R5 | R5 |
| P17 | Atajo de push-to-talk | Solo con la ventana enfocada, sin atajo global; tecla por definir | R4 |
| P18 | Política de fallback explícita configurable | Fuera del MVP; diseñarla con su propia propuesta | Posterior |
| P19 | Resúmenes y memoria de reuniones largas | No implementar; opción futura de resumen explícito como mensaje visible | Posterior |
| P20 | Una voz por agente e idioma (`agent_voices.locale`) | Sí | R3 |
| P22 | Alcance del límite de ejecuciones locales simultáneas | Una plaza compartida por máquina (A9 en [agent-run.md](../architecture/agent-run.md)) | Fase 3 |
| P23 | ¿Los consumos del coordinador IA cuentan para los límites diarios? | Sí | R2 |
| P24 | Licencia del TTS neuronal local en español (fonemizador GPL) | Revisión legal antes de incluir Piper o Kokoro | R3 |

Las decisiones aprobadas en la revisión de `AgentRun` sustituyen a P14 (ahora D9), P15 (D10) y P21 (D11).

### Aprobadas (2026-11-18, revisión de la propuesta de `AgentRun`)

| # | Decisión | Justificación | Alternativas descartadas |
|---|---|---|---|
| D14 | **Dirección arquitectónica de `AgentRun`:** estados `queued → running → succeeded` o `failed`, con `cancelled` y `expired` dentro de `failed`, transiciones con guarda, éxito (mensaje + `succeeded`) en una sola transacción, eventos después del commit, `agent_run:status` y `agent_run:token` con intento y secuencia, índice único `(room_id, client_request_id)`, hasta 3 intentos solo ante errores transitorios respetando `retry-after`, expiración de `queued` y `running` al arrancar, cancelación idempotente, una ejecución activa por agente y sala, control de concurrencia local y 4 simultáneas en la nube como punto de partida. Fuera del primer corte: coordinación, voz, herramientas, límites diarios y tareas | Portado de Rails y reforzado para streaming local | — |
| D15 | **L1: máximo 256 KiB de texto por respuesta**, medido en UTF-8 durante el streaming; al superarse, `failed/output_truncated` sin guardar nada y con un error traducido. Es un límite de seguridad interno, no del proveedor | Protege memoria y base ante servidores que ignoren el tope de tokens; muy por encima de cualquier respuesta válida | Sin límite técnico (depender solo del proveedor) |
| D16 | **Revisión 2 de `AgentRun`:**<br>• `client_request_id` obligatorio y generado por el cliente; una repetición devuelve la ejecución original sin llamar al proveedor; parámetros distintos → `client_request_conflict`.<br>• Buffer temporal acotado y aislado por sala.<br>• Contexto congelado en el primer intento.<br>• Cancelación idempotente filtrada por workspace.<br>• Duración máxima de 15 min (nube) y 45 min (local) desde el primer intento, que incluye reintentos y esperas y termina en `failed/timeout`.<br>• 1 ejecución local global (Ollama + LM Studio) y 4 en la nube, **liberando la plaza durante el backoff**, con límites configurables solo internamente.<br>• Presupuestos de 24 000 y 8000 caracteres y 100 mensajes; si el contexto mínimo no cabe, error controlado.<br>• Keychain existente para las pruebas, sin interfaz temporal.<br>• Eliminar el spike `stream_completion`.<br>• Errores `client_request_conflict` y `agent_has_activity`. | Cerrar los contratos antes de implementar | Conservar la plaza durante el backoff |

### Aprobadas (2026-11-18, aprobación final de `AgentRun`)

| # | Decisión | Justificación | Alternativas descartadas |
|---|---|---|---|
| D17 | **Aprobación final de `AgentRun` (revisión 3):**<br>• SQL íntegro de A1 y A13: las tres migraciones `add_id_room_id_index_to_messages`, `add_id_room_id_agent_id_index_to_messages` y `create_agent_runs`, sin tablas ni columnas adicionales; los 45 casos de integridad pasan a `cargo test`.<br>• A1b: `context_too_large` permanente, sin llamar al proveedor ni recortar instrucciones o la solicitud.<br>• A2: errores `invalid_client_request_id`, `agent_busy`, `client_request_conflict` y `agent_has_activity`.<br>• A8 y A8b: YAML de §9 con `gpt-6.1-sol` y `gpt-6.1-terra` (128 000, `openai_responses`) y `openai/gpt-oss-120b` (65 536, `chat_completions` con `max_completion_tokens`) en lugar de los modelos retirados; Anthropic sin cambios; locales con detección dinámica. Los agentes existentes no se modifican y fallan con `model_unavailable`.<br>• A8c: topes internos de salida 16 000 (nube) y 2048 (local), y L1 de 256 KiB; se pide el mínimo entre el máximo documentado y el tope interno, ajustable solo en la configuración interna del motor.<br>• Orden de los eventos de estado: posición lógica derivada de `status` y `attempts`, sin columna nueva, para que un evento tardío no sustituya a uno más reciente con el mismo intento.<br>• Reintentos, concurrencia, recuperación y cancelación según la revisión 3. | Cierra la revisión y autoriza la implementación en tres entregas (núcleo y persistencia, ejecución real, interfaz), probando primero un agente local real | Mantener los modelos retirados; topes de salida iguales al máximo del modelo |
| D18 | **Cambio explícito de proveedor y modelo de un agente** (revisa D12) como **entrega pequeña e independiente** del motor: iniciado por el usuario, validado contra el catálogo, sin migración, sin modificar las ejecuciones existentes y sin sustituciones automáticas | Los agentes con modelos retirados necesitan una salida explícita | Sustitución automática del modelo; hacerlo dentro de la implementación del motor |

### Riesgos y dependencias

Ver los riesgos en [conversation-engine.md §14](../architecture/conversation-engine.md#14-riesgos) y [voice-engine.md §12](../architecture/voice-engine.md#12-riesgos), y las dependencias de etapas en [roadmap.md](roadmap.md#pista-r--reuniones-conversacionales).
