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
    Json { pretty: bool },
    Tui { colour: bool },
}

impl OutputFormat {
    pub fn render(self, value: &Value) -> Result<String> {
        match self {
            OutputFormat::Json { pretty: false } => {
                serde_json::to_string(value).context("failed to render JSON")
            }
            OutputFormat::Json { pretty: true } => {
                serde_json::to_string_pretty(value).context("failed to render JSON")
            }
            OutputFormat::Tui { colour } => {
                render_tui(value, terminal_width(), colour && stdout_supports_colour())
            }
        }
    }
}

pub fn print_value(format: OutputFormat, value: &Value) -> Result<()> {
    println!("{}", format.render(value)?);
    Ok(())
}

fn render_tui(value: &Value, width: u16, colour: bool) -> Result<String> {
    let model = ResponseModel::from_value(value)?;
    let width = width.clamp(40, 140);
    let header = HeaderModel::for_width(width);
    let header_height = header.height();
    let content_lines = model.content_line_count();
    let footer_height = model.footer_line_count().saturating_add(2).clamp(3, 8) as u16;
    let height = content_lines
        .saturating_add(header_height as usize)
        .saturating_add(4)
        .saturating_add(footer_height as usize)
        .clamp(12, 60) as u16;

    let backend = TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).context("failed to create terminal renderer")?;

    terminal
        .draw(|frame| {
            let chunks = Layout::default()
                .direction(Direction::Vertical)
                .margin(1)
                .constraints(
                    [
                        Constraint::Length(header_height),
                        Constraint::Min(5),
                        Constraint::Length(footer_height),
                    ]
                    .as_ref(),
                )
                .split(frame.size());

            let title = Paragraph::new(header.lines()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan)),
            );
            frame.render_widget(title, chunks[0]);

            model.render(frame, chunks[1]);

            let footer = Paragraph::new(model.footer()).block(
                Block::default()
                    .borders(Borders::ALL)
                    .border_style(Style::default().fg(Color::Cyan))
                    .title(Span::styled("Meta", Style::default().fg(Color::Yellow))),
            );
            frame.render_widget(footer, chunks[2]);
        })
        .context("failed to render terminal response")?;

    Ok(buffer_to_string(terminal.backend().buffer(), colour))
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HeaderModel {
    Logo,
    Compact,
}

impl HeaderModel {
    fn for_width(width: u16) -> Self {
        if width >= 64 {
            Self::Logo
        } else {
            Self::Compact
        }
    }

    fn height(self) -> u16 {
        match self {
            Self::Logo => 7,
            Self::Compact => 3,
        }
    }

    fn lines(self) -> Vec<Spans<'static>> {
        match self {
            Self::Logo => vec![
                Spans::from(Span::styled(
                    r" ____                  _ _____           _     ",
                    logo_style(),
                )),
                Spans::from(Span::styled(
                    r"| __ )  __ _ _ __   __| |_   _|__   ___ | |___ ",
                    logo_style(),
                )),
                Spans::from(Span::styled(
                    r"|  _ \ / _` | '_ \ / _` | | |/ _ \ / _ \| / __|",
                    logo_style(),
                )),
                Spans::from(Span::styled(
                    r"| |_) | (_| | | | | (_| | | | (_) | (_) | \__ \",
                    logo_style(),
                )),
                Spans::from(Span::styled(
                    r"|____/ \__,_|_| |_|\__,_| |_|\___/ \___/|_|___/",
                    logo_style(),
                )),
            ],
            Self::Compact => vec![Spans::from(vec![
                Span::styled(
                    "BandTools",
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD),
                ),
                Span::raw(" response"),
            ])],
        }
    }
}

