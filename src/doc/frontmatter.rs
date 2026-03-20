use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};

/// Parsed YAML frontmatter from a document file.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Frontmatter {
    pub title: Option<String>,
    pub status: Option<String>,
    pub module: Option<String>,
    pub prd: Option<String>,
    pub parent: Option<String>,
    #[serde(rename = "merged-into")]
    pub merged_into: Option<String>,
    #[serde(rename = "superseded-by")]
    pub superseded_by: Option<String>,
    #[serde(rename = "design-doc")]
    pub design_doc: Option<String>,
    #[serde(rename = "exec-plan")]
    pub exec_plan: Option<String>,
    pub guidelines: Option<Vec<String>>,
    pub boundaries: Option<Boundaries>,
    pub completion_criteria: Option<Vec<CompletionCriterion>>,
}

/// Boundary constraints for a Task Spec.
#[derive(Debug, Clone, Serialize, Deserialize, Default)]
pub struct Boundaries {
    pub allowed: Vec<String>,
    pub forbidden_patterns: Option<Vec<String>>,
}

/// A single completion criterion in a Task Spec.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CompletionCriterion {
    pub id: String,
    pub scenario: String,
    pub test: String,
}

impl Frontmatter {
    /// Parse frontmatter from raw file content.
    ///
    /// Expects the file to start with `---`, followed by YAML, followed by `---`.
    pub fn parse(content: &str) -> Result<(Self, String)> {
        let parts: Vec<&str> = content.splitn(3, "---").collect();
        if parts.len() < 3 {
            bail!("missing YAML frontmatter (expected --- delimiters)");
        }
        let yaml = parts[1];
        let body = parts[2].to_string();
        let frontmatter: Frontmatter = serde_yaml::from_str(yaml)
            .context("parsing YAML frontmatter")?;
        Ok((frontmatter, body))
    }

    /// Render frontmatter back to a YAML string suitable for writing to a file.
    pub fn render(&self, body: &str) -> Result<String> {
        let yaml = serde_yaml::to_string(self)?;
        Ok(format!("---\n{}---\n{}", yaml, body))
    }
}
