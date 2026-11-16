use super::*;
use crate::Parameter;

fn summary() -> Summary {
    Summary {
        name: "Filter cap".into(),
        units: "mm".into(),
        features: vec!["Sketch1".into(), "Extrude1".into()],
        feature_count: 2,
        parameters: vec![Parameter { name: "F_d1".into(), expression: "60 mm".into() }],
        parameter_count: 1,
    }
}

#[test]
fn the_card_shows_the_picture_features_and_parameters() {
    let page = card(&summary(), Some("cid:picture"));
    assert!(page.contains("<h1>Filter cap<small>mm</small></h1>"));
    assert!(page.contains("<img alt=\"\" src=\"cid:picture\">"));
    assert!(page.contains("<li>Sketch1</li><li>Extrude1</li>"));
    assert!(page.contains("<td>F_d1</td><td>60 mm</td>"));
    assert!(!page.contains("class=\"more\""));
}

#[test]
fn a_card_without_a_picture_or_lists_has_just_the_name() {
    let page = card(&Summary { name: "Empty".into(), ..Summary::default() }, None);
    assert!(page.contains("<h1>Empty</h1>"));
    assert!(!page.contains("<img"));
    assert!(!page.contains("<ol>"));
    assert!(!page.contains("<table>"));
}

#[test]
fn the_designs_own_text_cannot_add_markup() {
    let hostile =
        Summary { name: "<script>alert(1)</script>".into(), features: vec!["a\" onload=\"x".into()], feature_count: 1, ..Summary::default() };
    let page = card(&hostile, Some("cid:\"><script>"));
    assert!(!page.contains("<script>"));
    assert!(page.contains("&lt;script&gt;alert(1)&lt;/script&gt;"));
    assert!(page.contains("a&quot; onload=&quot;x"));
}

#[test]
fn long_lists_count_what_they_leave_out() {
    let long = Summary { feature_count: 250, parameter_count: 3, ..summary() };
    let page = card(&long, None);
    assert!(page.contains("<li class=\"more\">+248</li>"));
    assert!(page.contains("<tr><td class=\"more\">+2</td></tr>"));
}