fn logo_style() -> Style {
    Style::default()
        .fg(Color::Cyan)
        .add_modifier(Modifier::BOLD)
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
                let column_widths = array_column_widths(columns, rows, area.width);
                let rows = rows.iter().map(|row| {
                    Row::new(row.iter().zip(column_widths.iter()).map(|(value, width)| {
                        Cell::from(truncate(value, *width as usize))
                            .style(Style::default().fg(Color::LightGreen))
                    }))
                });
                let widths = column_widths
                    .iter()
                    .copied()
                    .map(Constraint::Length)
                    .collect::<Vec<_>>();
                let table = Table::new(rows)
                    .header(header)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Cyan))
                            .title(Span::styled(
                                title.as_str(),
                                Style::default().fg(Color::Yellow),
                            )),
                    )
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
                        Cell::from(value.clone()).style(Style::default().fg(Color::LightGreen)),
                    ])
                });
                let widths = object_table_widths(area.width);
                let table = Table::new(rows)
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Cyan))
                            .title(Span::styled(
                                title.as_str(),
                                Style::default().fg(Color::Yellow),
                            )),
                    )
                    .widths(&widths)
                    .column_spacing(2);
                frame.render_widget(table, area);
            }
            Self::Text { title, lines, .. } => {
                let paragraph = Paragraph::new(lines.clone())
                    .style(Style::default().fg(Color::LightGreen))
                    .block(
                        Block::default()
                            .borders(Borders::ALL)
                            .border_style(Style::default().fg(Color::Cyan))
                            .title(Span::styled(
                                title.as_str(),
                                Style::default().fg(Color::Yellow),
                            )),
                    )
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
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(
                request_id.to_string(),
                Style::default().fg(Color::LightGreen),
            ),
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
        lines.push(Spans::from(vec![
            Span::styled("page ", Style::default().fg(Color::Yellow)),
            Span::styled(page, Style::default().fg(Color::LightGreen)),
            Span::raw(" of "),
            Span::styled(total_pages, Style::default().fg(Color::LightGreen)),
            Span::raw(", "),
            Span::styled(total, Style::default().fg(Color::LightGreen)),
            Span::raw(" total"),
        ]));
    }

    if let Some(location) = value.get("location").and_then(Value::as_str) {
        lines.push(Spans::from(vec![
            Span::styled(
                "location: ",
                Style::default()
                    .fg(Color::Yellow)
                    .add_modifier(Modifier::BOLD),
            ),
            Span::styled(location.to_string(), Style::default().fg(Color::LightGreen)),
        ]));
    }

    if lines.is_empty() {
        lines.push(Spans::from(Span::styled(
            "No metadata returned",
            Style::default().fg(Color::LightGreen),
        )));
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

fn stdout_supports_colour() -> bool {
    std::io::stdout().is_terminal() && std::env::var_os("NO_COLOR").is_none()
}

fn array_column_widths(columns: &[String], rows: &[Vec<String>], area_width: u16) -> Vec<u16> {
    let count = columns.len();
    if count == 0 {
        return vec![area_width.saturating_sub(2)];
    }

    let usable = area_width
        .saturating_sub(2)
        .saturating_sub((count.saturating_sub(1) * 2) as u16);
    let mut widths = columns
        .iter()
        .enumerate()
        .map(|(index, column)| desired_column_width(column, rows, index))
        .collect::<Vec<_>>();

    while widths.iter().sum::<u16>() > usable {
        let shrinkable = widths
            .iter()
            .enumerate()
            .filter(|(index, width)| **width > column_min_width(&columns[*index]))
            .max_by_key(|(_, width)| **width)
            .or_else(|| {
                widths
                    .iter()
                    .enumerate()
                    .filter(|(_, width)| **width > 1)
                    .max_by_key(|(_, width)| **width)
            });

        let Some((index, _)) = shrinkable else { break };

        widths[index] = widths[index].saturating_sub(1);
    }

    widths
}

fn desired_column_width(column: &str, rows: &[Vec<String>], index: usize) -> u16 {
    let content_width = rows
        .iter()
        .filter_map(|row| row.get(index))
        .map(|value| value.chars().count() as u16)
        .max()
        .unwrap_or(0);
    let observed_width = (column.chars().count() as u16).max(content_width);

    observed_width.clamp(column_min_width(column), column_width_cap(column))
}

fn column_width_cap(column: &str) -> u16 {
    match column {
        "id" => 24,
        "status" | "source" => 14,
        "email_address" => 28,
        "subject" => 34,
        "name" => 24,
        "confirmed" => 10,
        "created_at" | "updated_at" => 20,
        _ => 16,
    }
}

fn column_min_width(column: &str) -> u16 {
    match column {
        "id" => 16,
        "subject" => 22,
        "email_address" | "name" => 18,
        "created_at" | "updated_at" => 14,
        "status" | "source" | "lock_version" => 8,
        "confirmed" => 6,
        _ => 8,
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

fn buffer_to_string(buffer: &tui::buffer::Buffer, colour: bool) -> String {
    let area = buffer.area;
    let mut lines = Vec::new();

    for y in area.y..area.y + area.height {
        let mut line = String::new();
        let last_content_x = (area.x..area.x + area.width)
            .rev()
            .find(|x| !buffer.get(*x, y).symbol.trim().is_empty());

        if let Some(last_content_x) = last_content_x {
            let mut current_style = CellStyle::default();
            for x in area.x..=last_content_x {
                let cell = buffer.get(x, y);
                if colour {
                    let style = CellStyle::from_cell(cell);
                    if style != current_style {
                        line.push_str(style.ansi_transition());
                        current_style = style;
                    }
                }
                line.push_str(cell.symbol.as_str());
            }

            if colour && current_style.is_styled() {
                line.push_str("\x1b[0m");
            }
        }
        lines.push(line);
    }

    while lines.last().is_some_and(|line| line.is_empty()) {
        lines.pop();
    }

    lines.join("\n")
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
struct CellStyle {
    fg: Option<Color>,
    bold: bool,
}

impl CellStyle {
    fn from_cell(cell: &tui::buffer::Cell) -> Self {
        Self {
            fg: match cell.fg {
                Color::Reset => None,
                colour => Some(colour),
            },
            bold: cell.modifier.contains(Modifier::BOLD),
        }
    }

    fn is_styled(self) -> bool {
        self.fg.is_some() || self.bold
    }

    fn ansi_transition(self) -> &'static str {
        match (self.fg, self.bold) {
            (None, false) => "\x1b[0m",
            (Some(Color::Cyan), false) => "\x1b[36m",
            (Some(Color::Cyan), true) => "\x1b[1;36m",
            (Some(Color::LightGreen), false) => "\x1b[92m",
            (Some(Color::LightGreen), true) => "\x1b[1;92m",
            (Some(Color::Yellow), false) => "\x1b[33m",
            (Some(Color::Yellow), true) => "\x1b[1;33m",
            (_, false) => "\x1b[0m",
            (_, true) => "\x1b[1m",
        }
    }
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

        let rendered = render_tui(&value, 100, false).unwrap();

        assert!(rendered.contains(r"| __ )  __ _ _ __   __| |_   _|__   ___ | |___ "));
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

        let rendered = render_tui(&value, 60, false).unwrap();

        assert!(rendered.lines().all(|line| line.chars().count() <= 60));
    }

    #[test]
    fn json_output_pretty_prints_by_default() {
        let value = json!({"data": {"ok": true}});

        assert_eq!(
            OutputFormat::Json { pretty: true }.render(&value).unwrap(),
            "{\n  \"data\": {\n    \"ok\": true\n  }\n}"
        );
    }

    #[test]
    fn compact_json_output_stays_machine_readable() {
        let value = json!({"data": {"ok": true}});

        assert_eq!(
            OutputFormat::Json { pretty: false }.render(&value).unwrap(),
            r#"{"data":{"ok":true}}"#
        );
    }

    #[test]
    fn tui_output_can_be_rendered_without_colour() {
        let value = json!({
            "data": {
                "name": "Release party",
                "status": "confirmed"
            },
            "meta": {
                "request_id": "req_test"
            }
        });

        let rendered = render_tui(&value, 100, false).unwrap();

        assert!(rendered.contains("Release party"));
        assert!(!rendered.contains("\x1b["));
    }

    #[test]
    fn array_widths_prioritise_newsletter_id_and_subject() {
        let columns = vec![
            "id".to_string(),
            "subject".to_string(),
            "status".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
            "lock_version".to_string(),
        ];
        let rows = vec![vec![
            "returning-to-mountain-biking".to_string(),
            "Returning To Mountain Biking After A Long Break".to_string(),
            "draft".to_string(),
            "2026-04-21T06:02:18Z".to_string(),
            "2026-04-21T06:33:52Z".to_string(),
            "0".to_string(),
        ]];

        let widths = array_column_widths(&columns, &rows, 100);

        assert!(widths[0] > widths[2]);
        assert!(widths[1] > widths[0]);
        assert!(widths[1] > widths[2]);
        assert!(widths[2] <= 8);
    }

    #[test]
    fn array_widths_fit_narrow_tables() {
        let columns = vec![
            "id".to_string(),
            "subject".to_string(),
            "status".to_string(),
            "created_at".to_string(),
            "updated_at".to_string(),
            "lock_version".to_string(),
        ];
        let rows = vec![vec![
            "returning-to-mountain-biking".to_string(),
            "Returning To Mountain Biking After A Long Break".to_string(),
            "draft".to_string(),
            "2026-04-21T06:02:18Z".to_string(),
            "2026-04-21T06:33:52Z".to_string(),
            "0".to_string(),
        ]];
        let area_width = 60_u16;
        let spacing = (columns.len().saturating_sub(1) * 2) as u16;
        let usable = area_width.saturating_sub(2).saturating_sub(spacing);

        let width_sum = array_column_widths(&columns, &rows, area_width)
            .into_iter()
            .sum::<u16>();

        assert!(width_sum <= usable);
    }

    #[test]
    fn tui_output_can_include_terminal_colour() {
        let value = json!({
            "data": {
                "name": "Release party",
                "status": "confirmed"
            },
            "meta": {
                "request_id": "req_test"
            }
        });

        let rendered = render_tui(&value, 100, true).unwrap();

        assert!(rendered.contains(r" ____                  _ _____           _     "));
        assert!(rendered.contains("\x1b[36m"));
        assert!(rendered.contains("\x1b[92m"));
        assert!(rendered.contains("\x1b[1;33m"));
    }
}
