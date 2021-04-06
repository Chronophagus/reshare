use super::*;

use comfy_table::Table;
use reshare_models::{Error, FileInfo};
use std::iter::FromIterator;

pub fn execute(list: ListArgs) -> Result<()> {
    let server_url = load_configuration()?;

    let query_url = match list.key_phrase {
        Some(key_phrase) => server_url.join("private/")?.join(&key_phrase)?,
        None => server_url,
    };

    let resp = http::get(query_url.clone()).context(format!("Failure quering {}", query_url))?;

    if resp.status().is_success() {
        let files: Vec<FileInfo> = resp.json()?;

        let table: FilesTableView = files.into_iter().collect();

        if table.is_empty() {
            println!("No files are currently available");
        } else {
            println!("{}", table);
        }
    } else {
        let error: Error = resp.json()?;
        anyhow::bail!("{}", error.error_msg)
    }

    Ok(())
}

#[derive(Debug)]
struct FilesTableView {
    table: Table,
    rows_count: usize,
}

impl FilesTableView {
    fn is_empty(&self) -> bool {
        self.rows_count == 0
    }
}

impl FromIterator<FileInfo> for FilesTableView {
    fn from_iter<I: IntoIterator<Item = FileInfo>>(iter: I) -> Self {
        use bytesize::ByteSize;
        use comfy_table::modifiers::UTF8_ROUND_CORNERS;
        use comfy_table::presets::UTF8_FULL;
        use comfy_table::*;

        let mut table = Table::new();

        table
            .load_preset(UTF8_FULL)
            .apply_modifier(UTF8_ROUND_CORNERS)
            .set_header(vec![
                Cell::new("Name")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
                Cell::new("Size")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
                Cell::new("Upload date")
                    .set_alignment(CellAlignment::Center)
                    .add_attribute(Attribute::Bold),
            ]);

        let mut rows_count = 0;
        for item in iter {
            let human_readable_size = ByteSize::b(item.size).to_string_as(false);
            let human_readable_date = item.upload_date.format("%b %d, %H:%M").to_string();
            table.add_row(vec![
                Cell::new(item.name).set_alignment(CellAlignment::Center),
                Cell::new(human_readable_size).set_alignment(CellAlignment::Center),
                Cell::new(human_readable_date).set_alignment(CellAlignment::Center),
            ]);
            rows_count += 1;
        }

        Self { table, rows_count }
    }
}

impl std::fmt::Display for FilesTableView {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        write!(f, "{}", self.table)
    }
}
