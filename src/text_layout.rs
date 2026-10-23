//! Font-measured lines shared by screen and file rendering.
use crate::model::*;
#[derive(Clone)]
pub struct TextLine {
    pub pos: Point,
    pub text: String,
    pub size: f32,
    pub italic: bool,
    pub underline: bool,
}
pub struct ClassLayout {
    pub size: Point,
    pub lines: Vec<TextLine>,
    pub separators: Vec<f32>,
    pub overflow: bool,
}
pub fn wrap(text: &str, size: f32, width: f32) -> Vec<String> {
    let mut lines = vec![];
    for paragraph in text.split('\n') {
        let mut line = String::new();
        let mut measured = 0.;
        for c in paragraph.chars() {
            let mut buffer = [0; 4];
            let advance = text_width(c.encode_utf8(&mut buffer), size);
            if !line.is_empty() && measured + advance > width.max(size) {
                lines.push(std::mem::take(&mut line));
                measured = 0.;
            }
            line.push(c);
            measured += advance;
        }
        lines.push(line);
    }
    if lines.is_empty() {
        lines.push(String::new());
    }
    lines
}
pub fn class_layout(n: &Node, manual_width: Option<f32>) -> ClassLayout {
    let natural_width = std::iter::once(text_width(&n.name, 16.))
        .chain(n.attributes.iter().map(|m| text_width(&m.text(false), 14.)))
        .chain(n.methods.iter().map(|m| text_width(&m.text(true), 14.)))
        .fold(160., f32::max)
        + 32.;
    let width = manual_width.unwrap_or(natural_width).max(50.);
    let mut out = ClassLayout {
        size: Point::new(width, 0.),
        lines: vec![],
        separators: vec![],
        overflow: false,
    };
    let mut y = 8.;
    let stereotype = match n.kind {
        NodeKind::Interface => Some("«interface»"),
        NodeKind::Enumeration => Some("«enumeration»"),
        _ => None,
    };
    if let Some(text) = stereotype {
        out.lines.push(TextLine {
            pos: Point::new(12., y),
            text: text.into(),
            size: 12.,
            italic: false,
            underline: false,
        });
        y += 18.;
    }
    for text in wrap(&n.name, 16., width - 28.) {
        out.lines.push(TextLine {
            pos: Point::new(12., y),
            text,
            size: 16.,
            italic: n.kind == NodeKind::AbstractClass,
            underline: false,
        });
        y += 24.;
    }
    y += 6.;
    for (members, method) in [(&n.attributes, false), (&n.methods, true)] {
        if members.is_empty() && n.hide_empty {
            continue;
        }
        out.separators.push(y);
        let mut count = 0;
        for m in members {
            for text in wrap(&m.text(method), 14., width - 24.) {
                out.lines.push(TextLine {
                    pos: Point::new(10., y + 4. + count as f32 * 24.),
                    text,
                    size: 14.,
                    italic: m.is_abstract,
                    underline: m.is_static,
                });
                count += 1;
            }
        }
        y += count.max(1) as f32 * 24.;
    }
    out.size.y = (y + 8.).max(55.);
    out.overflow = manual_width.is_some() && out.size.y > n.layout.size.y;
    out
}
