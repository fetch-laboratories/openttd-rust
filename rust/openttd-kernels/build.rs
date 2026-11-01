//! List every module that invokes `layouts!` or `shared_records!`, so `abi::layout` needs no registry file.

use std::fmt::Write as _;
use std::path::Path;
use std::{env, fs, io};

fn collect(dir: &Path, module: &str, found: &mut Vec<String>) -> io::Result<()> {
    for entry in fs::read_dir(dir)? {
        let path = entry?.path();
        let stem = path
            .file_stem()
            .and_then(|stem| stem.to_str())
            .unwrap_or("");
        let name = if module.is_empty() {
            stem.to_owned()
        } else {
            format!("{module}::{stem}")
        };
        if path.is_dir() {
            collect(&path, &name, found)?;
        } else if path.extension().is_some_and(|ext| ext == "rs") && !["lib", "abi"].contains(&stem) {
            let source = fs::read_to_string(&path)?.replace("\r\n", "\n");
            for (marker, function) in [
                ("layouts! {\n    fn abi_layout;", "abi_layout"),
                ("shared_records! {\n    fn abi_record;", "abi_record"),
            ] {
                if source.contains(marker) {
                    println!("cargo:rerun-if-changed={}", path.display());
                    found.push(format!("crate::{name}::{function}"));
                }
            }
        }
    }
    Ok(())
}

fn main() -> io::Result<()> {
    println!("cargo:rerun-if-changed=src");
    let mut found = Vec::new();
    collect(Path::new("src"), "", &mut found)?;
    found.sort();
    let mut list = String::from("[\n");
    for name in &found {
        writeln!(list, "    {name} as fn(&str, usize) -> usize,").unwrap();
    }
    list.push(']');
    fs::write(
        Path::new(&env::var("OUT_DIR").unwrap()).join("abi_layouts.rs"),
        list,
    )
}
