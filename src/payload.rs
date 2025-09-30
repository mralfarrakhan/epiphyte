use std::{collections::HashMap, path::PathBuf};

use cli_table::{Cell, Style, Table, print_stdout};
use object::{File, Object};

use crate::config::Identifier;

#[derive(Debug)]
pub struct Metadata {
    alias: Option<String>,
    address: Option<u64>,
}

pub fn analyze_payload<I>(
    payload_path: &PathBuf,
    procedure_paths: I,
) -> Result<HashMap<String, Metadata>, Box<dyn std::error::Error>>
where
    I: IntoIterator<Item = Identifier>,
{
    let payload_binary = std::fs::read(payload_path)?;
    let data = File::parse(&*payload_binary)?;

    let symbol_map: HashMap<String, u64> = data
        .exports()?
        .iter()
        .filter_map(|v| {
            String::from_utf8(v.name().to_vec())
                .ok()
                .map(|name| (name, v.address()))
        })
        .collect();

    let name_map: HashMap<String, String> = procedure_paths
        .into_iter()
        .map(|i| (i.symbol, i.name))
        .collect();

    let mut res = HashMap::with_capacity(symbol_map.len().max(name_map.len()));

    for (symbol, &address) in &symbol_map {
        res.insert(
            symbol.clone(),
            Metadata {
                alias: name_map.get(symbol).cloned(),
                address: Some(address),
            },
        );
    }

    Ok(res)
}

pub fn print_symbol_table(symbol: &HashMap<String, Metadata>) -> Result<(), std::io::Error> {
    let t = symbol
        .keys()
        .map(|s| {
            let m = &symbol[s];

            let path = m.alias.clone().unwrap_or("UNACCESSIBLE".into());
            let address = match m.address {
                Some(a) => format!("{:#x}", a),
                None => "NOT FOUND".into(),
            };

            vec![path.cell(), s.cell(), address.cell()]
        })
        .table()
        .title(vec![
            "Path".cell().bold(true),
            "Symbol".cell().bold(true),
            "Address".cell().bold(true),
        ])
        .bold(true);

    println!("Symbol Table");
    print_stdout(t)
}
