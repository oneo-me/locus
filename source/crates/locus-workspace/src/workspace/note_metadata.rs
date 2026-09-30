use gpui_kit::component::{Sizable as _, Theme, description_list::DescriptionList};
use gpui_kit::{Div, FontWeight, Pixels, TextRun, Window, div, prelude::*, px, rems};
use locus_library::LibraryNode;
use serde_json::Value;

pub(super) fn note_metadata(
    node: &LibraryNode,
    panel_color: gpui_kit::Hsla,
    theme: &Theme,
    font_family: &str,
    window: &mut Window,
) -> Option<Div> {
    let summary = node
        .summary
        .as_ref()
        .filter(|summary| !summary.trim().is_empty());
    if summary.is_none() && node.additional_metadata.is_empty() {
        return None;
    }

    let mut panel = div()
        .w_full()
        .rounded(px(8.0))
        .overflow_hidden()
        .bg(panel_color)
        .flex()
        .flex_col();
    if let Some(summary) = summary {
        panel = panel.child(
            div()
                .px(px(14.0))
                .py(px(8.0))
                .when(node.canvas_path.is_some(), |summary| summary.px_4().py_1())
                .text_sm()
                .line_height(px(21.0))
                .child(summary.clone()),
        );
    }
    if !node.additional_metadata.is_empty() {
        let fields = node
            .additional_metadata
            .iter()
            .map(|(key, value)| (key.clone(), value))
            .collect::<Vec<_>>();
        panel = panel.child(
            div()
                .py_1()
                .when(node.canvas_path.is_some(), |fields| fields.py_0())
                .child(metadata_list(
                    &fields,
                    node.canvas_path.is_some(),
                    theme,
                    font_family,
                    window,
                )),
        );
    }
    Some(panel)
}

fn metadata_list(
    fields: &[(String, &Value)],
    canvas: bool,
    theme: &Theme,
    font_family: &str,
    window: &mut Window,
) -> DescriptionList {
    let font_size = rems(0.75).to_pixels(window.rem_size());
    let mut font = gpui_kit::font(font_family.to_owned());
    font.weight = FontWeight::MEDIUM;
    let label_width = fields
        .iter()
        .flat_map(|(key, _)| key.lines())
        .map(|key| {
            window
                .text_system()
                .shape_line(
                    key.to_owned().into(),
                    font_size,
                    &[TextRun {
                        len: key.len(),
                        font: font.clone(),
                        color: theme.muted_foreground,
                        ..Default::default()
                    }],
                    None,
                )
                .width()
        })
        .fold(Pixels::ZERO, Pixels::max);
    // DescriptionList requires a definite width. Measure each level with the same
    // font as its labels, retaining the full key width and rounding up for layout.
    let padding = rems(if canvas { 2.0 } else { 1.75 }).to_pixels(window.rem_size());
    let label_width = px(f32::from(label_width + padding).ceil());
    let mut list = DescriptionList::horizontal()
        .columns(1)
        .bordered(false)
        .small()
        .label_width(label_width);
    for (key, value) in fields {
        let label = metadata_cell(canvas)
            .font_family(font_family.to_owned())
            .whitespace_nowrap()
            .font_weight(FontWeight::MEDIUM)
            .text_color(theme.muted_foreground)
            .child(key.clone());
        let value = if let Some(children) = metadata_children(value) {
            div()
                .min_w_0()
                .child(metadata_list(&children, canvas, theme, font_family, window))
        } else {
            metadata_cell(canvas)
                .text_color(theme.foreground)
                .child(metadata_scalar(value))
        };
        list = list.item(label.into_any_element(), value.into_any_element(), 1);
    }
    list
}

fn metadata_cell(canvas: bool) -> Div {
    div()
        .min_w_0()
        .px(rems(0.875))
        .py(rems(0.125))
        .when(canvas, |field| field.px_4())
        .text_xs()
        .line_height(rems(1.125))
        .whitespace_normal()
}

fn metadata_children(value: &Value) -> Option<Vec<(String, &Value)>> {
    match value {
        Value::Object(fields) if !fields.is_empty() => Some(
            fields
                .iter()
                .map(|(key, value)| (key.clone(), value))
                .collect(),
        ),
        Value::Array(items) if !items.is_empty() => Some(
            items
                .iter()
                .enumerate()
                .map(|(ix, value)| (format!("[{}]", ix + 1), value))
                .collect(),
        ),
        _ => None,
    }
}

fn metadata_scalar(value: &Value) -> String {
    match value {
        Value::String(value) => value.clone(),
        Value::Null => "null".into(),
        Value::Bool(value) => value.to_string(),
        Value::Number(value) => value.to_string(),
        Value::Object(_) => "{}".into(),
        Value::Array(_) => "[]".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn nested_yaml_values_expand_without_serializing_containers() {
        let yaml = "shortcuts:\n  github: https://github.com/example/project\n  directory: ~/Projects/example\ntargets:\n  - name: macOS\n    enabled: true\n  - null\n  - [one, 2]\n";
        let value: Value = serde_saphyr::from_str(yaml).unwrap();
        let root = metadata_children(&value).unwrap();
        let shortcuts =
            metadata_children(root.iter().find(|(key, _)| key == "shortcuts").unwrap().1).unwrap();
        assert!(shortcuts.iter().any(|(key, value)| key == "github"
            && metadata_scalar(value) == "https://github.com/example/project"));
        let targets =
            metadata_children(root.iter().find(|(key, _)| key == "targets").unwrap().1).unwrap();
        assert_eq!(
            targets
                .iter()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            ["[1]", "[2]", "[3]"]
        );
        let first = metadata_children(targets[0].1).unwrap();
        assert!(
            first
                .iter()
                .any(|(key, value)| key == "enabled" && metadata_scalar(value) == "true")
        );
        assert_eq!(metadata_scalar(targets[1].1), "null");
        let nested_array = metadata_children(targets[2].1).unwrap();
        assert_eq!(metadata_scalar(nested_array[1].1), "2");
    }

    #[test]
    fn empty_containers_null_and_scalar_strings_remain_distinct() {
        for (value, expected) in [
            (json!({}), "{}"),
            (json!([]), "[]"),
            (Value::Null, "null"),
            (json!("line 1\nline 2"), "line 1\nline 2"),
            (json!(false), "false"),
            (json!(42), "42"),
        ] {
            assert!(metadata_children(&value).is_none());
            assert_eq!(metadata_scalar(&value), expected);
        }
    }
}
