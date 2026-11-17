# Usuarios y experiencia principal

## Usuarios objetivo

**Público inicial:** desarrolladores de software, freelancers técnicos y equipos pequeños que usan varias herramientas digitales y quieren que la IA trabaje sobre su código sin enviarlo a un servicio de terceros.

**Público futuro:** startups, Product Managers, equipos de desarrollo, agencias digitales.

**Usuario principal del MVP:** desarrollador o fundador técnico que quiere delegar tareas de código a agentes de IA, mantener control de su máquina y supervisar resultados desde una única interfaz, incluso sin conexión.

## Experiencia principal

### 1. Primer arranque (planificado)

No hay registro, login ni cuenta. Al abrir la app por primera vez:

1. Elegir idioma (`es` / `en`) y, opcionalmente, nombre y apellido para el perfil local.
2. Crear el primer workspace: nombre obligatorio, máximo 80 caracteres; los nombres pueden repetirse.
3. La app abre **Equipos**: organigrama con "Tú" en la raíz y el equipo "General".

Criterio de completitud: el primer arranque termina cuando `profile.current_workspace_id` apunta a un workspace existente. Si el usuario cierra la app a mitad del flujo, lo retoma al volver a abrirla.

Reglas:

- Un perfil local puede tener varios workspaces y cambiar entre ellos desde el riel.
- El botón se deshabilita mientras se procesa, para evitar duplicados.
- Quedan fuera: cuentas, invitaciones, miembros adicionales, sincronización e integraciones.

Debe ser rápido y evitar configuraciones innecesarias.

### 2. Dashboard principal

Ver [virtual-office.md](virtual-office.md#dashboard).

### 3. Sala de trabajo con agentes

Experiencia central y diferencial. El usuario selecciona o inicia una sala e interactúa con agentes especializados. Ver [virtual-office.md](virtual-office.md).

### 4. Tarea sobre un repositorio (planificado, Fase 4–5)

Desde una sala, el usuario crea una tarea eligiendo agente, carpeta de repositorio y rama base. El agente trabaja en un `git worktree` aislado; el usuario aprueba los comandos que requieren permiso y revisa el diff al terminar.
