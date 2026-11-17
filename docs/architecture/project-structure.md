# Estructura del proyecto

> **Estado:** Fase 0. Solo existen los módulos listados aquí como «✅». Lo marcado «pendiente» es la ubicación **prevista**; no se crea la carpeta hasta que haya código que la use (sin código muerto).

Reglas de capas y convenciones: [overview.md](overview.md) §3 y §5.

## Vista general

```
meelting-desktop/
├── CLAUDE.md              Instrucciones del proyecto (inglés)
├── LICENSE                FSL-1.1-MIT
├── README.md
├── docs/
│   ├── product/           Visión, roadmap, usuarios, oficina virtual, salas, agentes
│   └── architecture/      overview.md y este documento
├── .github/workflows/     CI en macOS, Windows y Ubuntu
├── index.html             Entrada de Vite
├── package.json           Scripts y dependencias del frontend (pnpm)
├── src/                   Frontend (React + TypeScript)
└── src-tauri/             Núcleo (Rust + Tauri 2)
```

## Núcleo Rust (`src-tauri/src`)

La dependencia va siempre hacia abajo: `commands → services → system`. Ninguna capa importa de una superior.

```
src-tauri/src/
├── main.rs                Arranque; solo llama a meelting_lib::run()
├── lib.rs                 Builder de Tauri y registro de comandos
├── commands/              Capa 1: comandos Tauri (delgados)
│   ├── keychain.rs        ✅ save_secret, secret_last_four, delete_secret
│   ├── worktree.rs        ✅ run_in_worktree
│   └── completion.rs      ✅ stream_completion
├── services/              Capa 2: acciones de negocio (verbos)
│   ├── worktree_run.rs    ✅ crea un worktree y ejecuta un comando en él
│   └── completion_stream.rs ✅ lee la clave del keychain y hace streaming de una respuesta
├── system/                Capa 3: integración con el SO, sin reglas de negocio
│   ├── git.rs             ✅ git por tokio::process (worktree add)
│   ├── terminal.rs        ✅ PTY portable y lectura de salida
│   ├── chat_completions.rs ✅ cliente HTTP con SSE (Chat Completions)
│   └── keychain.rs        ✅ keychain del SO (crate keyring)
├── repositories/          pendiente: acceso a SQLite (Fase 1)
├── domain/                pendiente: modelos y lógica pura, sin I/O
└── catalogs/              pendiente: agent_templates/*.yml y llm.yml embebidos
```

### Responsabilidad de cada capa

| Capa | Qué hace | Qué no hace |
|---|---|---|
| `commands` | Recibe argumentos de la UI, resuelve lo propio de Tauri (`AppHandle`, rutas de la app), llama a un servicio, emite eventos y devuelve el resultado | Reglas de negocio, SQL, procesos |
| `services` | Orquesta una acción completa y define su error serializable para la UI | Conocer Tauri (`AppHandle`, eventos): recibe *callbacks*, no emisores |
| `system` | Habla con el SO: procesos hijo, PTY, keychain. Cada módulo tiene su error propio | Decidir qué hacer ni serializar errores hacia la UI |
| `repositories` *(pendiente)* | Consultas SQLite y transiciones atómicas | Reglas entre tablas (van en servicios) |
| `domain` *(pendiente)* | Lógica pura testeable sin I/O (prompt, normalización, path jail) | Tocar disco, red o procesos |

### Convenciones del núcleo

- **Un error por módulo** con `thiserror`. Solo se implementa `Serialize` en el error que cruza hacia la UI (el del servicio o el de `keychain`), como cadena.
- **Comandos de secretos sin servicio:** `commands/keychain.rs` llama a `system/keychain.rs` directamente porque aún no hay regla que orquestar. Cuando exista la verificación previa a guardar (Fase 3), pasa a un servicio.
- **Servicios sin Tauri:** reciben `impl Fn` para salida y fin, de modo que se prueban sin ventana ni `AppHandle`.
- **Eventos hacia la UI:** los emite el comando. Nombres actuales: `terminal-output` (texto), `terminal-exit` (código de salida) y `llm-token` (fragmento de respuesta). El fin del streaming es el retorno del comando.
- **Claves:** solo el servicio las lee del keychain (`system::keychain::read`); nunca viajan a la UI. Los errores del proveedor no incluyen su texto, solo el estado HTTP.
- **Proveedores sin nombres en el código:** el cliente recibe `base_url`, `model` y, opcional, el nombre del secreto. Los adaptadores por proveedor llegan en la Fase 3.
- **Pruebas junto al código** (`#[cfg(test)] mod tests`), con repositorios git temporales reales (`tempfile`) y sin APIs externas.
- Nombres de archivo en `snake_case`; el servicio se nombra por la acción (`worktree_run`), no por el recurso.

## Frontend (`src`)

La UI llama al núcleo **solo** con `invoke` y `listen` de `@tauri-apps/api`, y únicamente desde hooks.

```
src/
├── main.tsx               Monta <App /> en #root
├── App.tsx                Composición de pantalla (hoy: pantalla provisional)
├── hooks/                 Acceso al núcleo y estado asociado
│   ├── useWorktreeRun.ts  ✅ invoke run_in_worktree + eventos de terminal
│   ├── useCompletionStream.ts ✅ invoke stream_completion + evento llm-token
│   └── useSecret.ts       ✅ guardar clave y mostrar sus últimos 4 caracteres
└── components/            Atomic design (overview.md §8)
    └── organisms/
        ├── WorktreeRunner.tsx  ✅ formulario y salida en vivo
        └── CompletionStreamer.tsx ✅ formulario y respuesta en streaming
```

| Carpeta | Responsabilidad |
|---|---|
| `components/atoms`, `molecules`, `organisms`, `templates` | Presentación. Sin `invoke`: reciben datos y callbacks por props o usan un hook. Se crea cada nivel cuando hay un componente que lo necesita |
| `hooks/` | Único lugar que llama a `invoke` o `listen`. Un hook por caso de uso, nombrado `use<Verbo><Objeto>` |
| `App.tsx` | Compone organismos o templates; sin lógica |

Pendientes con ubicación prevista: `src/i18n/` (`es.json`, `en.json`), estilos con Tailwind v4 y tokens Grafito, y tests con Vitest junto a cada componente.

## Dónde poner lo nuevo

| Quiero agregar... | Va en... |
|---|---|
| Un comando que invoca la UI | `commands/<recurso>.rs` + registro en `lib.rs` |
| Una acción de negocio | `services/<accion>.rs` |
| Un proceso hijo, PTY o API del SO | `system/<integracion>.rs` |
| Una consulta SQL | `repositories/<tabla>.rs` (requiere propuesta de migración aprobada) |
| Lógica pura sin I/O | `domain/<concepto>.rs` |
| Una pantalla o widget | `src/components/<nivel>/<Nombre>.tsx` |
| Llamada al núcleo desde la UI | `src/hooks/use<Verbo><Objeto>.ts` |
