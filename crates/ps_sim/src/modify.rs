//! Fixed-point modifiers in 4096ths, as PS computes them (design 5.3).
//!
//! PS keeps the running modifier of an event in `this.event.modifier`, a
//! JS number that is always a multiple of 1/4096; here it is the integer
//! count of 4096ths ([`crate::state::EventFrame::modifier`]). Every value
//! PS passes through `this.trunc` goes through [`Trunc`], because the
//! truncation is per format: stock PS's `Dex.trunc` wraps to an unsigned
//! 32-bit integer (and to `bits` bits when asked), while the customgame
//! formats swap in `Math.trunc`, which ignores `bits` (design section 3,
//! `Cartridge Trunc`).
//!
//! The arithmetic is exact integer arithmetic. PS does it in doubles, but
//! every quantity is an integer or an integer ratio well inside 2^53, and
//! `x * 4096` is exact in binary floating point, so the results agree.
//!
//! PS: sim/battle.ts `chain`, `chainModify`, `modify`, `finalModify`;
//! sim/dex.ts `trunc`; config/formats.ts `battle: { trunc: Math.trunc }`.

use crate::state::Battle;

/// Which `trunc` the format uses.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trunc {
    /// PS's `Dex.trunc`: `num >>> 0`, then `% 2 ** bits` when `bits` is set.
    Cartridge,
    /// `Math.trunc` (customgame formats): toward zero, `bits` ignored.
    Math,
}

const U32_RANGE: i64 = 1 << 32;

impl Trunc {
    /// `trunc(num, bits)` of an integer.
    pub fn int(self, num: i64, bits: u32) -> i64 {
        match self {
            Trunc::Cartridge => {
                let n = num.rem_euclid(U32_RANGE);
                if bits != 0 {
                    n % (1i64 << bits)
                } else {
                    n
                }
            }
            Trunc::Math => num,
        }
    }

    /// `trunc(n / d, bits)` of an exact ratio (`d != 0`). Rust's integer
    /// division truncates toward zero, as both truncations do before
    /// `>>> 0` wraps.
    pub fn ratio(self, n: i64, d: i64, bits: u32) -> i64 {
        self.int(n / d, bits)
    }

    /// `trunc(x)` of a double, for PS's decimal literals (`chainModify(1.5)`).
    pub fn float(self, x: f64) -> i64 {
        // `as` truncates toward zero (NaN to 0, as `>>> 0` does).
        self.int(x as i64, 0)
    }
}

/// PS `modify(value, numerator, denominator)`:
/// `tr((tr(value * tr(numerator * 4096 / denominator)) + 2048 - 1) / 4096)`.
pub fn modify(tr: Trunc, value: i64, numerator: i64, denominator: i64) -> i64 {
    let modifier = tr.ratio(numerator * 4096, denominator, 0);
    modify_4096(tr, value, modifier)
}

/// PS `modify(value, m)` where `m` is already a multiple of 1/4096 (the
/// event modifier): `tr(m * 4096)` is `m4096` itself.
pub fn modify_4096(tr: Trunc, value: i64, modifier_4096: i64) -> i64 {
    let modifier = tr.int(modifier_4096, 0);
    tr.ratio(tr.int(value * modifier, 0) + 2048 - 1, 4096, 0)
}

/// PS `modify(value, x)` with a decimal multiplier (`modify(atk, 1.5)`).
pub fn modify_f(tr: Trunc, value: i64, x: f64) -> i64 {
    modify_4096(tr, value, tr.float(x * 4096.0))
}

/// The JS `(a * b + 2048) >> 12`: the product is formed exactly, then
/// `>>` takes it to a signed 32-bit integer before shifting.
fn mul_shift(a: i64, b: i64) -> i64 {
    let wrapped = (a * b + 2048) as i32;
    i64::from(wrapped >> 12)
}

/// PS `chain(previousMod, nextMod)` over two `[numerator, denominator]`
/// modifiers; returns the chained modifier in 4096ths.
pub fn chain(tr: Trunc, prev: (i64, i64), next: (i64, i64)) -> i64 {
    let p = tr.ratio(prev.0 * 4096, prev.1, 0);
    let n = tr.ratio(next.0 * 4096, next.1, 0);
    mul_shift(p, n)
}

