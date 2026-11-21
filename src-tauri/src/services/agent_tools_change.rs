use crate::services::agent_run_engine::Engine;
use crate::services::session_load::SessionError;
use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
pub struct AgentTools {
    pub enabled: bool,
    pub available: bool,
}

const AVAILABLE: bool = cfg!(target_os = "macos");

pub fn current(engine: &Engine) -> AgentTools {
    AgentTools {
        enabled: engine.tools_enabled(),
        available: AVAILABLE,
    }
}

pub fn run(engine: &Engine, enabled: bool) -> Result<AgentTools, SessionError> {
    if enabled && !AVAILABLE {
        return Err(SessionError::AgentToolsUnsupportedPlatform);
    }
    engine.set_tools_enabled(enabled);
    Ok(current(engine))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::services::agent_run_engine::testing::{fast_limits, harness};

    #[tokio::test]
    async fn tools_start_off_and_cloud_consent_never_changes() {
        let h = harness("http://127.0.0.1:9", fast_limits(), &[]).await;

        assert!(!current(&h.engine).enabled);
        let _ = run(&h.engine, true);
        run(&h.engine, false).unwrap();

        assert!(!current(&h.engine).enabled);
        assert!(!h.engine.cloud_tools_consent());
    }

    #[cfg(target_os = "macos")]
    #[tokio::test]
    async fn on_macos_the_switch_turns_on_and_off() {
        let h = harness("http://127.0.0.1:9", fast_limits(), &[]).await;

        let on = run(&h.engine, true).unwrap();
        assert_eq!(
            on,
            AgentTools {
                enabled: true,
                available: true
            }
        );
        assert!(h.engine.tools_enabled());

        assert!(!run(&h.engine, false).unwrap().enabled);
        assert!(!h.engine.tools_enabled());
    }

    #[cfg(not(target_os = "macos"))]
    #[tokio::test]
    async fn outside_macos_the_switch_is_refused_and_stays_off() {
        let h = harness("http://127.0.0.1:9", fast_limits(), &[]).await;

        assert!(matches!(
            run(&h.engine, true),
            Err(SessionError::AgentToolsUnsupportedPlatform)
        ));
        assert!(!current(&h.engine).available);
        assert!(!h.engine.tools_enabled());
    }
}
