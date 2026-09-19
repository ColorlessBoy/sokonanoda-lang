//! Course **manifest v2** (`soko.course/2`) parsing, with the v1 flat array
//! still accepted (ledger G-07; design `docs/design/course-manifest-v2.md`).
//!
//! Two shapes, one flattened result:
//!
//! * **v1** — a JSON array of `{file, title, title_en, unit}` entries
//!   (`course/course.json`, `scripts/new-course-repo.sh` skeletons): no
//!   volume/chapter metadata at all;
//! * **v2** — a JSON object `{schema, name, title, volumes[].chapters[].units[]}`
//!   where `units[]` keeps the v1 entry shape **verbatim**, so flattening a v2
//!   manifest yields exactly the v1 unit list plus its volume/chapter context.
//!
//! Backward compatibility is a hard rule: every v1 field and event stays, v2
//! only *adds* (`course.unit.volume` / `.chapter` / `.tags`,
//! `course.summary.volumes` / `.chapters` — docs/protocol.md).
//!
//! Refusal rule: a JSON object whose `schema` is present and **not**
//! `soko.course/2` is an error, never a guess. An object without `schema` is
//! read as v2 (the shape is unambiguous: arrays are v1, objects are v2).

use serde_json::Value;

pub(crate) const SCHEMA_V2: &str = "soko.course/2";

/// A volume reference as carried by `course.unit.volume`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct VolumeRef {
    pub id: String,
    pub title: String,
}

/// A chapter reference as carried by `course.unit.chapter`.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ChapterRef {
    pub id: String,
    pub title: String,
    pub tags: Vec<String>,
}

/// One flattened unit entry: the v1 fields plus optional v2 context.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct UnitEntry {
    pub file: String,
    pub title: String,
    pub unit: u64,
    pub volume: Option<VolumeRef>,
    pub chapter: Option<ChapterRef>,
}

/// The flattened manifest: a unit list (document order) + structure counts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct Manifest {
    pub units: Vec<UnitEntry>,
    /// Number of `volumes[]` entries (0 for a v1 array).
    pub volumes: usize,
    /// Number of `chapters[]` entries across all volumes (0 for a v1 array).
    pub chapters: usize,
}

/// Parse a manifest source into the flattened [`Manifest`].
///
/// `Err` carries a human sentence for the CLI's stderr (the caller prints it
/// with the manifest path it was given, not the canonicalized one).
pub(crate) fn parse(raw: &str) -> Result<Manifest, String> {
    let value: Value = serde_json::from_str(raw).map_err(|e| format!("not valid JSON: {e}"))?;
    match &value {
        Value::Array(entries) => Ok(flatten_v1(entries)),
        Value::Object(_) => flatten_v2(&value),
        _ => Err("JSON array or `soko.course/2` object expected".to_string()),
    }
}

/// v1: a flat array; every entry is a unit and there is no structure to count.
fn flatten_v1(entries: &[Value]) -> Manifest {
    Manifest {
        units: entries
            .iter()
            .map(|entry| unit_entry(entry, None, None))
            .collect(),
        volumes: 0,
        chapters: 0,
    }
}

/// v2: `volumes[].chapters[].units[]`, flattened in document order.
fn flatten_v2(value: &Value) -> Result<Manifest, String> {
    if let Some(schema) = value.get("schema").and_then(Value::as_str) {
        if schema != SCHEMA_V2 {
            return Err(format!(
                "unknown course manifest schema {schema:?} (expected {SCHEMA_V2:?} or a v1 flat array)"
            ));
        }
    }
    let volumes = match value.get("volumes") {
        Some(Value::Array(volumes)) => volumes,
        // A `schema`-less object without volumes is still a v2-shaped manifest
        // with nothing in it; the CLI reports an empty course rather than a
        // parse error (progress is not an error).
        None => &Vec::new(),
        Some(_) => return Err("`volumes` must be an array".to_string()),
    };

    let mut manifest = Manifest {
        volumes: volumes.len(),
        ..Manifest::default()
    };
    for volume in volumes {
        let volume_ref = VolumeRef {
            id: string_field(volume, "id"),
            title: string_field(volume, "title"),
        };
        let chapters = match volume.get("chapters") {
            Some(Value::Array(chapters)) => chapters,
            None => &Vec::new(),
            Some(_) => return Err("`chapters` must be an array".to_string()),
        };
        manifest.chapters += chapters.len();
        for chapter in chapters {
            let chapter_ref = ChapterRef {
                id: string_field(chapter, "id"),
                title: string_field(chapter, "title"),
                tags: string_list(chapter.get("tags")),
            };
            let units = match chapter.get("units") {
                Some(Value::Array(units)) => units,
                None => &Vec::new(),
                Some(_) => return Err("`units` must be an array".to_string()),
            };
            for unit in units {
                manifest.units.push(unit_entry(
                    unit,
                    Some(volume_ref.clone()),
                    Some(chapter_ref.clone()),
                ));
            }
        }
    }
    Ok(manifest)
}

