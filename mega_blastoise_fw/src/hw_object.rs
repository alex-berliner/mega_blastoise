/// Drives a single piece of board hardware.
pub trait Driver<S> {
    fn apply(&mut self, state: &S);
}

/// No-op driver for mock/RTT-only operation.
impl<S> Driver<S> for () {
    fn apply(&mut self, _: &S) {}
}

/// Wraps a hardware object: always maintains state and logs over RTT;
/// optionally drives physical hardware if a driver is present.
pub struct HwObject<S, D = ()> {
    label: &'static str,
    state: S,
    driver: Option<D>,
}

impl<S: defmt::Format, D: Driver<S>> HwObject<S, D> {
    pub fn new(label: &'static str, initial: S, driver: Option<D>) -> Self {
        Self { label, state: initial, driver }
    }

    pub fn update(&mut self, new_state: S) {
        self.state = new_state;
        defmt::info!("{}: {}", self.label, &self.state);
        if let Some(d) = &mut self.driver {
            d.apply(&self.state);
        }
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}
