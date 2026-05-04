use std::io::IsTerminal;

use anyhow::{Context, Result};
use serde_json::Value;
use tui::{
    Terminal,
    backend::TestBackend,
    layout::{Constraint, Direction, Layout, Rect},
    style::{Color, Modifier, Style},
    text::{Span, Spans},
    widgets::{Block, Borders, Cell, Paragraph, Row, Table, Wrap},
};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum OutputFormat {
    Json,
    Tui,
}

impl OutputFormat {
    pub fn render(self, value: &Value) -> Result<String> {
        match self {
            OutputFormat::Json => serde_json::to_string(value).context("failed to render JSON"),
            OutputFormat::Tui => render_tui(value, terminal_width()),
        }
    }
}

pub fn print_value(format: OutputFormat, value: &Value) -> Result<()> {
    println!("{}", format.render(value)?);
    Ok(())
}

fn render_tui(value: &Value, width: u16) -> Result<String> {
    let model = ResponseModel::from_value(value)?;
    let content_lines = model.content_line_count();
    let footer_height = model.footer_line_count().saturating_add(2).clamp(3, 8) as u16;
    let height = content_lines
        .saturating_add(7)
        .saturating_add(footer_height as usize)
        .clamp(12, 60) as u16;
    let width = width.clamp(40, 140);

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).context("failed to create terminal renderer")?;

    terminal
        .draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(3),
                        Constraint::Min(5),
                        Constraint::Length(footer_height),
                    ]
                    .as_ref(),
                )
                .split(frame.size());

            let title = Paragraph::new(Spans::from(vec![
                Span::styled(
                    "BandTools",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" response"),
            ]))
            .block(Block::default().borders(Borders::ALL).title("bt"));
            frame.render_widget(title, chunks[0]);

            model.render(frame, chunks[1]);

            let footer = Paragraph::new(model.footer())
                .block(Block::default().borders(Borders::ALL).title("Meta"));
            frame.render_widget(footer, chunks[2]);
        })
        .context("failed to render terminal response")?;

    Ok(buffer_to_string(terminal.backend().buffer()))
}

#[derive(Debug)]
enum ResponseModel {
    Array {
        title: String,
        rows: Vec<Vec<String>>,
        columns: Vec<String>,
        footer: Vec<Spans<'static>>,
    },
    Object {
        title: String,
        rows: Vec<(String, String)>,
        footer: Vec<Spans<'static>>,
    },
    Text {
        title: String,
        lines: Vec<Spans<'static>>,
        footer: Vec<Spans<'static>>,
    },
}

impl ResponseModel {
    fn from_value(value: &Value) -> Result<Self> {
        let footer = footer_lines(value);
        let data = value.get("data").unwrap_or(value);

        match data {
            Value::Array(items) => {
                let columns = array_columns(items);
                if columns.is_empty() {
                    return Ok(Self::Text {
                        title: "Data".to_string(),
                        lines: vec![Spans::from("No items returned")],
                        footer,
                    });
                }

                let rows = items
                    .iter()
                    .map(|item| {
                        columns
                            .iter()
                            .map(|column| {
                                item.get(column)
                                    .map(format_value)
                                    .unwrap_or_else(|| "-".to_string())
                            })
                            .collect()
                    })
                    .collect();

                Ok(Self::Array {
                    title: format!("Data: {} item(s)", items.len()),
                    rows,
                    columns,
                    footer,
                })
            }
            Value::Object(object) => {
                let rows = object
                    .iter()
                    .map(|(key, value)| (key.clone(), format_value(value)))
                    .collect();
                Ok(Self::Object {
                    title: "Data".to_string(),
                    rows,
                    footer,
                })
            }
            _ => Ok(Self::Text {
                title: "Data".to_string(),
                lines: vec![Spans::from(format_value(data))],
                footer,
            }),
        }
    }

    fn content_line_count(&self) -> usize {
        match self {
            Self::Array { rows, .. } => rows.len().saturating_add(3),
            Self::Object { rows, .. } => rows.len().saturating_add(2),
            Self::Text { lines, .. } => lines.len().saturating_add(2),
        }
    }

