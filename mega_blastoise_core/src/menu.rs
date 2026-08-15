//! Pre-lobby menus: the generation picker and the options screen.
//!
//! Both are shown before a battle starts, drawn head-to-head like every other
//! screen so either seat can read and change them, per
//! `architecture/09-single-screen.md`. Keeping the state machine in core means
//! the browser and the firmware present identical menus and identical
//! defaults, the same rule the battle screens already follow.

use crate::cursor_nav::Dir;

/// Which generation's rules the next battle uses.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Gen {
    One,
    /// Selectable, but routes to Gen 1 combat until the engine lands — the UI
    /// must label it a preview and must not advertise Gen 3-only fields.
    ThreePreview,
}

impl Gen {
    pub fn as_str(self) -> &'static str {
        match self {
            Gen::One => "Gen 1",
            Gen::ThreePreview => "Gen 3 (preview)",
        }
    }
}

/// Text pacing. Multiplies the per-event animation delay.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum TextSpeed {
    Slow,
    Normal,
    Fast,
}

impl TextSpeed {
    pub fn as_str(self) -> &'static str {
        match self {
            TextSpeed::Slow => "Slow",
            TextSpeed::Normal => "Normal",
            TextSpeed::Fast => "Fast",
        }
    }

    /// Scale a base delay in milliseconds.
    pub fn scale(self, ms: u32) -> u32 {
        match self {
            TextSpeed::Slow => ms * 3 / 2,
            TextSpeed::Normal => ms,
            TextSpeed::Fast => ms / 2,
        }
    }

    fn next(self) -> Self {
        match self {
            TextSpeed::Slow => TextSpeed::Normal,
            TextSpeed::Normal => TextSpeed::Fast,
            TextSpeed::Fast => TextSpeed::Slow,
        }
    }
}

/// Everything the options screen controls. Applies to both seats: either
/// player may change a setting and it takes effect for the match.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub struct GameOptions {
    pub gen: Gen,
    /// 3 or 6.
    pub team_size: u8,
    pub text_speed: TextSpeed,
    pub sound: bool,
    /// Show the tutorial before the first battle of this power-on only.
    pub tutorial: bool,
    /// Seconds, or 0 for off.
    pub turn_timer: u16,
}

impl Default for GameOptions {
    fn default() -> Self {
        Self {
            gen: Gen::One,
            team_size: 3,
            text_speed: TextSpeed::Normal,
            sound: true,
            tutorial: false,
            turn_timer: 60,
        }
    }
}

/// Rows on the options screen, in display order.
pub const OPTION_ROWS: usize = 5;

impl GameOptions {
    /// Label and current value for row `i`, for the renderer.
    pub fn row(&self, i: usize) -> (&'static str, &'static str) {
        match i {
            0 => ("Team size", if self.team_size == 6 { "6 v 6" } else { "3 v 3" }),
            1 => ("Text speed", self.text_speed.as_str()),
            2 => ("Sound", if self.sound { "On" } else { "Off" }),
            3 => ("Tutorial", if self.tutorial { "First game" } else { "Off" }),
            _ => ("Turn timer", if self.turn_timer == 0 { "Off" } else { "60 s" }),
        }
    }

    fn cycle(&mut self, i: usize) {
        match i {
            0 => self.team_size = if self.team_size == 3 { 6 } else { 3 },
            1 => self.text_speed = self.text_speed.next(),
            2 => self.sound = !self.sound,
            3 => self.tutorial = !self.tutorial,
            _ => self.turn_timer = if self.turn_timer == 0 { 60 } else { 0 },
        }
    }
}

/// Which pre-battle screen is showing.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuScreen {
    GenPicker,
    Lobby,
    Options,
}

/// What the caller should do after handling an input.
///
/// [`MenuOut::EnterLobby`] is the only way out of the menus, and every path
/// that lands on [`MenuScreen::Lobby`] must return it: the menu layer has no
/// lobby screen of its own to draw, so a caller that keeps the menu open on
/// that state strands the player on a screen no button can leave.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum MenuOut {
    None,
    Redraw,
    /// Leave the menus and run the lobby ready-up sequence.
    EnterLobby,
}

/// The pre-lobby menu state machine.
///
/// This is cursor state only. The settings it edits live outside it and are
/// passed in, because a seat's cursor is private — one player opening the
/// options must not drag the other into them — while the settings themselves
/// are shared: either player may change one and it applies to the match.
#[derive(Clone, Copy, Debug)]
pub struct Menu {
    pub screen: MenuScreen,
    pub cursor: u8,
}

impl Default for Menu {
    fn default() -> Self {
        Self::new()
    }
}

impl Menu {
    pub fn new() -> Self {
        Self { screen: MenuScreen::GenPicker, cursor: 0 }
    }

    fn limit(&self) -> u8 {
        match self.screen {
            MenuScreen::GenPicker => 2,
            MenuScreen::Options => OPTION_ROWS as u8,
            MenuScreen::Lobby => 1,
        }
    }

    pub fn dpad(&mut self, dir: Dir, opts: &mut GameOptions) -> MenuOut {
        let limit = self.limit();
        if limit <= 1 {
            return MenuOut::None;
        }
        match self.screen {
            // Left/right cycles a setting in place; up/down moves between rows.
            MenuScreen::Options => match dir {
                Dir::Up => {
                    self.cursor = if self.cursor == 0 { limit - 1 } else { self.cursor - 1 };
                    MenuOut::Redraw
                }
                Dir::Down => {
                    self.cursor = (self.cursor + 1) % limit;
                    MenuOut::Redraw
                }
                _ => {
                    opts.cycle(self.cursor as usize);
                    MenuOut::Redraw
                }
            },
            _ => match dir {
                Dir::Up | Dir::Left => {
                    self.cursor = if self.cursor == 0 { limit - 1 } else { self.cursor - 1 };
                    MenuOut::Redraw
                }
                Dir::Down | Dir::Right => {
                    self.cursor = (self.cursor + 1) % limit;
                    MenuOut::Redraw
                }
            },
        }
    }

