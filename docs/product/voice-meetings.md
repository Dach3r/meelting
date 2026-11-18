# Reuniones conversacionales con agentes

> **Estado:** propuesta, **no implementada** salvo la conversación dirigida por texto (R1, ✅ implementada) y lo marcado «✅ implementado». Depende de `AgentRun` (Fase 3, sin construir). Las decisiones ya tomadas por el responsable del producto están en [decisions.md](decisions.md); el resto se marca **pendiente**. Ningún cambio de esquema queda aprobado por este documento.
>
> Documentos técnicos: [motor conversacional](../architecture/conversation-engine.md) y [motor de voz](../architecture/voice-engine.md). Salas actuales: [rooms.md](rooms.md). Ejecución: [agent-execution.md](agent-execution.md).

**Problema:** una sala hoy es un chat persistente donde solo escribe el usuario. Trabajar con varios agentes especializados exige poder dirigirse a uno, al equipo o pedirles que discutan, y hacerlo también hablando, sin perder el control ni pagar por respuestas que nadie pidió.
**Objetivo:** que entrar a una sala se sienta como entrar a una reunión de trabajo (estilo Meet o Teams) donde los demás participantes son agentes de IA: el usuario habla o escribe, los agentes responden con texto y voz, y todo lo que se ve corresponde a estados reales.
**Historia de usuario:** Como responsable de un workspace, quiero reunir a mis agentes en una sala, hablarles y escuchar sus propuestas, para discutir decisiones con mi equipo de IA como lo haría con personas, manteniendo el control de quién habla y de lo que se ejecuta.

## 1. Principios

- **El usuario dirige la reunión.** Siempre tiene prioridad: puede interrumpir, detener y cambiar de tema en cualquier momento.
- **Nada simulado.** «Hablando», «pensando», «no disponible» o «error» reflejan estados reales del núcleo. No hay animaciones de actividad sin actividad.
- **El texto es la fuente de verdad.** La voz es una forma de entrada (se transcribe a texto) y de salida (se sintetiza desde texto). Toda la reunión puede usarse sin voz.
- **Conversación ≠ autorización.** Una frase hablada o escrita nunca autoriza a ejecutar comandos, escribir archivos ni modificar repositorios; eso queda bajo las reglas de aprobación de las Fases 4–5.
- **Coste y turnos acotados.** Ningún mensaje despierta a todos los agentes por defecto, y ningún agente puede iniciar turnos por sí mismo.
- **Operación híbrida.** Cada agente usa su proveedor y modelo (nube o local); los motores de voz se eligen aparte. Se pueden mezclar en una misma reunión. Ver §7.

## 2. Qué cambia respecto a la sala actual

| Concepto | Hoy (✅ implementado) | Con esta propuesta |
|---|---|---|
| Sala (`rooms`) | Persistente: nombre, participantes, historial | Igual. Sigue siendo la unidad de contexto |
| Mensajes (`messages`) | Inmutables, del usuario; `message:created` | Igual. Se añaden mensajes de agentes vía `AgentRun` |
| Sesión de reunión | No existe | Estado **temporal** del núcleo mientras la sala está abierta; salir no borra nada |
| Destinatario | No existe (el selector «Pedir a» llega con `AgentRun`) | Por intervención: un agente, varios, el equipo o una discusión |
| Voz | No existe | Entrada push-to-talk y salida con voz por agente, opcionales |

## 3. Modos conversacionales

**Decisión aprobada:** el modo no se guarda ni en la sala ni en la sesión; se resuelve **por intervención**, según el destinatario. Cambiar de modo es elegir otro destinatario.

| Destinatario | Modo | Quién responde |
|---|---|---|
| Ninguno | Mensaje sin respuesta | Nadie (comportamiento actual). Ver «destinatario por defecto» abajo |
| Un agente (selector o mención) | **Dirigido** | Solo ese agente |
| Varios agentes nombrados | **Ronda** | Cada uno una vez, en el orden nombrado |
| «Equipo» | **Reunión** | El coordinador elige quién aporta (§4) |
| «Discusión» entre agentes nombrados | **Discusión** | Los nombrados alternan hasta el límite (§4) |

