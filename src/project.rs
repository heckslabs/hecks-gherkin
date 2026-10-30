//! Line-by-line projection of sentences into bluebook text.

use std::fmt;

/// A refusal: the line and column a sentence stopped fitting at, and why.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Diagnostic {
    /// 1-based line.
    pub line: usize,
    /// 1-based column of the first character that did not fit.
    pub column: usize,
    /// What was refused.
    pub message: String,
}

impl fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}:{}: {}", self.line, self.column, self.message)
    }
}

/// Where the projection is inside the sentence file.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Block {
    Domain,
    Aggregate,
    Has,
}

/// Projects a sentence file into bluebook text.
///
/// Supported so far: `Domain:` with its paragraph, `Aggregate:` with its paragraph, the
/// `is identified by` sentence, and `It has:` lines whose types are `text` or `whole number`.
///
/// # Errors
///
/// Returns the first line no phrase matches, with its column.
pub fn project(source: &str) -> Result<String, Diagnostic> {
    let mut out: Vec<String> = Vec::new();
    let mut open: Vec<Block> = Vec::new();
    let mut has_domain = false;
    let mut header_open = false;
    let mut aggregate = String::new();

    for (index, raw) in source.lines().enumerate() {
        let line_no = index + 1;
        let trimmed = raw.trim();
        if trimmed.is_empty() || trimmed.starts_with('#') {
            continue;
        }
        let column = raw.len() - raw.trim_start().len() + 1;
        let fail = |message: &str| Diagnostic {
            line: line_no,
            column,
            message: message.to_string(),
        };

        // True only on the line straight after a `Domain:` or `Aggregate:` header, where a
        // paragraph is the header's description.
        let after_header = std::mem::take(&mut header_open);

        if let Some(name) = trimmed.strip_prefix("Domain:") {
            if !open.is_empty() {
                return Err(fail("a domain starts at the top of the file"));
            }
            out.push(format!("Hecks.bluebook \"{}\" do", quoted(name.trim(), &fail)?));
            open.push(Block::Domain);
            has_domain = true;
            header_open = true;
        } else if let Some(name) = trimmed.strip_prefix("Aggregate:") {
            if !has_domain {
                return Err(fail("an aggregate belongs inside a domain"));
            }
            close_to(&mut open, &mut out, Block::Domain);
            let name = quoted(name.trim(), &fail)?;
            out.push(format!("  aggregate \"{name}\" do"));
            aggregate = name;
            open.push(Block::Aggregate);
            header_open = true;
        } else if trimmed == "It has:" {
            if open.last() != Some(&Block::Aggregate) {
                return Err(fail("\"It has:\" belongs inside an aggregate"));
            }
            open.push(Block::Has);
        } else if let Some(field) = identified_by(trimmed) {
            if open.last() != Some(&Block::Aggregate) {
                return Err(fail("an identity belongs inside an aggregate"));
            }
            out.push(format!("    identified_by :{field}"));
        } else if after_header && trimmed.ends_with('.') {
            let keyword = match open.last() {
                Some(Block::Domain) => "vision",
                Some(Block::Aggregate) => "description",
                _ => return Err(fail("a paragraph belongs under a header")),
            };
            let indent = "  ".repeat(open.len());
            out.push(format!("{indent}{keyword} \"{}\"", quoted(trimmed, &fail)?));
        } else if open.last() == Some(&Block::Has) {
            let (field, ruby, optional) = attribute_parts(trimmed)
                .ok_or_else(|| fail("no attribute phrase matches this line"))?;
            // Hecks types an attribute by a value object, so a primitive gets one of its own.
            let value_name = format!("{aggregate}{}", camel(&field));
            out.push(format!("    value_object \"{value_name}\", {ruby}"));
            let tail = if optional { ", optional: true" } else { "" };
            out.push(format!("    attribute :{field}, {value_name}{tail}"));
        } else {
            return Err(fail("no phrase matches this line"));
        }
    }

    if !has_domain {
        return Err(Diagnostic {
            line: 1,
            column: 1,
            message: "a file must start with \"Domain: <name>\"".to_string(),
        });
    }
    close_to(&mut open, &mut out, Block::Domain);
    out.push("end".to_string());
    Ok(format!("{}\n", out.join("\n")))
}

/// Matches `A <thing> is identified by its <field>.` and returns the field name.
fn identified_by(text: &str) -> Option<String> {
    let rest = text.strip_suffix('.')?;
    let (_, field) = rest.split_once(" is identified by its ")?;
    Some(field.trim().replace(' ', "_"))
}

/// Matches `a <field>: <type>` (or `an`), with an optional `, optional` tail. Returns the field
/// name, the Ruby primitive for its type, and whether it is optional.
fn attribute_parts(text: &str) -> Option<(String, &'static str, bool)> {
    let text = text.strip_suffix('.').unwrap_or(text);
    let (lhs, ty) = text.split_once(':')?;
    let lhs = lhs.trim();
    let field = lhs.strip_prefix("a ").or_else(|| lhs.strip_prefix("an "))?;
    let ty = ty.trim();
    let (ty, optional) = match ty.strip_suffix(", optional") {
        Some(ty) => (ty.trim(), true),
        None => (ty, false),
    };
    let ruby = match ty {
        "text" => "String",
        "whole number" => "Integer",
        _ => return None,
    };
    Some((field.replace(' ', "_"), ruby, optional))
}

/// `some_field` as `SomeField`, for naming a generated value object.
fn camel(field: &str) -> String {
    field
        .split('_')
        .filter(|part| !part.is_empty())
        .map(|part| {
            let mut chars = part.chars();
            chars.next().map_or_else(String::new, |first| {
                first.to_uppercase().chain(chars).collect()
            })
        })
        .collect()
}

/// Refuses a name that would need escaping in a Ruby string literal, rather than escaping it.
fn quoted(text: &str, fail: &dyn Fn(&str) -> Diagnostic) -> Result<String, Diagnostic> {
    if text.contains('"') || text.contains('\\') {
        Err(fail("a name cannot contain quotes or backslashes yet"))
    } else {
        Ok(text.to_string())
    }
}

/// Closes every open block above `keep`, emitting its `end` lines at the block's depth.
fn close_to(open: &mut Vec<Block>, out: &mut Vec<String>, keep: Block) {
    while let Some(&top) = open.last() {
        if top == keep {
            break;
        }
        open.pop();
        // `It has:` adds no `end` of its own: its attributes sit in the aggregate's body.
        if top != Block::Has {
            out.push(format!("{}end", "  ".repeat(open.len())));
        }
    }
}