impl Battle {
    /// PS `this.trunc(num, bits)` for this battle's format.
    pub fn trunc(&self, num: i64, bits: u32) -> i64 {
        self.trunc.int(num, bits)
    }

    /// PS `chainModify(numerator, denominator)`: fold a modifier into the
    /// running event's modifier.
    ///
    /// `singleEvent` frames have no modifier (`undefined` in PS); then
    /// `trunc(undefined * 4096)` is 0 or NaN, and either way
    /// `(x * next + 2048) >> 12` is 0, which is what this gives.
    // PS: sim/battle.ts chainModify
    pub fn chain_modify(&mut self, numerator: i64, denominator: i64) {
        let next = self.trunc.ratio(numerator * 4096, denominator, 0);
        self.chain_modify_4096(next);
    }

    /// PS `chainModify(x)` with a decimal (`chainModify(1.5)`).
    pub fn chain_modify_f(&mut self, x: f64) {
        let next = self.trunc.float(x * 4096.0);
        self.chain_modify_4096(next);
    }

    fn chain_modify_4096(&mut self, next: i64) {
        let prev = match self.event.modifier {
            Some(m) => self.trunc.int(i64::from(m), 0),
            None => 0,
        };
        self.event.modifier = Some(mul_shift(prev, next) as i32);
    }

    /// PS `modify(value, numerator, denominator)` with this format's trunc.
    pub fn modify(&self, value: i64, numerator: i64, denominator: i64) -> i64 {
        modify(self.trunc, value, numerator, denominator)
    }

    /// PS `modify(value, x)` with a decimal multiplier.
    pub fn modify_f(&self, value: i64, x: f64) -> i64 {
        modify_f(self.trunc, value, x)
    }

    /// PS `finalModify(relayVar)`: apply the event's modifier and reset it.
    // PS: sim/battle.ts finalModify
    pub fn final_modify(&mut self, value: i64) -> i64 {
        let m = i64::from(self.event.modifier.unwrap_or(0));
        let out = modify_4096(self.trunc, value, m);
        self.event.modifier = Some(4096);
        out
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn cartridge_trunc_wraps() {
        let t = Trunc::Cartridge;
        assert_eq!(t.int(-1, 0), 4_294_967_295);
        assert_eq!(t.int(1 << 32, 0), 0);
        assert_eq!(t.int(1100, 8), 1100 % 256);
        assert_eq!(t.ratio(-3, 2, 0), 4_294_967_295); // -1.5 >>> 0
        assert_eq!(Trunc::Math.int(1100, 8), 1100);
        assert_eq!(Trunc::Math.ratio(-3, 2, 0), -1);
    }

    #[test]
    fn modify_matches_ps() {
        // modify(100, 1.5): modifier 6144, (614400 + 2047) / 4096 = 150.49..
        assert_eq!(modify(Trunc::Math, 100, 3, 2), 150);
        assert_eq!(modify_f(Trunc::Math, 100, 1.5), 150);
        // modify(57, 0.5) = tr((57 * 2048 + 2047) / 4096) = 28 (rounds half down)
        assert_eq!(modify(Trunc::Math, 57, 1, 2), 28);
        // 1.2 * 4096 = 4915.2 -> 4915 both ways.
        assert_eq!(modify_f(Trunc::Math, 1000, 1.2), modify(Trunc::Math, 1000, 6, 5));
        // chain(1.5, 1.5) = (6144 * 6144 + 2048) >> 12 = 9216
        assert_eq!(chain(Trunc::Math, (3, 2), (3, 2)), 9216);
        // chain([5325, 4096], 0.5)
        assert_eq!(chain(Trunc::Math, (5325, 4096), (1, 2)), (5325 * 2048 + 2048) >> 12);
    }

    #[test]
    fn mul_shift_wraps_like_js() {
        // (2^20 * 2^12 + 2048) >> 12 in JS: ToInt32(2^32 + 2048) = 2048 -> 0.
        assert_eq!(mul_shift(1 << 20, 1 << 12), 0);
    }
}
