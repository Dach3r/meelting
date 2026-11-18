export function roleRepeatsName(name: string, role: string) {
  return name.trim().toLocaleLowerCase() === role.trim().toLocaleLowerCase();
}

export function agentLabel({ name, role }: { name: string; role: string }) {
  return roleRepeatsName(name, role) ? name : `${name} · ${role}`;
}
