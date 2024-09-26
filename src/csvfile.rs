use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Clone)]
pub struct CSVFile {
    pub filepath: PathBuf,
    pub data: Vec<[f64; 2]>,
    pub delimiter: u8,
    pub comment_char: u8,
    pub xcol: usize,
    pub ycol: usize,
    pub skip_header: usize,
    pub skip_footer: usize,
}

impl Default for CSVFile {
    fn default() -> Self {
        Self {
            filepath: "".into(),
            data: vec![],
            delimiter: b',',
            comment_char: b'#',
            xcol: 1,
            ycol: 2,
            skip_header: 0,
            skip_footer: 0,
        }
    }
}

impl CSVFile {
    pub fn new(
        filepath: PathBuf,
        xcol: usize,
        ycol: usize,
        delimiter: u8,
        comment_char: u8,
        skip_header: usize,
        skip_footer: usize,
        error_log: &mut Vec<String>,
    ) -> Option<Self> {
        let rdr = csv::ReaderBuilder::new()
            .comment(Some(comment_char))
            .delimiter(delimiter)
            .from_path(filepath.clone())
            .map_err(|err| {
                error_log.push(format!(
                    "ERROR: could not read CSV file {filepath:?}: {}",
                    err
                ))
            });
        if rdr.is_err() {
            return None;
        }

        let rdr = rdr.unwrap();

        let data = parse_rows(rdr, xcol, ycol, &filepath, error_log);
        if data.is_empty() {
            return None;
        }
        Some(CSVFile {
            filepath,
            data,
            delimiter,
            comment_char,
            xcol,
            ycol,
            skip_header,
            skip_footer,
        })
    }
}

fn parse_rows(
    mut rdr: csv::Reader<std::fs::File>,
    xcol: usize,
    ycol: usize,
    filepath: &Path,
    error_log: &mut Vec<String>,
) -> Vec<[f64; 2]> {
    let mut data = Vec::<[f64; 2]>::new();
    for (i, entry) in rdr.records().enumerate() {
        if let Err(e) = entry {
            error_log.push(format!(
                "WARNING: could not parse row {} of file {filepath:?}: {}",
                i + 1,
                e
            ));
            continue;
        }
        let entry = entry.unwrap();
        let x = entry.iter().nth(xcol).map(|x| x.parse::<f64>());
        let y = entry.iter().nth(ycol).map(|y| y.parse::<f64>());
        match (x, y) {
            (Some(Ok(x)), Some(Ok(y))) => {
                data.push([x, y]);
            }
            (Some(Ok(_)), Some(Err(e))) => {
                error_log.push(format!(
                    "WARNING: y-column {ycol} could not be parsed in entry {} for file {filepath:?}: {}",
                    i + 1,
                    e
                ));
                continue;
            }
            (Some(Err(e)), Some(Ok(_))) => {
                error_log.push(format!(
                    "WARNING: x-column {xcol} could not be parsed in entry {} for file {filepath:?}: {}",
                    i + 1,
                    e
                ));
                continue;
            }
            _ => {
                error_log.push(format!(
                    "WARNING: could not parse columns {xcol}, {ycol} in entry {} for file {filepath:?}",
                    i + 1
                ));
                continue;
            }
        }
    }
    data
}