/// One unit entry. v2 keeps the v1 shape verbatim, so the same reader serves
/// both — the v2 context is passed in by the caller.
fn unit_entry(entry: &Value, volume: Option<VolumeRef>, chapter: Option<ChapterRef>) -> UnitEntry {
    UnitEntry {
        file: string_field(entry, "file"),
        title: entry
            .get("title")
            .and_then(Value::as_str)
            .unwrap_or("（无标题）")
            .to_string(),
        unit: entry.get("unit").and_then(Value::as_u64).unwrap_or(0),
        volume,
        chapter,
    }
}

fn string_field(value: &Value, field: &str) -> String {
    value
        .get(field)
        .and_then(Value::as_str)
        .unwrap_or_default()
        .to_string()
}

fn string_list(value: Option<&Value>) -> Vec<String> {
    value
        .and_then(Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_owned)
                .collect()
        })
        .unwrap_or_default()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_array_flattens_without_structure() {
        let manifest = parse(
            r#"[{"file":"a.sokonanoda","title":"A","unit":1},
                {"file":"b.sokonanoda","unit":2}]"#,
        )
        .expect("v1 parses");
        assert_eq!(manifest.volumes, 0);
        assert_eq!(manifest.chapters, 0);
        assert_eq!(manifest.units.len(), 2);
        assert_eq!(manifest.units[0].title, "A");
        assert_eq!(manifest.units[1].title, "（无标题）", "v1 default title");
        assert!(manifest.units[0].volume.is_none());
        assert!(manifest.units[0].chapter.is_none());
    }

    #[test]
    fn v2_nests_are_flattened_in_document_order() {
        let manifest = parse(
            r#"{"schema":"soko.course/2","volumes":[
                 {"id":"I","title":"One","chapters":[
                   {"id":"I.1","title":"First","prereqs":[],"tags":["t1"],
                    "units":[{"file":"a","unit":1},{"file":"b","unit":2}]},
                   {"id":"I.2","title":"Second","units":[{"file":"c","unit":3}]}]},
                 {"id":"II","title":"Two","chapters":[]}]}"#,
        )
        .expect("v2 parses");
        assert_eq!(manifest.volumes, 2);
        assert_eq!(manifest.chapters, 2, "empty volumes still count");
        assert_eq!(
            manifest
                .units
                .iter()
                .map(|u| u.file.as_str())
                .collect::<Vec<_>>(),
            vec!["a", "b", "c"]
        );
        assert_eq!(
            manifest.units[2].volume.as_ref().map(|v| v.id.as_str()),
            Some("I")
        );
        assert_eq!(
            manifest.units[2].chapter.as_ref().map(|c| c.id.as_str()),
            Some("I.2")
        );
        assert_eq!(
            manifest.units[0].chapter.as_ref().map(|c| c.tags.clone()),
            Some(vec!["t1".to_string()])
        );
    }

    #[test]
    fn unknown_schema_is_refused_and_schema_less_object_is_v2() {
        let err = parse(r#"{"schema":"soko.course/3","volumes":[]}"#).expect_err("refused");
        assert!(err.contains("soko.course/3"), "the message names it: {err}");
        let ok = parse(r#"{"volumes":[{"id":"I","chapters":[]}]}"#).expect("schema-less v2");
        assert_eq!(ok.volumes, 1);
        assert!(parse("42").is_err(), "a bare number is not a manifest");
        assert!(parse("not json").is_err());
    }
}