- **Modo dirigido** es el inicial recomendado y el primero que se construye: el usuario controla quién responde y no hay ejecuciones innecesarias.
- **Preguntar al equipo** solo es posible eligiendo «Equipo» (selector o vocativo «Equipo», «Team», «todos»). Nunca se activa por omisión.
- **Destinatario por defecto (✅ implementado, D19):** si el usuario no nombra a nadie y el último agente que respondió con éxito en la sala durante esta ejecución de la app sigue participando, el destinatario propuesto es ese agente («Respondiendo a Backend»), visible y modificable antes de enviar. Resuelve seguimientos como «Explícame mejor lo de PKCE». Sin sesión previa, no hay destinatario.

### Menciones y ambigüedad

- **Texto:** `@` abre el selector de participantes; también se reconoce el nombre del agente al inicio de la frase («Backend, …»).
- **Voz:** la transcripción se analiza igual: vocativo al inicio («Backend, ¿cómo…?»), nombres o roles de participantes, y «equipo».
- La comparación ignora mayúsculas y tildes y acepta el nombre o el rol del agente (por ejemplo «Backend» coincide con «Backend Developer» si es único).
- **Ambigüedad:** si una referencia coincide con dos o más participantes (dos agentes «Backend»), el núcleo no elige: la intervención queda como borrador y la UI pide elegir entre los candidatos. Nunca se ejecuta a un agente «probable».
- Una referencia a alguien que no participa en la sala no se resuelve (los agentes de otras salas o workspaces no existen para la reunión).

## 4. Coordinación de turnos

**Decisiones aprobadas:** coordinador **híbrido** con **reglas primero e IA opcional**; **discusiones acotadas** a un máximo de **4 intervenciones automáticas**, solo si el usuario las pide.

- Las reglas (deterministas, en Rust) resuelven: destinatario explícito, menciones, rondas, límites, disponibilidad, cancelaciones y fin de discusión por límite.
- Solo ante «Equipo» se pide **sugerencia** a un modelo coordinador elegido por el usuario en Ajustes. La sugerencia es una lista ordenada de participantes que las reglas validan (solo participantes disponibles, sin repetir, dentro del límite). Si no hay modelo coordinador configurado, no está disponible, falla o devuelve algo inválido, se usa la **ronda fija** (orden de la sala) con la opción de que cada agente pase.
- **Pasar (pendiente, recomendado):** en el modo equipo cada agente puede responder que no tiene nada que aportar; esa respuesta no se publica como mensaje. Evita que todos hablen siempre.
- **Límite:** una intervención del usuario genera como máximo 4 respuestas automáticas de agentes. Aplicarlo también a «Equipo» (además de a la discusión) es **pendiente, recomendado**.
- **Fin de una discusión:** se alcanza el límite, un agente pasa, el usuario interviene o pulsa «Detener», o (si hay modelo coordinador) este indica que la discusión terminó. Al terminar, el turno vuelve al usuario.
- **Los agentes no convocan a otros.** Que un agente escriba «@QA, ¿qué opinas?» no ejecuta a QA. En una etapa posterior, la UI podría ofrecerlo como sugerencia que el usuario acepta (**pendiente**).
- El coordinador no ejecuta herramientas, no cambia límites ni proveedores, y no puede crear aprobaciones.

## 5. Escenarios

