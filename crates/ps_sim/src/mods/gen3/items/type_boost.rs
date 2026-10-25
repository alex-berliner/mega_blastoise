//! The gen 3 type-boost items: Black Belt, Black Glasses, Charcoal,
//! Dragon Fang, Hard Stone, Magnet, Metal Coat, Miracle Seed, Mystic
//! Water, Never-Melt Ice, Poison Barb, Sea Incense, Sharp Beak, Silk
//! Scarf, Silver Powder, Soft Sand, Spell Tag, Twisted Spoon.
//!
//! Gen 3 boosts the attacking stat, not the base power: each item has one
//! `onModifyAtk` (a physical type) or `onModifySpA` (a special type),
//! priority 1, `if (move?.type === TYPE) return this.chainModify(1.1)`
//! (Sea Incense: 1.05). Their `onBasePowerPriority: 15` has no
//! `onBasePower` left to order.
//!
//! PS: data/mods/gen3/items.ts (each item's `onModifyAtk` /
//! `onModifySpA`)

use ps_dex::Type;

use crate::effect::{EffectDef, EventArgs, EventId, Handler};
use crate::mods::gen3::conditions::event_move_type;
use crate::relay::Relay;
use crate::state::Battle;

/// `if (move?.type === type) return this.chainModify(factor)`: the move
/// in flight's type (Hidden Power's is its own); a typeless move object
/// (the confusion hit) boosts nothing.
fn type_boost(b: &mut Battle, a: &EventArgs, type_: Type, factor: f64) -> Option<Relay> {
    if event_move_type(b, a) == type_ {
        b.chain_modify_f(factor);
    }
    None
}

macro_rules! type_boost_item {
    ($def:ident, $handler:ident, $event:ident, $type_:ident, $factor:expr, $ps:literal) => {
        #[doc = concat!("`", stringify!($event), "`: ", stringify!($type_), " moves, `chainModify(", stringify!($factor), ")`.")]
        #[doc = concat!("PS: data/mods/gen3/items.ts ", $ps)]
        fn $handler(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
            type_boost(b, a, Type::$type_, $factor)
        }

        pub static $def: EffectDef = EffectDef::new(&[Handler::on(EventId::$event, $handler).priority(1)]);
    };
}

type_boost_item!(BLACKBELT, blackbelt, ModifyAtk, Fighting, 1.1, "blackbelt.onModifyAtk");
type_boost_item!(BLACKGLASSES, blackglasses, ModifySpA, Dark, 1.1, "blackglasses.onModifySpA");
type_boost_item!(CHARCOAL, charcoal, ModifySpA, Fire, 1.1, "charcoal.onModifySpA");
type_boost_item!(DRAGONFANG, dragonfang, ModifySpA, Dragon, 1.1, "dragonfang.onModifySpA");
type_boost_item!(HARDSTONE, hardstone, ModifyAtk, Rock, 1.1, "hardstone.onModifyAtk");
type_boost_item!(MAGNET, magnet, ModifySpA, Electric, 1.1, "magnet.onModifySpA");
type_boost_item!(METALCOAT, metalcoat, ModifyAtk, Steel, 1.1, "metalcoat.onModifyAtk");
type_boost_item!(MIRACLESEED, miracleseed, ModifySpA, Grass, 1.1, "miracleseed.onModifySpA");
type_boost_item!(MYSTICWATER, mysticwater, ModifySpA, Water, 1.1, "mysticwater.onModifySpA");
type_boost_item!(NEVERMELTICE, nevermeltice, ModifySpA, Ice, 1.1, "nevermeltice.onModifySpA");
type_boost_item!(POISONBARB, poisonbarb, ModifyAtk, Poison, 1.1, "poisonbarb.onModifyAtk");
type_boost_item!(SEAINCENSE, seaincense, ModifySpA, Water, 1.05, "seaincense.onModifySpA");
type_boost_item!(SHARPBEAK, sharpbeak, ModifyAtk, Flying, 1.1, "sharpbeak.onModifyAtk");
type_boost_item!(SILKSCARF, silkscarf, ModifyAtk, Normal, 1.1, "silkscarf.onModifyAtk");
type_boost_item!(SILVERPOWDER, silverpowder, ModifyAtk, Bug, 1.1, "silverpowder.onModifyAtk");
type_boost_item!(SOFTSAND, softsand, ModifyAtk, Ground, 1.1, "softsand.onModifyAtk");
type_boost_item!(SPELLTAG, spelltag, ModifyAtk, Ghost, 1.1, "spelltag.onModifyAtk");
type_boost_item!(TWISTEDSPOON, twistedspoon, ModifySpA, Psychic, 1.1, "twistedspoon.onModifySpA");
