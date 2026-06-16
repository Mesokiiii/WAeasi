//! Tiny TOML subset parser — handles tables, key=value (string / int /
//! bool), and arrays of strings.  Comments (`#`) ignored.  Quoting is
//! ASCII double-quotes only.  Sufficient for the manifest schema.

use super::schema::{Capabilities, Manifest, Resources};

#[derive(Debug, PartialEq, Eq)]
pub enum ParseError { Syntax, MissingField, BadValue }

pub fn parse(input: &str) -> Result<Manifest, ParseError> {
    let mut m = Manifest::default();
    let mut section = String::new();
    for raw in input.lines() {
        let line = raw.trim();
        if line.is_empty() || line.starts_with('#') { continue; }
        if line.starts_with('[') && line.ends_with(']') {
            section = line[1..line.len() - 1].to_string();
            continue;
        }
        let (k_raw, v_raw) = line.split_once('=').ok_or(ParseError::Syntax)?;
        let key = k_raw.trim();
        let val = v_raw.trim();
        match section.as_str() {
            "component"    => assign_component(&mut m, key, val)?,
            "capabilities" => assign_caps(&mut m.capabilities, key, val)?,
            "resources"    => assign_resources(&mut m.resources, key, val)?,
            "exports"      => m.exports.push((key.to_string(), strip_str(val)?.to_string())),
            _              => return Err(ParseError::Syntax),
        }
    }
    if m.name.is_empty() || m.version.is_empty() { return Err(ParseError::MissingField); }
    Ok(m)
}

fn assign_component(m: &mut Manifest, k: &str, v: &str) -> Result<(), ParseError> {
    match k {
        "name"    => m.name    = strip_str(v)?.to_string(),
        "version" => m.version = strip_str(v)?.to_string(),
        "digest"  => m.digest  = Some(strip_str(v)?.to_string()),
        _ => return Err(ParseError::Syntax),
    }
    Ok(())
}

fn assign_caps(c: &mut Capabilities, k: &str, v: &str) -> Result<(), ParseError> {
    if k != "rights" { return Err(ParseError::Syntax); }
    c.rights = parse_str_array(v)?;
    Ok(())
}

fn assign_resources(r: &mut Resources, k: &str, v: &str) -> Result<(), ParseError> {
    match k {
        "cpu_shares"        => r.cpu_shares = Some(v.parse().map_err(|_| ParseError::BadValue)?),
        "memory_pages_max"  => r.memory_pages_max = Some(v.parse().map_err(|_| ParseError::BadValue)?),
        "linear_mem_max"    => r.linear_mem_max = Some(strip_str(v)?.to_string()),
        _ => return Err(ParseError::Syntax),
    }
    Ok(())
}

fn strip_str(v: &str) -> Result<&str, ParseError> {
    let v = v.trim();
    if v.len() >= 2 && v.starts_with('"') && v.ends_with('"') {
        Ok(&v[1..v.len() - 1])
    } else {
        Err(ParseError::BadValue)
    }
}

fn parse_str_array(v: &str) -> Result<Vec<String>, ParseError> {
    let v = v.trim();
    if !v.starts_with('[') || !v.ends_with(']') { return Err(ParseError::BadValue); }
    let inner = &v[1..v.len() - 1];
    let mut out = Vec::new();
    for tok in inner.split(',') {
        let t = tok.trim();
        if t.is_empty() { continue; }
        out.push(strip_str(t)?.to_string());
    }
    Ok(out)
}
