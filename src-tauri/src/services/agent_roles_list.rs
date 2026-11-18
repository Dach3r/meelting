use crate::catalogs::agent_roles::{self, RoleGroup};

pub fn run() -> Vec<RoleGroup> {
    agent_roles::groups()
}
