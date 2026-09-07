//! Skill-conformance tests (skills/): the SKILL.md files are the machine
//! contract for code agents taking over the teaching loop or the compiler.
//! They must declare a proper frontmatter, only reference repo paths that
//! exist, and only advertise event/method names that the real tooling
//! actually emits/answers — the same anti-drift policy as docs/protocol.md.

use std::fs;
use std::path::Path;

use common::{repo_root, EVENT_VOCABULARY, LSP_CUSTOM_METHODS, WATCH_VOCABULARY};

mod common;

struct Skill {
    /// Directory name under skills/.
    dir: String,
    /// Raw SKILL.md content.
    body: String,
    /// Frontmatter block (between the leading --- fences).
    frontmatter: String,
}

fn load_skills() -> Vec<Skill> {
    let root = repo_root();
    let mut skills = Vec::new();
    let entries = fs::read_dir(root.join("skills")).expect("skills/ directory exists");
    for entry in entries {
        let entry = entry.expect("skills entry readable");
        let path = entry.path();
        if !path.is_dir() {
            continue;
        }
        let skill_md = path.join("SKILL.md");
        let body = fs::read_to_string(&skill_md)
            .unwrap_or_else(|e| panic!("{} readable: {e}", skill_md.display()));
        let frontmatter = body
            .strip_prefix("---\n")
            .and_then(|rest| rest.split("---\n").next().map(|s| s.to_string()))
            .expect("SKILL.md starts with a --- frontmatter block");
        skills.push(Skill {
            dir: path
                .file_name()
                .expect("dir name")
                .to_string_lossy()
                .into_owned(),
            body,
            frontmatter,
        });
    }
    assert!(
        !skills.is_empty(),
        "at least one skill must exist under skills/"
    );
    skills
}

fn frontmatter_value(frontmatter: &str, key: &str) -> Option<String> {
    frontmatter.lines().find_map(|line| {
        let value = line.strip_prefix(&format!("{key}: "))?;
        Some(value.trim().to_string())
    })
}

/// Path-like tokens the skills may point at; each must exist in the repo.
const PATH_PREFIXES: [&str; 5] = ["docs/", "course/", "crates/", "examples/", "skills/"];

fn referenced_paths(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let chars: Vec<char> = text.chars().collect();
    let mut i = 0;
    while i < chars.len() {
        let rest: String = chars[i..].iter().collect();
        let Some(prefix) = PATH_PREFIXES
            .iter()
            .find(|p| rest.starts_with(**p))
            .map(|p| p.to_string())
        else {
            i += 1;
            continue;
        };
        let mut j = prefix.len();
        while i + j < chars.len()
            && (chars[i + j].is_ascii_alphanumeric() || "_-./".contains(chars[i + j]))
        {
            j += 1;
        }
        let mut token: String = chars[i..i + j].iter().collect();
        // A trailing dot is prose punctuation, not part of the path.
        while token.ends_with('.') {
            token.pop();
        }
        out.push(token);
        i += j;
    }
    out
}

#[test]
fn skills_declare_name_and_description_frontmatter() {
    for skill in load_skills() {
        let name = frontmatter_value(&skill.frontmatter, "name")
            .unwrap_or_else(|| panic!("{}: frontmatter declares name", skill.dir));
        assert_eq!(
            name, skill.dir,
            "frontmatter name must match the directory name"
        );
        let description = frontmatter_value(&skill.frontmatter, "description")
            .unwrap_or_else(|| panic!("{}: frontmatter declares description", skill.dir));
        assert!(
            description.len() >= 20 && description.len() <= 1024,
            "{}: description must be 20..=1024 chars, got {}",
            skill.dir,
            description.len()
        );
    }
}

#[test]
fn skill_referenced_repo_paths_exist() {
    let root = repo_root();
    let mut checked = 0usize;
    for skill in load_skills() {
        for path in referenced_paths(&skill.body) {
            let target = root.join(&path);
            assert!(
                target.exists(),
                "{} references `{}` which does not exist in the repo",
                skill.dir,
                path
            );
            checked += 1;
        }
    }
    assert!(
        checked >= 10,
        "skills should anchor to real repo docs, checked {checked}"
    );
}

#[test]
fn skill_event_and_method_vocabulary_is_closed() {
    let protocol = fs::read_to_string(repo_root().join("docs/protocol.md"))
        .expect("docs/protocol.md readable");
    let vocabulary: Vec<&str> = EVENT_VOCABULARY
        .iter()
        .chain(WATCH_VOCABULARY.iter())
        .chain(LSP_CUSTOM_METHODS.iter())
        .copied()
        .collect();
    for skill in load_skills() {
        for word in referenced_tokens(&skill.body) {
            let known = vocabulary.contains(&word.as_str());
            if known {
                // Advertised names must be part of the real contract.
                assert!(
                    protocol.contains(word.as_str()),
                    "{}: `{word}` must be documented in docs/protocol.md",
                    skill.dir
                );
            } else if word.starts_with("decl.")
                || word.starts_with("exercise.")
                || word.starts_with("soko/")
                || word.starts_with("file.")
            {
                panic!(
                    "{} advertises `{word}` which is not part of the protocol contract",
                    skill.dir
                );
            }
        }
    }
}

/// Backticked token scan: any `…` span that looks like an event/method name.
fn referenced_tokens(text: &str) -> Vec<String> {
    let mut out = Vec::new();
    let mut rest = text;
    while let Some(start) = rest.find('`') {
        let after = &rest[start + 1..];
        let Some(end) = after.find('`') else { break };
        let token = &after[..end];
        let t = token.trim();
        if t.split_whitespace().count() == 1 && (t.contains('.') || t.contains('/')) {
            out.push(t.to_string());
        }
        rest = &after[end + 1..];
    }
    out
}

#[test]
fn teacher_skill_anchors_to_canvas_course_and_solutions() {
    let root = repo_root();
    let body = fs::read_to_string(Path::new(&root).join("skills/sokonanoda-teacher/SKILL.md"))
        .expect("teacher skill readable");
    for anchor in [
        "playground.sokonanoda",
        "course/course.json",
        "docs/teaching-session.md",
        "--json",
    ] {
        assert!(
            body.contains(anchor),
            "teacher skill must mention `{anchor}`"
        );
    }
    // Every course unit listed by course.json must exist as a canvas + twin.
    let course: serde_json::Value = serde_json::from_str(
        &fs::read_to_string(root.join("course/course.json")).expect("course.json readable"),
    )
    .expect("course.json is valid JSON");
    for unit in course.as_array().expect("course.json holds an array") {
        let file = unit["file"].as_str().expect("unit file");
        assert!(
            root.join("course").join(file).exists(),
            "canvas {file} exists"
        );
        let solution = format!(
            "solutions/{}-solution.sokonanoda",
            file.trim_end_matches(".sokonanoda")
        );
        assert!(
            root.join("course").join(&solution).exists(),
            "solution twin {solution} exists"
        );
    }
}
