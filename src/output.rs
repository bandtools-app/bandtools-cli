use anyhow::{Context, Result};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Json,
    PrettyJson,
}

impl OutputFormat {
    pub fn render(self, value: &Value) -> Result<String> {
        match self {
            OutputFormat::Json => serde_json::to_string(value).context("failed to render JSON"),
            OutputFormat::PrettyJson => {
                serde_json::to_string_pretty(value).context("failed to render JSON")
            }
        }
    }
}

pub fn print_json(format: OutputFormat, value: &Value) -> Result<()> {
    println!("{}", format.render(value)?);
    Ok(())
}
