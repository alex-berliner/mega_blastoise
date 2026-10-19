/// Host mirror of `mega_blastoise_fw::subsystems::buzzer`.
///
/// Prints a brief log line instead of driving PWM — lets tests assert that
/// the right sound events fire without any audio hardware.
pub struct HostBuzzer {
    pub silent: bool,
}

impl HostBuzzer {
    pub fn new() -> Self {
        Self { silent: false }
    }

    pub fn silent() -> Self {
        Self { silent: true }
    }

    pub fn hit(&self) {
        self.log("[SFX] hit (880 Hz, 50 ms)");
    }

    pub fn super_effective(&self) {
        self.log("[SFX] super-effective (880 Hz + 1320 Hz)");
    }

    pub fn critical_hit(&self) {
        self.log("[SFX] critical hit (1760 Hz x2)");
    }

    pub fn faint(&self) {
        self.log("[SFX] faint descending (440→330→220 Hz)");
    }

    pub fn win(&self) {
        self.log("[SFX] win jingle (C5-E5-G5-C6)");
    }

    fn log(&self, msg: &str) {
        if !self.silent {
            println!("{msg}");
        }
    }
}

impl Default for HostBuzzer {
    fn default() -> Self {
        Self::new()
    }
}
