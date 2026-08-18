//! Cursor navigation for the D-pad + A/B/? control scheme.
//!
//! The old scheme encoded the choice in *which* button you pressed: a corner
//! button meant the move drawn at that corner. The new scheme separates
//! pointing from committing, which is what buys an explicit back path and
//! stops a stray tap from locking in a move.
//!
//! This lives in core, not in a platform, for the same reason every other
//! input rule does: the web build and the firmware must agree exactly. It
//! translates navigation into the [`crate::PadEvent`]s the existing
//! [`crate::ChoiceCollector`] already understands, so the collector and its
//! test suite are untouched during the migration.

/// Which list the cursor is currently pointing into.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavMode {
    /// The move row along the bottom of the half.
    Moves,
    /// Party list.
    Party,
    /// A `?` detail view is open over one of the above.
    Detail,
}

/// What the platform should do as a result of a button press.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum NavOut {
    /// Nothing changed.
    None,
    /// Cursor moved; redraw, no game input.
    Redraw,
    /// Commit the move in this slot.
    TapMove(u8),
    /// Switch to this party index.
    TapSwitch(u8),
    /// Open the detail view for a move slot.
    HoldMove(u8),
    /// Open the stats view for a party index.
    HoldSwitch(u8),
    /// Close whatever detail view is open.
    HoldEnd,
}

/// Directional input.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum Dir {
    Up,
    Down,
    Left,
    Right,
}

/// One seat's cursor.
#[derive(Clone, Copy, Debug)]
pub struct CursorNav {
    pub cursor: u8,
    pub mode: NavMode,
    /// Number of selectable moves (0..=4).
    pub n_moves: u8,
    /// Number of party slots (0..=6).
    pub n_party: u8,
    /// A forced switch is in progress: the party list cannot be left.
    pub forced_switch: bool,
    /// The move slot this seat confirmed last, so the next turn opens on it.
    pub last_move: u8,
}

impl Default for CursorNav {
    fn default() -> Self {
        Self::new()
    }
}

impl CursorNav {
    pub const fn new() -> Self {
        Self {
            cursor: 0,
            mode: NavMode::Moves,
            n_moves: 4,
            n_party: 3,
            forced_switch: false,
            last_move: 0,
        }
    }

    /// Reset for a new turn. Keeps the cursor on a valid slot.
    pub fn begin_turn(&mut self, n_moves: u8, n_party: u8, forced_switch: bool) {
        self.n_moves = n_moves;
        self.n_party = n_party;
        self.forced_switch = forced_switch;
        self.mode = if forced_switch { NavMode::Party } else { NavMode::Moves };
        // Open on the move this seat used last. A battle is mostly the same
        // few moves in a row, and starting back at slot 0 every turn made the
        // player re-walk the grid to the move they had just picked.
        self.cursor = match self.mode {
            NavMode::Moves if self.last_move < n_moves => self.last_move,
            _ => 0,
        };
    }

    fn limit(&self) -> u8 {
        match self.mode {
            NavMode::Moves => self.n_moves,
            NavMode::Party => self.n_party,
            NavMode::Detail => 1,
        }
    }

    /// D-pad. The move menu is a 2x2 grid like the games', so left/right step
    /// within a row and up/down jump a row; the party list is a column and
    /// wraps.
    pub fn dpad(&mut self, dir: Dir) -> NavOut {
        let limit = self.limit();
        if limit == 0 {
            return NavOut::None;
        }
        let before = self.cursor;
        match self.mode {
            NavMode::Moves => {
                let (mut col, mut row) = (self.cursor % 2, self.cursor / 2);
                match dir {
                    Dir::Left => col = col.wrapping_sub(1) & 1,
                    Dir::Right => col = (col + 1) & 1,
                    Dir::Up => row = row.wrapping_sub(1) & 1,
                    Dir::Down => row = (row + 1) & 1,
                }
                // Skip past empty grid cells when a mon has fewer than 4 moves.
                let want = row * 2 + col;
                self.cursor = if want < limit { want } else { self.cursor };
            }
            NavMode::Party => match dir {
                Dir::Up | Dir::Left => {
                    self.cursor = if self.cursor == 0 { limit - 1 } else { self.cursor - 1 }
                }
                Dir::Down | Dir::Right => self.cursor = (self.cursor + 1) % limit,
            },
            // Paging through a description is the same gesture.
            NavMode::Detail => return NavOut::Redraw,
        }
        if self.cursor == before {
            NavOut::None
        } else {
            NavOut::Redraw
        }
    }

