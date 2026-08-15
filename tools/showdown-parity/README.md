# Showdown parity

Differential tests: the core engines (`gen1_battle`, `gen3_battle`) against
the real Pokémon Showdown simulator.

    cd tools/showdown-parity && npm install     # arm (one-time, needs network)
    cargo test -p mega-blastoise-test --test showdown_parity

Unarmed checkouts skip the suite loudly and stay green.

Randomness is scripted on both sides, not disabled: each scenario fixes
hit/miss, crit, and the exact damage roll, the harness forces the sim down
that branch (accuracy at the old-gen `randomChance` seam, the roll at
`randomizer`/`random(217,256)`, crits via `willCrit`), and the Rust side takes
the same script through its `*_scripted` seams. Misses spending PP, crit
stage/screen rules, and both roll extremes are all first-class comparisons.

Found on its first runs: two transposed cells in type-chart.json (shipped in
Gen 1 as Bug->Rock x0.5 and Fighting->Dragon x2), Shedinja's 1 HP rule,
the Gen 3 damage roll applying last rather than before STAB, the +2 landing
after burn/screen halving, and one wrong patch added from memory during the
fixing (gen 1 Bug->Ghost is resisted, not super effective).
