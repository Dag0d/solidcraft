//! The card a file browser shows for a design: its picture, then its features and parameters.
//!
//! The card is a small HTML page with no script and no fixed words, only the design's own text
//! (names, expressions, the unit) and counts, so it needs no translation. Colours are the CSS
//! system colours, so the card follows the viewer's light or dark appearance.

use std::fmt::Write;

use crate::Summary;

/// The page's style: system colours and fonts only.
const STYLE: &str = "\
:root{color-scheme:light dark}\
body{margin:0;padding:16px 20px;font:13px -apple-system,system-ui,sans-serif;background:Canvas;color:CanvasText}\
h1{margin:0 0 12px;font-size:17px;font-weight:600;overflow-wrap:anywhere}\
h1 small{margin-left:8px;font-size:12px;font-weight:400;color:GrayText}\
img{display:block;width:100%;max-width:960px;height:auto;margin:0 auto 16px}\
.lists{display:flex;flex-wrap:wrap;gap:12px 32px}\
ol,table{margin:0;padding:0;flex:1 1 220px}\
ol{list-style-position:inside}\
li,td{padding:1px 0;overflow-wrap:anywhere}\
table{border-collapse:collapse;align-self:flex-start}\
td:first-child{padding-right:12px;color:GrayText;white-space:nowrap}\
.more{color:GrayText;list-style:none}";

/// The card for `summary`, with the picture at `picture` (a URL such as `cid:picture`) on top.
pub fn card(summary: &Summary, picture: Option<&str>) -> String {
    let mut page = String::with_capacity(4096);
    page.push_str("<!DOCTYPE html><html><head><meta charset=\"utf-8\"><style>");
    page.push_str(STYLE);
    page.push_str("</style></head><body><h1>");
    escape(&mut page, &summary.name);
    if !summary.units.is_empty() {
        page.push_str("<small>");
        escape(&mut page, &summary.units);
        page.push_str("</small>");
    }
    page.push_str("</h1>");
    if let Some(src) = picture {
        page.push_str("<img alt=\"\" src=\"");
        escape(&mut page, src);
        page.push_str("\">");
    }
    page.push_str("<div class=\"lists\">");
    if summary.feature_count > 0 {
        page.push_str("<ol>");
        for feature in &summary.features {
            page.push_str("<li>");
            escape(&mut page, feature);
            page.push_str("</li>");
        }
        more(&mut page, "li", summary.feature_count, summary.features.len());
        page.push_str("</ol>");
    }
    if summary.parameter_count > 0 {
        page.push_str("<table>");
        for parameter in &summary.parameters {
            page.push_str("<tr><td>");
            escape(&mut page, &parameter.name);
            page.push_str("</td><td>");
            escape(&mut page, &parameter.expression);
            page.push_str("</td></tr>");
        }
        if summary.parameter_count > summary.parameters.len() {
            page.push_str("<tr>");
            more(&mut page, "td", summary.parameter_count, summary.parameters.len());
            page.push_str("</tr>");
        }
        page.push_str("</table>");
    }
    page.push_str("</div></body></html>");
    page
}

/// A `+12` line in `tag` when a list shows fewer than `count` entries.
fn more(page: &mut String, tag: &str, count: usize, shown: usize) {
    if count > shown {
        let _ = write!(page, "<{tag} class=\"more\">+{}</{tag}>", count - shown);
    }
}

/// `text` with HTML's special characters escaped, for an element or a quoted attribute.
fn escape(page: &mut String, text: &str) {
    for c in text.chars() {
        match c {
            '&' => page.push_str("&amp;"),
            '<' => page.push_str("&lt;"),
            '>' => page.push_str("&gt;"),
            '"' => page.push_str("&quot;"),
            '\'' => page.push_str("&#39;"),
            c => page.push(c),
        }
    }
}

#[cfg(test)]
#[path = "card_tests.rs"]
mod tests;
