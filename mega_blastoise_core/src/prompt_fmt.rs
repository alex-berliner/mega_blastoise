extern crate alloc;

use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

use battler::{PlayerBattleData, Request};

fn player_label(id: &str) -> &'static str {
    match id {
        "p1" => "Red",
        "p2" => "Blue",
        _ => "?",
    }
}

/// Format the display for a Turn or Switch prompt. Lines are `\n`-separated.
/// Caller handles line endings and the actual input prompt line.
pub fn format_prompt(
    player_id: &str,
    request: &Request,
    player_data: Option<&PlayerBattleData>,
) -> String {
    let mut out = String::new();
    match request {
        Request::Turn(turn) => {
            out.push_str("══════════════════════════════════\n");
            if let Some(pd) = player_data {
                out.push_str(&format_player_state(pd));
                out.push_str("──────────────────────────────────\n");
            }
            let label = player_label(player_id);
            for mon_req in &turn.active {
                let n = mon_req.moves.len().min(4);
                let mon_name = player_data
                    .and_then(|pd| {
                        pd.mons.iter().find(|m| m.player_team_position == mon_req.team_position)
                    })
                    .map(|m| m.summary.name.as_str())
                    .unwrap_or("?");
                out.push_str(&format!(
                    "{} ({}) — choose move for {}\n",
                    label, player_id, mon_name
                ));
                if n == 0 {
                    out.push_str("  (no moves available — will pass automatically)\n");
                } else {
                    for i in 0..n {
                        let m = &mon_req.moves[i];
                        let tag =
                            if m.disabled { " [DISABLED]" } else if m.pp == 0 { " [NO PP]" } else { "" };
                        let avail = if !m.disabled && m.pp > 0 { "  <--" } else { "" };
                        out.push_str(&format!(
                            "  [{}] {:<20}  PP {}/{}{}{}\n",
                            i + 1, m.name, m.pp, m.max_pp, tag, avail
                        ));
                    }
                }
                if !mon_req.trapped {
                    if let Some(pd) = player_data {
                        let bench: Vec<_> = pd.mons.iter().enumerate()
                            .filter(|(_, m)| !m.active && m.hp > 0)
                            .collect();
                        if !bench.is_empty() {
                            out.push_str("  ── or switch ──\n");
                            for (i, m) in &bench {
                                let pct = m.hp as u32 * 100 / m.max_hp.max(1) as u32;
                                out.push_str(&format!(
                                    "  [s{}] {} — {}%\n",
                                    i + 1, m.summary.name, pct
                                ));
                            }
                        }
                    }
                }
            }
        }
        Request::Switch(sw) => {
            out.push_str("══════════════════════════════════\n");
            out.push_str(&format!(
                "SWITCH REQUIRED — {} slot(s) need a replacement\n",
                sw.needs_switch.len()
            ));
            if let Some(pd) = player_data {
                out.push_str(&format_bench_for_switch(pd));
            }
            out.push_str("──────────────────────────────────\n");
        }
        _ => {}
    }
    out
}

/// Format the active + bench state for one player.
///
/// Shows: species, HP%, status, types, ability, item, stat boosts, move list with PP.
pub fn format_player_state(pd: &PlayerBattleData) -> String {
    let mut out = String::new();
    let label = player_label(&pd.id);

    for m in pd.mons.iter().filter(|m| m.active) {
        let status = m.status.as_deref().unwrap_or("ok");
        let item = m.item.as_deref().unwrap_or("—");
        let pct = if m.max_hp > 0 { m.hp as u32 * 100 / m.max_hp as u32 } else { 0 };
        let types: Vec<String> = m.types.iter().map(|t| format!("{t:?}")).collect();
        out.push_str(&format!(
            "{} — {} ({})  HP {}/{} ({}%)  status: {}  types: [{}]\n",
            label, m.summary.name, m.species, m.hp, m.max_hp, pct, status, types.join("/")
        ));
        out.push_str(&format!("  ability: {}  item: {}\n", m.ability, item));
        let b = &m.boosts;
        if b.atk != 0 || b.def != 0 || b.spa != 0 || b.spd != 0 || b.spe != 0 {
            out.push_str(&format!(
                "  boosts  atk:{:+}  def:{:+}  spa:{:+}  spd:{:+}  spe:{:+}\n",
                b.atk, b.def, b.spa, b.spd, b.spe
            ));
        }
        for mv in &m.moves {
            let dis = if mv.disabled { " [DISABLED]" } else { "" };
            out.push_str(&format!("  • {}  {}/{} PP{}\n", mv.name, mv.pp, mv.max_pp, dis));
        }
    }

    let bench_alive: Vec<_> = pd.mons.iter().filter(|m| !m.active && m.hp > 0).collect();
    let bench_fainted: Vec<_> = pd.mons.iter().filter(|m| !m.active && m.hp == 0).collect();
    if !bench_alive.is_empty() {
        let parts: Vec<String> = bench_alive
            .iter()
            .map(|m| {
                let pct = if m.max_hp > 0 { m.hp as u32 * 100 / m.max_hp as u32 } else { 0 };
                format!("{} {}%({}hp)", m.summary.name, pct, m.hp)
            })
            .collect();
        out.push_str(&format!("  bench: {}\n", parts.join("  ")));
    }
    if !bench_fainted.is_empty() {
        let parts: Vec<String> = bench_fainted
            .iter()
            .map(|m| format!("{} [fnt]", m.summary.name))
            .collect();
        out.push_str(&format!("  fainted: {}\n", parts.join("  ")));
    }
    out
}

fn format_bench_for_switch(pd: &PlayerBattleData) -> String {
    let mut out = String::new();
    let label = player_label(&pd.id);
    out.push_str(&format!("  {} party:\n", label));
    for (i, m) in pd.mons.iter().enumerate() {
        let slot = i + 1;
        if m.active {
            out.push_str(&format!(
                "    [{}] {} — active (HP {}/{})\n",
                slot, m.summary.name, m.hp, m.max_hp
            ));
        } else if m.hp == 0 {
            out.push_str(&format!("    [{}] {} — fainted\n", slot, m.summary.name));
        } else {
            let pct = m.hp as u32 * 100 / m.max_hp.max(1) as u32;
            out.push_str(&format!(
                "    [{}] {} — HP {}/{} ({}%)  <-- available\n",
                slot, m.summary.name, m.hp, m.max_hp, pct
            ));
        }
    }
    out
}

/// Format the active Pokémon state for both players (used in post-turn display).
pub fn format_active_state(battle: &mut battler::PublicCoreBattle<'_>) -> String {
    let mut out = String::from("── Active Pokémon ──\n");
    for pid in ["p1", "p2"] {
        let Ok(data) = battle.player_data(pid) else { continue };
        out.push_str(&format_player_state(&data));
    }
    out
}