    fn footer(&self) -> Vec<Spans<'static>> {
        match self {
            Self::Array { footer, .. }
            | Self::Object { footer, .. }
            | Self::Text { footer, .. } => footer.clone(),
        }
    }

    fn footer_line_count(&self) -> usize {
        match self {
            Self::Array { footer, .. }
            | Self::Object { footer, .. }
            | Self::Text { footer, .. } => footer.len(),
        }
    }

    fn render<B: tui::backend::Backend>(&self, frame: &mut tui::Frame<B>, area: Rect) {
        match self {
            Self::Array {
                title,
                rows,
                columns,
                ..
            } => {
                let header = Row::new(columns.iter().map(|column| {
                    Cell::from(column.clone()).style(
                        Style::default()
                            .fg(Color::Yellow)
                            .add_modifier(Modifier::BOLD),
                    )
                }))
                .bottom_margin(1);
                let rows = rows
                    .iter()
                    .map(|row| Row::new(row.iter().map(|value| Cell::from(truncate(value, 36)))));
                let widths = array_table_widths(columns, area.width);
                let table = Table::new(rows)
                    .header(header)
                    .block(Block::default().borders(Borders::ALL).title(title.as_str()))
                    .widths(&widths)
                    .column_spacing(2);
                frame.render_widget(table, area);
            }
            Self::Object { title, rows, .. } => {
                let rows = rows.iter().map(|(key, value)| {
                    Row::new(vec![
                        Cell::from(key.clone()).style(
                            Style::default()
                                .fg(Color::Yellow)
                                .add_modifier(Modifier::BOLD),
                        ),
                        Cell::from(value.clone()),
                    ])
                });
                let widths = object_table_widths(area.width);
                let table = Table::new(rows)
                    .block(Block::default().borders(Borders::ALL).title(title.as_str()))
                    .widths(&widths)
                    .column_spacing(2);
                frame.render_widget(table, area);
            }
            Self::Text { title, lines, .. } => {
                let paragraph = Paragraph::new(lines.clone())
                    .block(Block::default().borders(Borders::ALL).title(title.as_str()))
                    .wrap(Wrap { trim: false });
                frame.render_widget(paragraph, area);
            }
        }
    }
}

fn array_columns(items: &[Value]) -> Vec<String> {
    let preferred = [
        "id",
        "email_address",
        "subject",
        "name",
        "status",
        "confirmed",
        "source",
        "created_at",
        "updated_at",
    ];
    let mut columns = Vec::new();

    for key in preferred {
        if items
            .iter()
            .any(|item| item.get(key).is_some_and(is_displayable))
        {
            columns.push(key.to_string());
        }
    }

    for item in items {
        if let Some(object) = item.as_object() {
            for (key, value) in object {
                if columns.len() >= 6 {
                    return columns;
                }
                if is_displayable(value) && !columns.iter().any(|column| column == key) {
                    columns.push(key.clone());
                }
            }
        }
    }

    columns
}

fn footer_lines(value: &Value) -> Vec<Spans<'static>> {
    let mut lines = Vec::new();
    if let Some(request_id) = value.pointer("/meta/request_id").and_then(Value::as_str) {
        lines.push(Spans::from(vec![
            Span::styled(
                "request_id: ",
                Style::default().add_modifier(Modifier::BOLD),
            ),
            Span::raw(request_id.to_string()),
        ]));
    }

    if let Some(pagination) = value.pointer("/meta/pagination").and_then(Value::as_object) {
        let page = pagination
            .get("page")
            .map(format_value)
            .unwrap_or_else(|| "-".to_string());
        let total = pagination
            .get("total")
            .map(format_value)
            .unwrap_or_else(|| "-".to_string());
        let total_pages = pagination
            .get("total_pages")
            .map(format_value)
            .unwrap_or_else(|| "-".to_string());
        lines.push(Spans::from(format!(
            "page {page} of {total_pages}, {total} total"
        )));
    }

    if let Some(location) = value.get("location").and_then(Value::as_str) {
        lines.push(Spans::from(format!("location: {location}")));
    }

    if lines.is_empty() {
        lines.push(Spans::from("No metadata returned"));
    }

    lines
}

