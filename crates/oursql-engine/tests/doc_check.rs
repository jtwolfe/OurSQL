//! Docs must describe the code that exists. Nothing more.

use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use oursql_nashcql::keywords::KEYWORDS;
use oursql_wire::{
    T_BIND, T_DONE, T_ERROR, T_HELLO, T_NOTICE, T_PING, T_PODPIS, T_ROWS, T_STMT, T_WELCOME,
};

fn repo() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..")
}

fn read(rel: impl AsRef<Path>) -> String {
    std::fs::read_to_string(repo().join(rel.as_ref()))
        .unwrap_or_else(|e| panic!("missing {}: {e}", rel.as_ref().display()))
}

fn error_ctors() -> Vec<(u16, String)> {
    let src = read("crates/oursql-core/src/error.rs");
    let collapsed = src.replace('\n', " ");
    let mut out = Vec::new();
    let mut i = 0;
    while i < collapsed.len() {
        let rest = &collapsed[i..];
        let Some(rel) = [
            "Self::lang(",
            "Self::bureau(",
            "Self::storage(",
            "Self::mesh(",
        ]
        .iter()
        .filter_map(|n| rest.find(n).map(|p| (p, n.len())))
        .min_by_key(|(p, _)| *p) else {
            break;
        };
        let start = i + rel.0 + rel.1;
        let Some(end_rel) = collapsed[start..].find(')') else {
            break;
        };
        let inner = &collapsed[start..start + end_rel];
        let mut parts = inner.split(',');
        if let (Some(code), Some(name)) = (parts.next(), parts.next()) {
            if let Ok(code) = code.trim().parse::<u16>() {
                let name = name.trim().trim_matches('"').to_string();
                if name.chars().all(|c| c.is_ascii_uppercase() || c == '_') {
                    out.push((code, name));
                }
            }
        }
        i = start + end_rel + 1;
    }
    out
}

fn catalog_rows(md: &str) -> Vec<(u16, String)> {
    let mut out = Vec::new();
    for line in md.lines() {
        let line = line.trim();
        if !line.starts_with('|') {
            continue;
        }
        let cols: Vec<&str> = line
            .split('|')
            .map(|s| s.trim())
            .filter(|s| !s.is_empty())
            .collect();
        if cols.len() < 2 {
            continue;
        }
        if let Ok(code) = cols[0].parse::<u16>() {
            out.push((code, cols[1].to_string()));
        }
    }
    out
}

#[test]
fn error_catalog_matches_constructors() {
    let ctors = error_ctors();
    assert!(!ctors.is_empty());
    let cat = catalog_rows(&read("docs/14-error-catalog.md"));
    let cat_set: BTreeSet<_> = cat.into_iter().collect();
    let ctor_set: BTreeSet<_> = ctors.into_iter().collect();
    for (code, name) in &ctor_set {
        assert!(
            cat_set.iter().any(|(c, n)| c == code && n == name),
            "constructor {code} {name} missing from docs/14-error-catalog.md"
        );
    }
    for (code, name) in &cat_set {
        assert!(
            ctor_set.iter().any(|(c, n)| c == code && n == name),
            "catalog {code} {name} has no Error constructor"
        );
    }
}

#[test]
fn keywords_appear_in_nashcql_docs() {
    let blob = format!(
        "{}\n{}",
        read("docs/06-nashcql.md"),
        read("crates/oursql-nashcql/grammar.md")
    );
    for k in KEYWORDS {
        assert!(
            blob.contains(k.nash),
            "keyword {} not mentioned in 06-nashcql.md or grammar.md",
            k.nash
        );
    }
}

#[test]
fn wire_types_match_docs() {
    let d = read("docs/10-wire-protocol.md");
    let pairs = [
        (T_HELLO, "HELLO", "0x01"),
        (T_WELCOME, "WELCOME", "0x02"),
        (T_STMT, "STMT", "0x03"),
        (T_BIND, "BIND", "0x04"),
        (T_ROWS, "ROWS", "0x05"),
        (T_DONE, "DONE", "0x06"),
        (T_NOTICE, "NOTICE", "0x07"),
        (T_ERROR, "ERROR", "0x08"),
        (T_PODPIS, "PODPIS", "0x09"),
        (T_PING, "PING", "0x0B"),
    ];
    for (val, name, hex) in pairs {
        assert!(d.contains(hex), "wire doc missing {hex}");
        assert!(d.contains(name), "wire doc missing {name}");
        let _ = val;
    }
    assert_eq!(T_PODPIS, 0x09);
    assert!(
        !d.contains("0x0A"),
        "0x0A is not a frame type; do not document it"
    );
}

#[test]
fn unsigned_mutation_code_is_2110() {
    let seven = read("docs/07-comrades-and-auth.md");
    assert!(
        seven.contains("2110") && seven.contains("UNSIGNED_MUTATION"),
        "07 must cite 2110 UNSIGNED_MUTATION"
    );
    assert!(
        !seven.contains("2109 UNSIGNED"),
        "07 must not call 2109 UNSIGNED (that code is PERESTROJ_WAIT)"
    );
}

#[test]
fn official_docs_are_us_keyboard_ascii() {
    let docs = repo().join("docs");
    for ent in std::fs::read_dir(&docs).unwrap() {
        let ent = ent.unwrap();
        if ent.path().extension().and_then(|s| s.to_str()) != Some("md") {
            continue;
        }
        let text = std::fs::read_to_string(ent.path()).unwrap();
        for (i, ch) in text.chars().enumerate() {
            assert!(
                ch.is_ascii(),
                "{} offset {i} is non-ASCII U+{:04X}",
                ent.path().display(),
                ch as u32
            );
        }
    }
}