    pub fn confirm(&mut self, opts: &mut GameOptions) -> MenuOut {
        match self.screen {
            MenuScreen::GenPicker => {
                opts.gen = if self.cursor == 0 { Gen::One } else { Gen::ThreePreview };
                self.screen = MenuScreen::Lobby;
                self.cursor = 0;
                MenuOut::EnterLobby
            }
            MenuScreen::Options => {
                opts.cycle(self.cursor as usize);
                MenuOut::Redraw
            }
            MenuScreen::Lobby => MenuOut::None,
        }
    }

    pub fn back(&mut self) -> MenuOut {
        match self.screen {
            MenuScreen::Options => {
                self.screen = MenuScreen::Lobby;
                self.cursor = 0;
                MenuOut::EnterLobby
            }
            MenuScreen::Lobby => {
                self.screen = MenuScreen::GenPicker;
                self.cursor = 0;
                MenuOut::Redraw
            }
            MenuScreen::GenPicker => MenuOut::None,
        }
    }

    /// `?` opens the options from the lobby, and closes them again.
    pub fn info(&mut self) -> MenuOut {
        match self.screen {
            MenuScreen::Lobby => {
                self.screen = MenuScreen::Options;
                self.cursor = 0;
                MenuOut::Redraw
            }
            MenuScreen::Options => {
                self.screen = MenuScreen::Lobby;
                self.cursor = 0;
                MenuOut::EnterLobby
            }
            MenuScreen::GenPicker => MenuOut::None,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn gen_picker_leads_into_the_lobby() {
        let mut m = Menu::new();
        let mut o = GameOptions::default();
        assert_eq!(m.screen, MenuScreen::GenPicker);
        assert_eq!(m.confirm(&mut o), MenuOut::EnterLobby);
        assert_eq!(m.screen, MenuScreen::Lobby);
        assert_eq!(o.gen, Gen::One);
    }

    #[test]
    fn picking_gen_three_records_the_preview_choice() {
        let mut m = Menu::new();
        let mut o = GameOptions::default();
        m.dpad(Dir::Down, &mut o);
        m.confirm(&mut o);
        assert_eq!(o.gen, Gen::ThreePreview);
    }

    /// Two seats each have their own cursor, and neither is dragged into the
    /// other's menu, but a setting either one changes applies to both.
    #[test]
    fn seats_have_private_cursors_over_shared_settings() {
        let mut o = GameOptions::default();
        let mut p1 = Menu { screen: MenuScreen::Options, cursor: 0 };
        let p2 = Menu { screen: MenuScreen::Lobby, cursor: 0 };
        p1.dpad(Dir::Down, &mut o);
        assert_eq!(p1.cursor, 1);
        assert_eq!(p2.cursor, 0, "one seat moving its cursor must not move the other's");
        assert_eq!(p2.screen, MenuScreen::Lobby, "and must not open the other's menu");
        p1.dpad(Dir::Right, &mut o);
        assert_eq!(o.text_speed, TextSpeed::Fast, "settings are shared, not per seat");
    }

    #[test]
    fn options_open_from_the_lobby_and_close_again() {
        let mut m = Menu::new();
        let mut o = GameOptions::default();
        m.confirm(&mut o);
        assert_eq!(m.info(), MenuOut::Redraw);
        assert_eq!(m.screen, MenuScreen::Options);
        assert_eq!(m.info(), MenuOut::EnterLobby, "closing must hand the screen back");
        assert_eq!(m.screen, MenuScreen::Lobby);
    }

    #[test]
    fn b_out_of_the_options_hands_the_screen_back_to_the_lobby() {
        let mut m = Menu::new();
        let mut o = GameOptions::default();
        m.confirm(&mut o);
        m.info();
        assert_eq!(m.screen, MenuScreen::Options);
        // Returning Redraw here would leave the menu layer owning the panel on
        // a screen it cannot draw and no button can leave.
        assert_eq!(m.back(), MenuOut::EnterLobby);
        assert_eq!(m.screen, MenuScreen::Lobby);
    }

    #[test]
    fn left_right_cycles_a_setting_without_moving_the_cursor() {
        let mut m = Menu::new();
        let mut o = GameOptions::default();
        m.confirm(&mut o);
        m.info();
        assert_eq!(o.team_size, 3);
        m.dpad(Dir::Right, &mut o);
        assert_eq!(o.team_size, 6);
        assert_eq!(m.cursor, 0, "cycling a value must not move the cursor");
    }

    #[test]
    fn turn_timer_toggles_between_off_and_sixty() {
        let mut m = Menu::new();
        let mut o = GameOptions::default();
        m.confirm(&mut o);
        m.info();
        m.cursor = 4;
        assert_eq!(o.turn_timer, 60);
        m.confirm(&mut o);
        assert_eq!(o.turn_timer, 0);
    }

    #[test]
    fn text_speed_scales_delays() {
        assert_eq!(TextSpeed::Normal.scale(2500), 2500);
        assert_eq!(TextSpeed::Fast.scale(2500), 1250);
        assert_eq!(TextSpeed::Slow.scale(2500), 3750);
    }
}
