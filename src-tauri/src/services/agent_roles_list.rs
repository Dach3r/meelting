use crate::catalogs::agent_roles;

pub fn run() -> Vec<String> {
    agent_roles::all()
}
