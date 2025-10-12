use std::{collections::HashMap, path::PathBuf};

use cli_table::{Cell, Style, Table, print_stdout};
use object::{File, Object};

use crate::{config::Identifier, remote::RemoteProcSignature};

#[derive(Debug, Default)]
pub struct Metadata {
    pub symbol: Option<String>,
    pub address: Option<u64>,
    pub signature: Option<RemoteProcSignature>,
}

impl Metadata {
    pub fn is_valid(&self) -> bool {
        self.symbol.is_some() && self.address.is_some()
    }
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

    let name_map: HashMap<String, (String, RemoteProcSignature)> = procedure_paths
        .into_iter()
        .map(|i| (i.symbol, (i.name, i.signature)))
        .collect();

    let mut res = HashMap::with_capacity(symbol_map.len().max(name_map.len()));

    for (symbol, &address) in &symbol_map {
        match name_map.get(symbol) {
            Some(v) => res.insert(
                symbol.clone(),
                Metadata {
                    symbol: Some(v.0.clone()),
                    address: Some(address),
                    signature: Some(v.1),
                },
            ),
            None => res.insert(symbol.clone(), Default::default()),
        };
    }

    Ok(res)
}

pub fn print_symbol_table(symbol: &HashMap<String, Metadata>) -> Result<(), std::io::Error> {
    let t = symbol
        .keys()
        .map(|s| {
            let m = &symbol[s];

            let path = m.symbol.clone().unwrap_or("UNACCESSIBLE".into());
            let address = match m.address {
                Some(a) => format!("{:#x}", a),
                None => "NOT FOUND".into(),
            };
            let signature = m
                .signature
                .map_or("UNDEFINED".into(), |t| format!("{:?}", t));

            vec![path.cell(), s.cell(), address.cell(), signature.cell()]
        })
        .table()
        .title(vec![
            "Path".cell().bold(true),
            "Symbol".cell().bold(true),
            "Address".cell().bold(true),
            "Type".cell().bold(true),
        ])
        .bold(true);

    println!("[INFO] Symbol Table");
    print_stdout(t)
}
