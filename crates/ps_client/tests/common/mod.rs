//! Helpers shared by the integration tests.

#![allow(dead_code)]

use std::fmt::Write as _;
use std::io::Read;
use std::path::{Path, PathBuf};

use ps_dex::{Dex, Type};
use ps_protocol::text::names::Tables;
use ps_protocol::to_id;

pub fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../ps_protocol/tests/fixtures").join(name)
}

pub fn read_gz(name: &str) -> String {
    let raw = std::fs::read(fixture(name)).unwrap();
    let mut s = String::new();
    flate2::read::GzDecoder::new(&raw[..]).read_to_string(&mut s).unwrap();
    s
}

pub fn id(s: &str) -> String {
    to_id::<64>(s).unwrap().as_str().to_string()
}

/// Codec tables numbered like `dex`; natures and cosmetic formes (not in
/// `ps_dex`) come from P1-4's `names.tsv`.
pub fn tables(dex: &Dex) -> Tables {
    let mut tsv = String::new();
    for m in dex.moves {
        writeln!(tsv, "move\t{}\t{}", id(m.name.as_str()), m.name.as_str()).unwrap();
    }
    for s in dex.species {
        writeln!(tsv, "species\t{}\t{}", id(s.name.as_str()), s.name.as_str()).unwrap();
    }
    for i in dex.items {
        writeln!(tsv, "item\t{}\t{}", id(i.name.as_str()), i.name.as_str()).unwrap();
    }
    for a in dex.abilities {
        writeln!(tsv, "ability\t{}\t{}", id(a.name.as_str()), a.name.as_str()).unwrap();
    }
    for t in Type::ALL {
        writeln!(tsv, "type\t{}\t{}", id(t.ps_name()), t.ps_name()).unwrap();
    }
    let names = std::fs::read_to_string(fixture("names.tsv")).unwrap();
    for line in names.lines().filter(|l| l.starts_with("nature\t") || l.starts_with("cosmetic\t")) {
        writeln!(tsv, "{line}").unwrap();
    }
    Tables::from_tsv(&tsv)
}

