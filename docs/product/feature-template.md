# Formato de funcionalidades y toma de decisiones

## Responsabilidades del Product Owner

1. Mantener decisiones alineadas con la visión.
2. Identificar problemas reales del usuario antes de proponer funcionalidades.
3. Definir historias de usuario y criterios de aceptación.
4. Priorizar el backlog por valor, esfuerzo, riesgo y dependencias.
5. Mantener controlado el alcance del MVP.
6. Diferenciar requisitos confirmados, propuestas e hipótesis.
7. Identificar dependencias entre funcionalidades.
8. Validar viabilidad con agentes técnicos.
9. Pedir aclaraciones ante decisiones importantes sin definir.
10. Evitar que la implementación técnica desplace los objetivos del producto.

## Plantilla de funcionalidad

```markdown
# <Nombre>

**Problema:** necesidad concreta del usuario.
**Objetivo:** resultado que buscamos.
**Historia de usuario:** Como [tipo de usuario], quiero [acción], para [beneficio].

## Flujo funcional
1. ...

## Criterios de aceptación
- [ ] Condición verificable.

## Reglas de negocio
- Restricciones y comportamientos del sistema.

## Dependencias
- Funcionalidades o componentes necesarios.

**Prioridad:** Must Have | Should Have | Could Have | Won't Have
**Métrica de éxito:** cómo validaremos su utilidad.
```

## Reglas de toma de decisiones

Antes de proponer una funcionalidad, evaluar:

- ¿Qué problema concreto resuelve?
- ¿Es necesaria para el MVP?
- ¿Qué valor aporta al usuario?
- ¿Puede implementarse de forma más sencilla?
- ¿Depende de capacidades todavía inexistentes?
- ¿Cómo sabremos si funciona?
- ¿Introduce riesgos de seguridad, permisos o privacidad?
- ¿Validamos una necesidad o agregamos complejidad?

Si falta una decisión de producto: presentar alternativas, ventajas, riesgos y una recomendación justificada. No asumir decisiones importantes sin validación del responsable del producto.
