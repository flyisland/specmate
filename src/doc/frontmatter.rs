use crate::doc::types::{Boundaries, CompletionCriterion, Frontmatter};
use crate::error::DocumentModelError;
use anyhow::Result;
use serde::Deserialize;
use std::path::Path;

#[derive(Debug, Deserialize)]
struct RawFrontmatter {
    id: Option<String>,
    title: Option<String>,
    status: Option<String>,
    created: Option<String>,
    closed: Option<String>,
    module: Option<String>,
    prd: Option<String>,
    parent: Option<String>,
    #[serde(rename = "merged-into")]
    merged_into: Option<String>,
    #[serde(rename = "superseded-by")]
    superseded_by: Option<String>,
    #[serde(rename = "design-doc")]
    design_doc: Option<String>,
    #[serde(rename = "design-docs")]
    design_docs: Option<Vec<String>>,
    #[serde(rename = "exec-plan")]
    exec_plan: Option<String>,
    guidelines: Option<Vec<String>>,
    boundaries: Option<RawBoundaries>,
    completion_criteria: Option<Vec<RawCompletionCriterion>>,
}

#[derive(Debug, Deserialize)]
struct RawBoundaries {
    allowed: Option<Vec<String>>,
    forbidden_patterns: Option<Vec<String>>,
}

#[derive(Debug, Deserialize)]
struct RawCompletionCriterion {
    id: Option<String>,
    scenario: Option<String>,
    test: Option<String>,
}

/// Parses the leading YAML frontmatter block from a markdown document.
pub fn parse_frontmatter(path: &Path, raw: &str) -> Result<Frontmatter> {
    let block = extract_frontmatter_block(path, raw)?;
    let parsed: RawFrontmatter =
        serde_yaml::from_str(&block).map_err(|error| DocumentModelError::InvalidFrontmatter {
            path: path.to_path_buf(),
            message: error.to_string(),
        })?;

    let boundaries = parsed.boundaries.map(|boundaries| Boundaries {
        allowed: boundaries.allowed.unwrap_or_default(),
        forbidden_patterns: boundaries.forbidden_patterns.unwrap_or_default(),
    });

    let completion_criteria = parsed
        .completion_criteria
        .unwrap_or_default()
        .into_iter()
        .map(|criterion| CompletionCriterion {
            id: criterion.id.unwrap_or_default(),
            scenario: criterion.scenario.unwrap_or_default(),
            test: criterion.test.unwrap_or_default(),
        })
        .collect();

    Ok(Frontmatter {
        id: parsed.id,
        title: parsed.title,
        status: parsed.status,
        created: parsed.created,
        closed: parsed.closed,
        module: parsed.module,
        prd: parsed.prd,
        parent: parsed.parent,
        merged_into: parsed.merged_into,
        superseded_by: parsed.superseded_by,
        design_doc: parsed.design_doc,
        design_docs: parsed.design_docs.unwrap_or_default(),
        exec_plan: parsed.exec_plan,
        guidelines: parsed.guidelines.unwrap_or_default(),
        boundaries,
        completion_criteria,
    })
}

fn extract_frontmatter_block(path: &Path, raw: &str) -> Result<String> {
    let mut lines = raw.lines();
    let first = lines
        .next()
        .map(|line| line.trim_end_matches('\r'))
        .ok_or_else(|| DocumentModelError::MissingFrontmatter {
            path: path.to_path_buf(),
        })?;
    if first != "---" {
        return Err(DocumentModelError::MissingFrontmatter {
            path: path.to_path_buf(),
        }
        .into());
    }

    let mut block = String::new();
    let mut found_end = false;
    for line in lines {
        let trimmed = line.trim_end_matches('\r');
        if trimmed == "---" {
            found_end = true;
            break;
        }
        if !block.is_empty() {
            block.push('\n');
        }
        block.push_str(trimmed);
    }

    if !found_end {
        return Err(DocumentModelError::MissingFrontmatter {
            path: path.to_path_buf(),
        }
        .into());
    }

    Ok(block)
}
