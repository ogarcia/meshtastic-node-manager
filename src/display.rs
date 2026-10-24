//! Terminal output: banner, panels and the node table.

use comfy_table::{
    Attribute, Cell, CellAlignment, Color, ContentArrangement, ContentLineStyle, LineStyle, Table,
    TableStyle,
};
use console::{Style, measure_text_width, style};
use jiff::Timestamp;
use jiff::tz::TimeZone;

use crate::fl;
use crate::nodes::{Node, human_delta};

/// Rounded borders, a header separator and no lines between rows.
const TABLE_STYLE: TableStyle = TableStyle::new()
    .top_border(LineStyle::new('╭', '─', '┬', '╮'))
    .header_lines(ContentLineStyle::new('│', '│', '│'))
    .header_separator(LineStyle::new('├', '─', '┼', '┤'))
    .content_lines(ContentLineStyle::new('│', '│', '│'))
    .bottom_border(LineStyle::new('╰', '─', '┴', '╯'));

/// Print `text` inside a rounded box drawn in `color`.
pub fn panel(text: &str, color: console::Color) {
    let border = Style::new().fg(color);
    let width = text.lines().map(measure_text_width).max().unwrap_or(0);
    println!(
        "{}",
        border.apply_to(format!("╭{}╮", "─".repeat(width + 2)))
    );
    for line in text.lines() {
        let pad = " ".repeat(width - measure_text_width(line));
        println!(
            "{} {line}{pad} {}",
            border.apply_to("│"),
            border.apply_to("│")
        );
    }
    println!(
        "{}",
        border.apply_to(format!("╰{}╯", "─".repeat(width + 2)))
    );
}

pub fn banner() {
    panel(
        &format!(
            "{}\n{}",
            style(fl!("app-name")).cyan().bold(),
            style(fl!("app-tagline")).dim()
        ),
        console::Color::Cyan,
    );
    println!();
}

/// Format a Unix timestamp in the local time zone.
fn format_time(epoch_secs: u32) -> String {
    Timestamp::from_second(epoch_secs.into())
        .map(|ts| {
            ts.to_zoned(TimeZone::system())
                .strftime("%Y-%m-%d %H:%M")
                .to_string()
        })
        .unwrap_or_else(|_| "?".into())
}

/// Make a node name safe for column alignment.
///
/// Drops the emoji variation selector (U+FE0F): width tables count symbols
/// such as ☢️ or ⛰️ as one column while terminals draw them as two, which
/// breaks the table borders.  Without the selector they render as one-column
/// text symbols.
fn table_text(text: &str) -> String {
    text.replace('\u{FE0F}', "")
}

/// Print the node table: own node first as row 0, then favorites, then the
/// rest of `nodes` in the given order, with the ones to delete last.
pub fn nodes_table(own: Option<&Node>, nodes: &[Node], now: u64) {
    let mut table = Table::new();
    table
        .load_style(TABLE_STYLE)
        .set_content_arrangement(ContentArrangement::Dynamic);
    if console::colors_enabled() {
        table.enforce_styling();
    }

    let header = |s: String| {
        Cell::new(s)
            .fg(Color::Magenta)
            .add_attribute(Attribute::Bold)
    };
    table.set_header(vec![
        header("#".into()),
        header(String::new()),
        header(fl!("col-id")),
        header(fl!("col-long-name")),
        header(fl!("col-short-name")),
        header(fl!("col-hardware")),
        header(fl!("col-last-heard")),
        header(fl!("col-ago")),
    ]);

    let rows = own
        .map(|n| (n, true))
        .into_iter()
        .chain(nodes.iter().filter(|n| n.favorite).map(|n| (n, false)))
        .chain(
            nodes
                .iter()
                .filter(|n| !n.favorite && !n.delete)
                .map(|n| (n, false)),
        )
        .chain(nodes.iter().filter(|n| n.delete).map(|n| (n, false)));

    for (i, (node, is_own)) in rows.enumerate() {
        let index = if own.is_some() { i } else { i + 1 };
        let marker = if is_own {
            Cell::new("★").fg(Color::Cyan)
        } else if node.delete {
            Cell::new("✗").fg(Color::Red).add_attribute(Attribute::Bold)
        } else if node.favorite {
            Cell::new("♥").fg(Color::Magenta)
        } else if node.ignored {
            Cell::new("⊘").fg(Color::DarkGrey)
        } else {
            Cell::new("·").fg(Color::DarkGrey)
        };
        let id = if node.unknown {
            format!("{} ✦", node.id())
        } else {
            node.id()
        };
        let dash = || "—".to_string();
        let (last, ago) = match node.last_heard {
            Some(t) => (format_time(t), human_delta(now.saturating_sub(t.into()))),
            None if is_own => (dash(), dash()),
            None => (dash(), fl!("never-heard")),
        };
        let row_color = if node.delete {
            Color::Red
        } else {
            Color::Reset
        };
        table.add_row(vec![
            Cell::new(index)
                .fg(Color::DarkGrey)
                .set_alignment(CellAlignment::Right),
            marker.set_alignment(CellAlignment::Center),
            Cell::new(id).fg(Color::Cyan),
            Cell::new(node.long_name.as_deref().map_or_else(dash, table_text)).fg(row_color),
            Cell::new(node.short_name.as_deref().map_or_else(dash, table_text)).fg(Color::Yellow),
            Cell::new(node.hw_model.clone().unwrap_or_else(dash)).fg(Color::Blue),
            Cell::new(last).fg(Color::Green),
            Cell::new(ago).fg(row_color),
        ]);
    }

    println!("{table}");
    println!(
        "{}",
        style(format!(
            "★ {} · ✗ {} · ♥ {} · ⊘ {} · ✦ {}",
            fl!("legend-own"),
            fl!("legend-delete"),
            fl!("legend-favorite"),
            fl!("legend-ignored"),
            fl!("legend-unknown"),
        ))
        .dim()
    );
    println!();
}
