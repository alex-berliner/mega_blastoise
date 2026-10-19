/// Host mirror of `mega_blastoise_fw::hw_object::Driver`.
pub trait HostDriver<S> {
    fn apply(&mut self, state: &S);
}

impl<S> HostDriver<S> for () {
    fn apply(&mut self, _: &S) {}
}

/// Host mirror of `mega_blastoise_fw::hw_object::HwObject`.
/// Logs state changes via `println!` instead of `defmt::info!`.
pub struct HostHwObject<S, D = ()> {
    label: &'static str,
    state: S,
    driver: Option<D>,
}

impl<S: std::fmt::Display, D: HostDriver<S>> HostHwObject<S, D> {
    pub fn new(label: &'static str, initial: S, driver: Option<D>) -> Self {
        Self { label, state: initial, driver }
    }

    pub fn update(&mut self, new_state: S) {
        self.state = new_state;
        println!("[HW] {}: {}", self.label, &self.state);
        if let Some(d) = &mut self.driver {
            d.apply(&self.state);
        }
    }

    pub fn state(&self) -> &S {
        &self.state
    }
}
