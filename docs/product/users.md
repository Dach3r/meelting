# Usuarios y experiencia principal

## Usuarios objetivo

**Público inicial:** desarrolladores de software, freelancers técnicos y equipos pequeños que usan varias herramientas digitales y quieren que la IA trabaje sobre su código sin enviarlo a un servicio de terceros.

**Público futuro:** startups, Product Managers, equipos de desarrollo, agencias digitales.

**Usuario principal del MVP:** desarrollador o fundador técnico que quiere delegar tareas de código a agentes de IA, mantener control de su máquina y supervisar resultados desde una única interfaz, incluso sin conexión.

## Experiencia principal

### 1. Primer arranque (planificado)

No hay registro, login ni cuenta. Al abrir la app por primera vez:

1. Elegir idioma (`es` / `en`).
2. Crear el primer workspace: nombre obligatorio, máximo 80 caracteres; los nombres pueden repetirse. El perfil y el workspace se guardan juntos al terminar.
3. La app abre **Equipos**: organigrama con "Tú" en la raíz y el equipo "General".

Criterio de completitud: el primer arranque termina cuando `profile.current_workspace_id` apunta a un workspace existente. Si el usuario cierra la app a mitad del flujo, no se guarda nada y lo retoma desde el principio al volver a abrirla.

Reglas:

- Un perfil local puede tener varios workspaces y cambiar entre ellos desde el riel.
- El botón se deshabilita mientras se procesa, para evitar duplicados.
- Quedan fuera: cuentas, invitaciones, miembros adicionales, sincronización e integraciones.

Debe ser rápido y evitar configuraciones innecesarias.

### 2. Ajustes

Una página con las categorías a la izquierda y los ajustes de la categoría elegida a la derecha, cada uno como una fila con su control. Hoy: **General** (idioma `es` / `en`) y **Apariencia** (tema). Cada ajuste nuevo es una fila más dentro de su categoría; una categoría nueva se suma a la lista lateral. Más adelante, **workspace** (renombrar; mismas reglas que al crearlo). Docker, shell y proveedor local se añaden con sus fases. Exportar y borrar los datos locales vive aquí (Fase 7). Eliminar un workspace es una decisión abierta (ver [../architecture/overview.md](../architecture/overview.md#13-decisiones-abiertas)).

### 3. Dashboard principal

Ver [virtual-office.md](virtual-office.md#dashboard).

### 4. Sala de trabajo con agentes

Experiencia central y diferencial. El usuario selecciona o inicia una sala e interactúa con agentes especializados. Ver [virtual-office.md](virtual-office.md).

### 5. Tarea sobre un repositorio (planificado, Fase 4–5)

Desde una sala, el usuario crea una tarea eligiendo agente, carpeta de repositorio y rama base. El agente trabaja en un `git worktree` aislado; el usuario aprueba los comandos que requieren permiso y revisa el diff al terminar.
