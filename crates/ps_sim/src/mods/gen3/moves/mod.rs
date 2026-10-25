//! Gen 3 moves with handlers, one file per move.

pub mod attract;
pub mod block;
pub mod curse;
pub mod destinybond;
pub mod disable;
pub mod followme;
pub mod foresight;
pub mod helpinghand;
pub mod hiddenpower;
pub mod imprison;
pub mod leechseed;
pub mod lockon;
pub mod meanlook;
pub mod mindreader;
pub mod odorsleuth;
pub mod perishsong;
pub mod spiderweb;
pub mod struggle;
pub mod substitute;
pub mod teleport;
pub mod yawn;

use crate::effect::EventArgs;
use crate::relay::Relay;
use crate::state::Battle;
use ps_dex::gen3::conditions;

/// `onHit(target, source, move)` of Block, Mean Look and Spider Web:
/// `return target.addVolatile('trapped', source, move, 'trapper')`.
// PS: data/moves.ts block.onHit, meanlook.onHit, spiderweb.onHit
fn trap_on_hit(b: &mut Battle, a: &EventArgs) -> Option<Relay> {
    let target = a.target.mon()?;
    let source = a.source.expect("a trapping move without a source");
    Some(b.add_volatile_linked(target, conditions::TRAPPED, source, a.effect, conditions::TRAPPER))
}
