//! Leptos-, Tauri-, and CLI-free OneCalc session boundary.

pub use dnacalc_skin_ir::{
    RuntimeProfileProjection, SkinIntent, SkinIntentDiagnostic, SkinIntentEnvelope,
    SkinIntentReceipt, SkinSnapshot,
};

pub trait OneCalcSessionHost {
    fn snapshot(&self) -> SkinSnapshot;
    fn dispatch(&mut self, intent: SkinIntent) -> bool;
}

pub struct OneCalcSession<H> {
    host: H,
}

impl<H: OneCalcSessionHost> OneCalcSession<H> {
    pub fn new(host: H) -> Self {
        Self { host }
    }

    pub fn snapshot(&self) -> SkinSnapshot {
        self.host.snapshot()
    }

    pub fn handle(&mut self, envelope: SkinIntentEnvelope) -> SkinIntentReceipt {
        if let Err(error) = envelope.validate() {
            return SkinIntentReceipt::Rejected {
                intent_id: envelope.intent_id,
                diagnostic: SkinIntentDiagnostic {
                    code: "invalid_intent_envelope".into(),
                    message: format!("{error:?}"),
                    recoverable: false,
                },
            };
        }
        let intent_id = envelope.intent_id;
        if self.host.dispatch(envelope.intent) {
            SkinIntentReceipt::Applied {
                intent_id,
                snapshot_revision: 0,
            }
        } else {
            SkinIntentReceipt::Rejected {
                intent_id,
                diagnostic: SkinIntentDiagnostic {
                    code: "no_change".into(),
                    message: "intent produced no state change".into(),
                    recoverable: true,
                },
            }
        }
    }

    pub fn handle_json(&mut self, json: &str) -> Result<String, serde_json::Error> {
        let envelope = serde_json::from_str(json)?;
        serde_json::to_string(&self.handle(envelope))
    }
}

#[must_use]
pub const fn runtime_profile() -> RuntimeProfileProjection {
    if cfg!(target_arch = "wasm32") {
        RuntimeProfileProjection::BrowserWasm
    } else if cfg!(target_os = "windows") {
        RuntimeProfileProjection::WindowsDesktop
    } else {
        RuntimeProfileProjection::NativeUnix
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RuntimeTarget {
    BrowserWasm,
    HostedWeb,
    WindowsDesktop,
    WindowsHeadless,
    NativeUnix,
    NullTest,
}

impl RuntimeTarget {
    #[must_use]
    pub const fn profile(self) -> RuntimeProfileProjection {
        match self {
            Self::BrowserWasm => RuntimeProfileProjection::BrowserWasm,
            Self::HostedWeb => RuntimeProfileProjection::HostedWeb,
            Self::WindowsDesktop => RuntimeProfileProjection::WindowsDesktop,
            Self::WindowsHeadless => RuntimeProfileProjection::WindowsHeadless,
            Self::NativeUnix => RuntimeProfileProjection::NativeUnix,
            Self::NullTest => RuntimeProfileProjection::NullTest,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_shared_runtime_profile_has_a_onecalc_target() {
        assert_eq!(
            RuntimeTarget::BrowserWasm.profile(),
            RuntimeProfileProjection::BrowserWasm
        );
        assert_eq!(
            RuntimeTarget::HostedWeb.profile(),
            RuntimeProfileProjection::HostedWeb
        );
        assert_eq!(
            RuntimeTarget::WindowsDesktop.profile(),
            RuntimeProfileProjection::WindowsDesktop
        );
        assert_eq!(
            RuntimeTarget::WindowsHeadless.profile(),
            RuntimeProfileProjection::WindowsHeadless
        );
        assert_eq!(
            RuntimeTarget::NativeUnix.profile(),
            RuntimeProfileProjection::NativeUnix
        );
        assert_eq!(
            RuntimeTarget::NullTest.profile(),
            RuntimeProfileProjection::NullTest
        );
    }
}