fn is_displayable(value: &Value) -> bool {
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
}

fn format_value(value: &Value) -> String {
    match value {
        Value::Null => "-".to_string(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::String(value) => value.clone(),
        Value::Array(values) => format!("{} item(s)", values.len()),
        Value::Object(_) => serde_json::to_string(value).unwrap_or_else(|_| "{...}".to_string()),
    }
}

fn terminal_width() -> u16 {
    if !std::io::stdout().is_terminal() {
        return 100;
    }

    terminal_size::terminal_size()
        .map(|(width, _)| width.0)
        .unwrap_or(100)
}

fn array_table_widths(columns: &[String], area_width: u16) -> Vec<Constraint> {
    let count = columns.len();
    if count == 0 {
        return vec![Constraint::Percentage(100)];
    }

    let usable = area_width
        .saturating_sub(2)
        .saturating_sub((count.saturating_sub(1) * 2) as u16);
    let equal_width = (usable / count as u16).max(6);

    columns
        .iter()
        .map(|column| {
            let desired = column_width_hint(column).min(equal_width.max(12));
            Constraint::Length(desired)
        })
        .collect()
}

fn column_width_hint(column: &str) -> u16 {
    match column {
        "id" | "status" | "source" => 14,
        "email_address" => 28,
        "subject" | "name" => 24,
        "confirmed" => 10,
        "created_at" | "updated_at" => 20,
        _ => 16,
    }
}

fn object_table_widths(area_width: u16) -> [Constraint; 2] {
    let usable = area_width.saturating_sub(4);
    let key_width = usable.saturating_mul(35).saturating_div(100).clamp(12, 28);
    let value_width = usable.saturating_sub(key_width).max(10);

    [
        Constraint::Length(key_width),
        Constraint::Length(value_width),
    ]
}

fn truncate(value: &str, max_chars: usize) -> String {
    if value.chars().count() <= max_chars {
        return value.to_string();
    }

    let mut truncated = value
        .chars()
        .take(max_chars.saturating_sub(1))
        .collect::<String>();
    truncated.push('…');
    truncated
}

fn buffer_to_string(buffer: &tui::buffer::Buffer) -> String {
    let area = buffer.area;
    let mut lines = Vec::new();

    for y in area.y..area.y + area.height {
        let mut line = String::new();
        for x in area.x..area.x + area.width {
            line.push_str(buffer.get(x, y).symbol.as_str());
        }
        lines.push(line.trim_end().to_string());
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn renders_array_response_as_terminal_ui() {
        let value = json!({
            "data": [
                {
                    "id": "sub_1",
                    "email_address": "fan@example.com",
                    "confirmed": true
                }
            ],
            "meta": {
                "request_id": "req_test",
                "pagination": {
                    "page": 1,
                    "per_page": 25,
                    "total": 1,
                    "total_pages": 1,
                    "next_page": null,
                    "prev_page": null
                }
            }
        });

        let rendered = render_tui(&value, 100).unwrap();

        assert!(rendered.contains("BandTools"));
        assert!(rendered.contains("fan@example.com"));
        assert!(rendered.contains("req_test"));
        assert!(rendered.contains("page 1 of 1, 1 total"));
    }

    #[test]
    fn tui_output_respects_narrow_terminal_width() {
        let value = json!({
            "data": {
                "api_url": "http://localhost:3000/api/v1",
                "config_path": "/tmp/bandtools/config.toml"
            }
        });

        let rendered = render_tui(&value, 60).unwrap();

        assert!(rendered.lines().all(|line| line.chars().count() <= 60));
    }

    #[test]
    fn json_output_stays_machine_readable() {
        let value = json!({"data": {"ok": true}});

        assert_eq!(
            OutputFormat::Json.render(&value).unwrap(),
            r#"{"data":{"ok":true}}"#
        );
    }
}
