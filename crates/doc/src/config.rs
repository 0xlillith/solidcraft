//! Configurations: a table whose rows are variants of the design. Each column is a parameter
//! expression, feature suppression, body material or body appearance. Activating a row sets
//! those values in the design atomically, which then rebuilds as usual.

use serde::{Deserialize, Serialize};

use crate::{DocError, Document, Result};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Column {
    /// A parameter's expression.
    Param(String),
    /// A feature's suppression (by feature id).
    Suppress(u64),
    /// A body's physical material (library name, or Default to clear).
    Material(String),
    /// A body's appearance (library name, serialized Look, or empty to clear).
    Appearance(String),
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct ConfigRow {
    pub name: String,
    /// One cell per column: expression, suppression, material name or appearance look.
    pub cells: Vec<String>,
}

#[derive(Clone, Debug, Default, PartialEq, Serialize, Deserialize)]
pub struct ConfigTable {
    pub columns: Vec<Column>,
    pub rows: Vec<ConfigRow>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub active: Option<String>,
}

pub const MAX_COLUMNS: usize = 500;
pub const MAX_ROWS: usize = 2000;

impl ConfigTable {
    pub fn is_empty(&self) -> bool {
        self.columns.is_empty() && self.rows.is_empty()
    }
}

/// Parse an appearance cell. Empty/Default follows the body's material; named library looks
/// and JSON Look objects support both convenient editing and lossless custom colours.
pub fn appearance_cell(value: &str) -> Result<Option<crate::appearance::Look>> {
    let value = value.trim();
    if value.is_empty() || value.eq_ignore_ascii_case("default") {
        return Ok(None);
    }
    if value.len() > 4096 {
        return Err(DocError::Invalid("appearance value too long".into()));
    }
    if let Some(look) = crate::appearance::library(value) {
        return Ok(Some(look));
    }
    let look: crate::appearance::Look =
        serde_json::from_str(value).map_err(|_| DocError::Invalid(format!("unknown configuration appearance `{value}`")))?;
    if look.name.chars().count() > 128 || !look.opacity.is_finite() || !(0.0..=1.0).contains(&look.opacity) {
        return Err(DocError::Invalid("appearance requires a name up to 128 characters and opacity in 0..1".into()));
    }
    Ok(Some(look))
}

/// Parse a physical material cell, returning None for the default material.
pub fn material_cell(value: &str) -> Result<Option<String>> {
    let name = crate::MATERIALS
        .iter()
        .find(|(n, _)| n.eq_ignore_ascii_case(value.trim()))
        .map(|(n, _)| *n)
        .ok_or_else(|| DocError::Invalid(format!("unknown configuration material `{value}`")))?;
    Ok((name != "Default").then(|| name.to_string()))
}

impl Document {
    /// A column's current value in the design.
    pub fn config_current(&self, c: &Column) -> Option<String> {
        match c {
            Column::Param(n) => self.all_param_exprs().into_iter().find(|(m, _, _)| m == n).map(|(_, e, _)| e),
            Column::Suppress(id) => self.feature(*id).map(|f| f.suppressed.to_string()),
            Column::Material(body) => Some(self.materials.get(body).cloned().unwrap_or_else(|| "Default".into())),
            Column::Appearance(body) => Some(self.appearances.bodies.get(body).and_then(|look| serde_json::to_string(look).ok()).unwrap_or_default()),
        }
    }

    /// A column's display key, prefixed by its kind to distinguish targets with equal names.
    pub fn config_key(&self, c: &Column) -> String {
        match c {
            Column::Param(n) => format!("param:{n}"),
            Column::Suppress(id) => format!("suppress:{}", self.feature(*id).map(|f| f.name.clone()).unwrap_or_else(|| id.to_string())),
            Column::Material(body) => format!("material:{body}"),
            Column::Appearance(body) => format!("appearance:{body}"),
        }
    }

    /// Set the design to a row's values (all or nothing).
    pub fn apply_configuration(&mut self, row: &str) -> Result<()> {
        let r = self.configs.rows.iter().find(|r| r.name == row).cloned().ok_or_else(|| DocError::Unknown(format!("configuration `{row}`")))?;
        if self.configs.columns.len() > MAX_COLUMNS || self.configs.rows.len() > MAX_ROWS || r.cells.len() != self.configs.columns.len() {
            return Err(DocError::Invalid("configuration table exceeds limits or has incomplete rows".into()));
        }
        let mut next = self.clone();
        for (c, v) in self.configs.columns.iter().zip(&r.cells) {
            if v.len() > 4096 {
                return Err(DocError::Invalid("configuration value too long".into()));
            }
            match c {
                Column::Param(n) => {
                    if next.config_current(c).as_deref() != Some(v.as_str()) {
                        next.change_param(n, v, None, None).map_err(|e| DocError::Invalid(format!("configuration `{row}`, parameter `{n}`: {e}")))?;
                    }
                }
                Column::Suppress(id) => {
                    let on = match v.trim() {
                        "true" | "1" | "yes" => true,
                        "false" | "0" | "no" => false,
                        o => return Err(DocError::Invalid(format!("configuration `{row}`: suppression must be true or false, not `{o}`"))),
                    };
                    let f = next.feature_mut(*id).ok_or_else(|| DocError::Unknown(format!("configuration `{row}`: feature {id} is gone")))?;
                    f.suppressed = on;
                }
                Column::Material(body) => match material_cell(v)? {
                    Some(material) => {
                        next.materials.insert(body.clone(), material);
                    }
                    None => {
                        next.materials.remove(body);
                    }
                },
                Column::Appearance(body) => match appearance_cell(v)? {
                    Some(look) => {
                        next.appearances.bodies.insert(body.clone(), look);
                    }
                    None => {
                        next.appearances.bodies.remove(body);
                    }
                },
            }
        }
        next.configs.active = Some(row.to_string());
        *self = next;
        Ok(())
    }

    /// The active row's values all hold in the design.
    pub fn configuration_matches(&self) -> bool {
        let Some(r) = self.configs.active.as_ref().and_then(|a| self.configs.rows.iter().find(|r| r.name == *a)) else { return false };
        if r.cells.len() != self.configs.columns.len() {
            return false;
        }
        self.configs.columns.iter().zip(&r.cells).all(|(c, v)| match c {
            Column::Suppress(_) => self.config_current(c).as_deref() == Some(v.trim()),
            Column::Param(_) => self.config_current(c).is_some_and(|x| x.replace(' ', "") == v.replace(' ', "")),
            Column::Material(body) => material_cell(v).is_ok_and(|m| self.materials.get(body) == m.as_ref()),
            Column::Appearance(body) => appearance_cell(v).is_ok_and(|look| self.appearances.bodies.get(body) == look.as_ref()),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rows_switch_parameters() {
        let mut d = Document::new("A");
        d.set_param("w", "10 mm", None, None).unwrap();
        d.configs.columns.push(Column::Param("w".into()));
        d.configs.rows.push(ConfigRow { name: "Small".into(), cells: vec!["10 mm".into()] });
        d.configs.rows.push(ConfigRow { name: "Large".into(), cells: vec!["40 mm".into()] });
        d.apply_configuration("Large").unwrap();
        assert_eq!(d.config_current(&Column::Param("w".into())).as_deref(), Some("40 mm"));
        assert!(d.configuration_matches());
        d.set_param("w", "12 mm", None, None).unwrap();
        assert!(!d.configuration_matches());
        d.configs.rows.push(ConfigRow { name: "Bad".into(), cells: vec!["1/".into()] });
        let before = d.clone();
        assert!(d.apply_configuration("Bad").is_err());
        assert_eq!(d, before, "all or nothing");
    }

    #[test]
    fn invalid_saved_appearance_rolls_back_material_and_parameters() {
        let mut d = Document::new("A");
        d.set_param("width", "10 mm", None, None).unwrap();
        d.configs.columns = vec![Column::Param("width".into()), Column::Material("Body".into()), Column::Appearance("Body".into())];
        d.configs.rows.push(ConfigRow {
            name: "Broken".into(),
            cells: vec!["20 mm".into(), "Steel".into(), r#"{"name":"Invalid","color":[1,2,3],"opacity":2}"#.into()],
        });
        let before = d.clone();
        assert!(d.apply_configuration("Broken").is_err());
        assert_eq!(d, before);
        assert!(appearance_cell(r#"{"name":"Invalid","color":[1,2,3],"opacity":1e999}"#).is_err());
        assert!(appearance_cell(&"a".repeat(4097)).is_err());
        assert!(appearance_cell("default").unwrap().is_none());
    }
}