    /// A — confirm.
    pub fn confirm(&mut self) -> NavOut {
        match self.mode {
            NavMode::Moves => {
                self.last_move = self.cursor;
                NavOut::TapMove(self.cursor)
            }
            NavMode::Party => NavOut::TapSwitch(self.cursor),
            NavMode::Detail => {
                self.mode = NavMode::Moves;
                NavOut::HoldEnd
            }
        }
    }

    /// B — back out. From the move row this opens the party list, since
    /// there is nothing above it to return to during a turn.
    pub fn back(&mut self) -> NavOut {
        match self.mode {
            NavMode::Detail => {
                self.mode = if self.forced_switch { NavMode::Party } else { NavMode::Moves };
                self.cursor = 0;
                NavOut::HoldEnd
            }
            NavMode::Party if !self.forced_switch => {
                self.mode = NavMode::Moves;
                self.cursor = 0;
                NavOut::Redraw
            }
            NavMode::Party => NavOut::None,
            NavMode::Moves => {
                self.mode = NavMode::Party;
                self.cursor = 0;
                NavOut::Redraw
            }
        }
    }

    /// `?` — explain whatever the cursor is on.
    pub fn info(&mut self) -> NavOut {
        match self.mode {
            NavMode::Moves => {
                let slot = self.cursor;
                self.mode = NavMode::Detail;
                NavOut::HoldMove(slot)
            }
            NavMode::Party => {
                let idx = self.cursor;
                self.mode = NavMode::Detail;
                NavOut::HoldSwitch(idx)
            }
            NavMode::Detail => {
                self.mode = if self.forced_switch { NavMode::Party } else { NavMode::Moves };
                NavOut::HoldEnd
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn move_grid_wraps_within_two_by_two() {
        let mut n = CursorNav::new();
        n.begin_turn(4, 3, false);
        assert_eq!(n.cursor, 0);
        n.dpad(Dir::Right);
        assert_eq!(n.cursor, 1);
        n.dpad(Dir::Down);
        assert_eq!(n.cursor, 3, "down jumps a row of the 2x2 menu");
        n.dpad(Dir::Left);
        assert_eq!(n.cursor, 2);
        n.dpad(Dir::Up);
        assert_eq!(n.cursor, 0);
    }

    #[test]
    fn cursor_never_lands_on_a_missing_move() {
        let mut n = CursorNav::new();
        n.begin_turn(2, 3, false);
        n.dpad(Dir::Down);
        assert!(n.cursor < 2, "cursor {} escaped a 2-move grid", n.cursor);
    }

    #[test]
    fn confirm_commits_the_pointed_slot() {
        let mut n = CursorNav::new();
        n.begin_turn(4, 3, false);
        n.dpad(Dir::Right);
        assert_eq!(n.confirm(), NavOut::TapMove(1));
    }

    #[test]
    fn forced_switch_cannot_be_backed_out_of() {
        let mut n = CursorNav::new();
        n.begin_turn(4, 3, true);
        assert_eq!(n.mode, NavMode::Party);
        assert_eq!(n.back(), NavOut::None);
        assert_eq!(n.mode, NavMode::Party);
    }

    #[test]
    fn info_opens_and_closes_over_either_list() {
        let mut n = CursorNav::new();
        n.begin_turn(4, 3, false);
        assert_eq!(n.info(), NavOut::HoldMove(0));
        assert_eq!(n.mode, NavMode::Detail);
        assert_eq!(n.info(), NavOut::HoldEnd);
        assert_eq!(n.mode, NavMode::Moves);
    }

    /// A battle is mostly the same few moves in a row, so the menu opens on
    /// the one this seat used last instead of walking back to slot 0.
    #[test]
    fn the_menu_reopens_on_the_move_that_was_used() {
        let mut n = CursorNav::new();
        n.begin_turn(4, 3, false);
        n.dpad(Dir::Right);
        n.dpad(Dir::Down);
        assert_eq!(n.confirm(), NavOut::TapMove(3));
        n.begin_turn(4, 3, false);
        assert_eq!(n.cursor, 3);
        // A mon with fewer moves cannot inherit a slot it does not have.
        n.begin_turn(2, 3, false);
        assert_eq!(n.cursor, 0);
        // A forced switch opens on the party list, from the top.
        n.begin_turn(4, 3, true);
        assert_eq!(n.cursor, 0);
        assert_eq!(n.mode, NavMode::Party);
    }
}
