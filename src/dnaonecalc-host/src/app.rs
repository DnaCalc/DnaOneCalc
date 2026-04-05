use crate::state::OneCalcHostState;

#[derive(Debug, Default)]
pub struct OneCalcHostApp {
    state: OneCalcHostState,
}

impl OneCalcHostApp {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn state(&self) -> &OneCalcHostState {
        &self.state
    }

    pub fn launch_message(&self) -> &'static str {
        "DNA OneCalc host skeleton initialized"
    }
}