Los diagramas de secuencia de cada escenario están en [conversation-engine.md](../architecture/conversation-engine.md#8-diagramas-de-secuencia).

### A. Conversación dirigida con un agente

Sala con Product Owner, Backend Developer y QA Engineer. El usuario mantiene pulsado el botón de micrófono y dice «Backend, ¿cómo implementarías autenticación con OAuth en este proyecto?».

1. Mientras mantiene pulsado, el núcleo captura audio y la UI muestra «Micrófono activo».
2. Al soltar, termina la intervención (push-to-talk: no hace falta detectar el final).
3. El motor STT configurado transcribe; la UI muestra «Transcribiendo…».
4. El núcleo resuelve el destinatario: «Backend» → Backend Developer (único).
5. Se guarda el mensaje del usuario (texto transcrito) y se crea un `AgentRun` para Backend.
6. El contexto incluye instrucciones de Backend, participantes, historial reciente de la sala y la pregunta.
7. La tarjeta de Backend pasa a «Pensando…»; la transcripción muestra el texto provisional token a token.
8. Al terminar la generación, la respuesta se guarda como mensaje de Backend.
9. El motor TTS sintetiza la respuesta con la voz de Backend y se reproduce; su tarjeta muestra «Hablando».
10. Si el usuario vuelve a pulsar el micrófono, el audio se detiene en el acto (§6).
11. Al terminar la reproducción, Backend vuelve a «Disponible».

Después, «Explícame mejor lo de PKCE» (sin nombre) va a Backend por el destinatario por defecto (pendiente) y su contexto ya contiene la respuesta anterior: la conversación no se reinicia.

**Excepciones:** micrófono sin permiso, audio vacío o silencio (no se envía nada), STT no disponible (se ofrece escribir), Backend no disponible (§7), error del proveedor (línea de estado con el motivo, se puede volver a pedir).

### B. Pregunta a todo el equipo

«Equipo, ¿qué opinan de implementar autenticación mediante OAuth?»

1. Vocativo «Equipo» → modo reunión.
2. Las reglas filtran participantes disponibles. Si hay modelo coordinador, este sugiere quién aporta y en qué orden (p. ej. PO → Backend → QA); si no, ronda fija en el orden de la sala.
3. Las respuestas se generan **en secuencia**: cada agente ve lo que dijeron los anteriores (Backend puede responder a la propuesta del PO). La generación del siguiente puede empezar mientras se reproduce la voz del anterior; las voces nunca se superponen.
4. Un agente que no tiene nada que aportar pasa (pendiente) y no se publica nada.
5. «Me gusta. Backend, desarrolla tu propuesta» → modo dirigido a Backend, con todo lo anterior en su contexto.

### C. Discusión entre agentes

«Product Owner y Backend, discutan cómo deberíamos implementar el sistema de autenticación.»

1. Se reconoce una discusión entre PO y Backend: por el destinatario «Discusión» del selector o por un verbo de la lista localizada («discutan», «debatan», «discuss», «debate»). Detección exacta **pendiente**.
2. Alternan PO → Backend → PO → Backend: como máximo 4 intervenciones.
3. Cada intervención es un `AgentRun` cuyo origen es la intervención del usuario: ningún agente lanza turnos.
4. El usuario puede intervenir en cualquier momento: su intervención descarta los turnos pendientes de la discusión y se atiende primero.
5. Al alcanzar el límite, la UI muestra «Discusión terminada (4 de 4)» y ofrece «Continuar la discusión» (una nueva intervención del usuario, con otro presupuesto de 4).

### D. Interrupción humana

Mientras Backend habla, el usuario pulsa el micrófono: «Espera, explícame primero cómo funcionaría el flujo».

1. Al pulsar, el audio de Backend se detiene de inmediato y se descarta el audio pendiente.
2. Los turnos planificados que no han empezado se pausan. Al enviar la nueva intervención se descartan.
3. Si había generación en curso, se cancela al enviar la nueva intervención (**pendiente, recomendado**; alternativa: dejarla terminar en silencio).
4. Se procesa la nueva intervención con su destinatario.

**Qué queda en el historial (decisión aprobada):** si la generación terminó, el mensaje completo queda guardado aunque el audio se cortara. Durante la sesión, la UI lo marca «interrumpido». Esa marca vive solo en memoria y no se persiste. Si la generación se canceló, no hay mensaje: la ejecución queda `failed` con `error_kind = cancelled`, como ya define `AgentRun`.

| Situación | ¿Mensaje persistido? | Marca visible |
|---|---|---|
| Generado y escuchado completo | Sí | — |
| Generado, audio interrumpido | Sí, texto completo | «Interrumpido» (solo en la sesión) |
| Generado, audio nunca reproducido (descartado) | Sí, texto completo | «No reproducido» (solo en la sesión) |
| Generación cancelada | No | Línea de estado «Cancelado» |
| Generación fallida | No | Línea de estado con el motivo |

Al reabrir la sala no se sabe qué parte se escuchó: es una limitación aceptada.

### E. Participantes no disponibles

Un agente usa Ollama, pero Ollama no está en ejecución. Otro usa un proveedor en la nube sin clave.

- Cada tarjeta muestra su estado real con el motivo: «Ollama no está en ejecución» y «Falta la clave de Anthropic».
- La reunión sigue con los agentes disponibles. Un agente no disponible no se planifica en rondas ni discusiones; si se le dirige una pregunta, la UI avisa antes de enviar; si el usuario envía igualmente, la intervención se guarda y la ejecución falla de inmediato con su motivo (`provider_not_configured`, `provider_unavailable`…), como ya define [agent-execution.md](agent-execution.md).
- Alternativas ofrecidas: «Reintentar» (vuelve a comprobar), «Configurar clave», «Abrir Ollama» (solo como indicación) o dirigir la pregunta a otro agente disponible de la sala.
- **Nunca** se cambia de proveedor o modelo automáticamente. Una política de fallback explícita y configurada por el usuario queda como evolución **pendiente**.
- Hoy un agente no puede cambiar de proveedor o modelo después de crearse: `update_agent` solo edita nombre, rol, descripción, instrucciones y equipo. Ofrecer «Cambiar modelo del agente» depende de añadir esa edición.

### F. Reunión sin conexión

Sin internet, con Ollama o LM Studio funcionando:

- Siguen disponibles workspaces, equipos, salas e historial.
- Responden los agentes con modelo local en ejecución.
- La voz funciona si el STT y el TTS elegidos son locales y sus modelos están descargados.
- Las reglas deterministas del coordinador funcionan. La sugerencia por IA solo funciona si el modelo coordinador es local; si no, se usa la ronda fija.
- Los agentes y motores que dependen de la nube se muestran como «Requiere conexión». Nunca se simula su respuesta.
- No se hace ninguna llamada externa sin una configuración explícita del usuario.
- Al volver la conexión, «Reintentar» recupera su disponibilidad.

### G. Reunión con trabajo real (evolución, fases 4–5 y posteriores)

«Backend, revisa el módulo de autenticación y propón una solución.»

- **Hasta la Fase 5:** el agente responde verbalmente con lo que sabe del contexto. No lee archivos.
- **Después:** desde la reunión se crea una **tarea** (agente, repositorio, rama base) con su worktree. El agente pide permisos según [agent-execution.md](agent-execution.md#herramientas-y-aprobaciones), y la pantalla compartida muestra terminal, diff y artefactos reales. El agente puede explicar su progreso con voz.
- Crear una tarea o aprobar un comando son **acciones explícitas de la UI**, nunca el resultado de una frase hablada ni de un mensaje de otro agente.

## 6. Voz

**Decisiones aprobadas:**
- **Push-to-talk** en la primera versión.
- **Motores configurables** locales o en la nube, sin preferencia obligatoria.
- **Voz por agente** independiente de su proveedor LLM, guardada en una tabla aparte (`agent_voices`). El esquema exacto está pendiente de aprobación.

- **Entrada:** mantener pulsado el botón (o el atajo dentro de la ventana) para hablar. Al pulsar se detiene la voz de los agentes. Como el micrófono no está abierto mientras hablan, se evita el eco sin cancelación de eco en la v1.
- **Transcripción:** con el destinatario resuelto, se envía sola. Con destinatario ambiguo o ausente, queda como borrador editable en el compositor. Ambas reglas son **pendiente, recomendado**.
- **Salida:** cada respuesta se sintetiza con la voz del agente. **Solo una voz suena a la vez**; las demás esperan en cola en el orden lógico de la conversación.
- **Controles:** silenciar a los agentes (la conversación sigue en texto), subtítulos, detener, volumen y, si es viable, dispositivo de entrada y salida.
- **Reproducir de nuevo:** cualquier mensaje de un agente se puede volver a escuchar (se sintetiza desde el texto guardado).
- **Idiomas:** español e inglés. Por defecto el STT usa el idioma del perfil. Cada agente necesita una voz por idioma: **pendiente** si `agent_voices` se indexa por agente e idioma.
- **Audio:** no se guardan grabaciones. El audio del micrófono vive en memoria hasta transcribirse y luego se descarta. El audio sintetizado no se guarda. Política **propuesta, pendiente de confirmar**.
- **Transparencia:** con un motor en la nube, la UI indica que el audio sale de la máquina hacia ese proveedor. Las voces sintéticas se identifican como generadas por IA, como exigen algunos proveedores.

Detalle técnico y comparativa de motores: [voice-engine.md](../architecture/voice-engine.md).

## 7. Disponibilidad y operación híbrida

Meelting es una app de escritorio sin backend propio. Trabaja con servicios en la nube y con modelos y motores instalados en el equipo, y el usuario elige qué usa cada agente y cada motor de voz. **No hay preferencia obligatoria por local ni por nube.**

Estados de disponibilidad de un agente o de un motor de voz:

| Estado | Significado | Ejemplo |
|---|---|---|
| Requiere configuración | Falta lo mínimo para usarlo | Proveedor en la nube sin clave; modelo de voz local no descargado |
| Configurado | Tiene lo necesario, sin comprobar aún | Clave guardada, sin verificar en esta sesión |
| Disponible | Comprobado y respondiendo | Ollama en ejecución con el modelo del agente |
| No disponible | Comprobado y falla | Ollama apagado; modelo ausente; clave rechazada |
| Requiere conexión | Depende de internet y no hay conexión | Agente con Claude sin red |

- Una clave guardada no significa «Disponible».
- Los estados se calculan con comprobaciones reales y con el resultado de las ejecuciones. «Reintentar» vuelve a comprobar.
- **Fallback:** no se cambia de proveedor ni de modelo de forma silenciosa. Si se pierde la conexión, el agente con Claude queda «Requiere conexión»: se informa, se permite reintentar y se ofrecen agentes disponibles de la sala. Una política de fallback explícita es **pendiente**.

## 8. Experiencia visual

Coherente con Grafito: mismos tokens, componentes y tono que la sala actual. Sin controles decorativos: cada control corresponde a una función real de su etapa.

```
┌──────────────────────────────────────────────────────────────────────┐
│ ← Salas   Autenticación OAuth   ● En reunión   4 participantes   ⋯   │  encabezado
├───────────────────────────────────────────────┬──────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐        │ Conversación         │
│  │  (Tú)   │  │   PO    │  │ Backend │        │ ─ mensajes ─         │
│  │ Superv. │  │Disponib.│  │▶Hablando│        │ Backend (escribiendo)│
│  └─────────┘  └─────────┘  └─────────┘        │ …texto provisional…  │
│  ┌─────────┐                                  │                      │
│  │   QA    │      [subtítulo de quien habla]  │ Para: [Backend ▾]    │
│  │Pensando…│                                  │ [compositor     ] ⏎  │
│  └─────────┘                                  │                      │
├───────────────────────────────────────────────┴──────────────────────┤
│ 🎙 Mantén para hablar   🔊 Voces   CC Subtítulos   ■ Detener   Salir   ⚙ │  barra inferior
└──────────────────────────────────────────────────────────────────────┘
```

- **Encabezado:** volver, nombre de la sala, estado de la reunión («En reunión» / «Detenida»), número de participantes y menú (ajustes de audio, eliminar sala). Eliminar sala hoy está en la barra inferior; se propone moverla al menú.
- **Tarjetas:** avatar, nombre, rol y estado. «Hablando» lleva anillo, icono y texto (no solo color). «Pensando…» solo aparece con una ejecución activa. «No disponible» muestra el motivo. Con más de 6 agentes, el que habla pasa a primer plano.
- **Panel lateral:** conversación persistida y texto provisional (con estilo de provisional), marcas «interrumpido» o «no reproducido», líneas de estado de cada ejecución, compositor con destinatario («Para: Backend · Equipo · Discusión»).
- **Barra inferior:** micrófono push-to-talk, silenciar voces, subtítulos, «Detener» (cancela todo lo pendiente de la sala), salir de la reunión y ajustes de audio. Cada control aparece en la etapa que lo hace real.
- **Responsive** (ventana mínima 800×600): en anchos grandes, cuadrícula y panel lateral fijo (como hoy). En anchos menores, el panel se pliega a un cajón o a pestañas «Participantes · Conversación». Cada zona tiene su propio scroll y el compositor queda siempre visible.
- **Teclado:** todo se opera con teclado. El atajo de push-to-talk funciona solo con la ventana enfocada y fuera del compositor (sin atajo global). Atajo concreto **pendiente**.
- **Accesibilidad:** los mensajes nuevos se anuncian con `aria-live="polite"` y los errores con `assertive`; los estados tienen texto; se respeta `prefers-reduced-motion`; los subtítulos están siempre disponibles y toda acción hablada tiene equivalente escrito.

### Estados vacíos y de error

| Caso | Experiencia |
|---|---|
| Sala sin participantes | Se puede escribir; los controles de voz y destinatario quedan desactivados y se invita a crear una sala con agentes (una sala creada no se edita) |
| Sala sin mensajes | Sugerencia de cómo dirigirse a un agente o al equipo |
| Micrófono sin permiso | Botón de micrófono desactivado con «Permiso de micrófono denegado» y cómo concederlo en el SO; el texto sigue disponible |
| Ningún modelo disponible | Tarjetas con el motivo; el compositor guarda mensajes sin destinatario |
| STT no configurado | El micrófono lleva a Ajustes › Voz |
| TTS no configurado | Voces desactivadas, la conversación sigue en texto |
| Reunión detenida | «Detenida»; nada pendiente; el usuario puede seguir escribiendo |
| Agente no disponible | Estado y motivo en la tarjeta; se excluye de rondas |

## 9. Criterios de aceptación (por etapa; ver [roadmap.md](roadmap.md))

**Conversación dirigida (texto)**
- [ ] Solo se puede dirigir una intervención a agentes participantes de la sala; un id ajeno o de otro workspace falla sin guardar nada.
- [ ] Una mención ambigua no ejecuta a nadie y pide elegir.
- [ ] El contexto de cada agente incluye los mensajes de los demás con su nombre y rol, y conserva sus propias instrucciones.
- [ ] Un mensaje de un agente nunca se presenta ni se envía al modelo como si fuera del usuario.

**Coordinación (texto)**
- [ ] Una intervención genera como máximo 4 respuestas automáticas.
- [ ] Ningún mensaje de un agente crea una ejecución.
- [ ] Una intervención del usuario descarta los turnos pendientes de la anterior.
- [ ] «Detener» cancela todo lo pendiente de la sala de forma idempotente; un evento tardío no reactiva nada.
- [ ] Sin modelo coordinador, o si su sugerencia es inválida, se usa la ronda fija.
- [ ] Un agente no disponible no se planifica.

**Voz de salida**
- [ ] Nunca suenan dos voces a la vez.
- [ ] Silenciar las voces no altera la conversación en texto.
- [ ] Salir de la reunión, cambiar de workspace o cerrar la ventana detiene el audio en menos de un objetivo a medir.
- [ ] No se guarda audio sintetizado.

**Voz de entrada**
- [ ] El micrófono solo está abierto mientras se mantiene pulsado y la UI lo indica.
- [ ] Pulsar el micrófono detiene la voz de los agentes.
- [ ] Audio vacío o silencio no crea mensajes.
- [ ] Sin permiso, sin dispositivo o sin STT, el texto sigue funcionando y se informa la causa.
- [ ] No se guarda audio del micrófono.

**Transversales**
- [ ] Todo en `es` y `en`; estados accesibles sin depender del color.
- [ ] Ninguna clave en eventos, logs ni transcripciones.
- [ ] Con solo motores y modelos locales disponibles, la reunión funciona sin conexión.

## 10. MVP propuesto

**Primera reunión hablada demostrable** (al final de la etapa R4 del [roadmap](roadmap.md#pista-r--reuniones-conversacionales)): modo dirigido y modo equipo por texto con el coordinador híbrido, voz de salida con un motor por plataforma y entrada push-to-talk con un motor STT. Quedan fuera del MVP la escucha continua, el TTS por frases en streaming, la cancelación de eco, speech-to-speech y las herramientas.

## 11. Fuera de alcance

- Video real, cámaras, WebRTC entre personas, participantes humanos adicionales.
- Sincronización remota, cuentas, servicios propios en la nube.
- Grabaciones de audio persistidas.
- Memoria a largo plazo y resúmenes automáticos (pendiente de aprobar su comportamiento y su persistencia).
- Agentes que hablan espontáneamente o se convocan entre sí.
- Cambio automático de proveedor o de modelo.

## Dependencias
- `AgentRun` con streaming y cancelación ([agent-execution.md](agent-execution.md)).
- Salas y mensajes ✅ ([rooms.md](rooms.md)).
- Tabla `settings` (planificada) para el modelo coordinador y las preferencias de voz; tabla `agent_voices` (propuesta).
- Spikes de audio y voz ([voice-engine.md](../architecture/voice-engine.md#9-spikes)).

**Prioridad:** Should Have (después de `AgentRun`).
**Métrica de éxito:** porcentaje de salas con al menos una intervención a «Equipo» o a varios agentes, tasa de intervenciones por voz frente a texto, tasa de interrupciones y de discusiones que llegan al límite, y tiempo hasta el primer audio (medido localmente).
