//! Check engine for validating the managed document system.

mod git;

use crate::doc::{build_index, validate_index, DocType, Document, DocumentIndex, Status};
use anyhow::{anyhow, bail, Context, Result};
use glob::Pattern;
use std::collections::BTreeSet;
use std::path::Path;

/// A named check available under `specmate check`.
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum CheckName {
    /// Validate managed document filenames and managed markdown locations.
    Names,
    /// Validate frontmatter fields and frontmatter-level repository rules.
    Frontmatter,
    /// Validate directory placement against status.
    Status,
    /// Validate cross-document references.
    Refs,
    /// Validate boundary overlap across tasks.
    Conflicts,
}

impl CheckName {
    /// Returns the stable CLI spelling for this check.
    pub fn as_str(self) -> &'static str {
        match self {
            CheckName::Names => "names",
            CheckName::Frontmatter => "frontmatter",
            CheckName::Status => "status",
            CheckName::Refs => "refs",
            CheckName::Conflicts => "conflicts",
        }
    }

    fn label(self) -> String {
        format!("check {}", self.as_str())
    }
}

/// A concrete violation produced by a check.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CheckViolation {
    /// Repository-relative path associated with the violation.
    pub path: String,
    /// Human-readable rule explanation.
    pub message: String,
    /// Concrete next step to fix the violation.
    pub fix: String,
}

/// The pass/fail result for a single named check.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CheckReport {
    /// Stable label such as `check names`.
    pub label: String,
    /// Human-readable pass summary.
    pub pass_summary: String,
    /// Violations reported by this check.
    pub violations: Vec<CheckViolation>,
}

impl CheckReport {
    fn pass(name: CheckName, pass_summary: String) -> Self {
        Self {
            label: name.label(),
            pass_summary,
            violations: Vec::new(),
        }
    }

    fn fail(name: CheckName, pass_summary: String, violations: Vec<CheckViolation>) -> Self {
        Self {
            label: name.label(),
            pass_summary,
            violations,
        }
    }

    /// Returns whether the check passed.
    pub fn passed(&self) -> bool {
        self.violations.is_empty()
    }
}

/// Runs the aggregate check suite used by `specmate check`.
pub fn run_all(repo_root: &Path) -> Result<Vec<CheckReport>> {
    let index = build_index(repo_root).context("building document index for checks")?;
    Ok(vec![
        run_named_with_index(&index, CheckName::Names),
        run_named_with_index(&index, CheckName::Frontmatter),
        run_named_with_index(&index, CheckName::Status),
        run_named_with_index(&index, CheckName::Refs),
        run_named_with_index(&index, CheckName::Conflicts),
    ])
}

/// Runs one named check.
pub fn run_named(repo_root: &Path, name: CheckName) -> Result<CheckReport> {
    let index = build_index(repo_root).context("building document index for checks")?;
    Ok(run_named_with_index(&index, name))
}

/// Runs `check boundaries <task-id>`.
pub fn run_boundaries(repo_root: &Path, task_id: &str) -> Result<CheckReport> {
    let index = build_index(repo_root).context("building document index for checks")?;
    let document = find_task_spec(&index, task_id)?;

    if document.doc_type != DocType::TaskSpec {
        bail!("{task_id} is not a Task Spec");
    }
    if document.status != Status::Candidate {
        bail!("{task_id} is not candidate");
    }

    let boundaries = document
        .frontmatter
        .boundaries
        .as_ref()
        .ok_or_else(|| anyhow!("{task_id} is missing boundaries"))?;
    let changed_paths = git::changed_paths(repo_root)?;

    let mut violations = Vec::new();
    let allowed_patterns = compile_patterns(
        repo_root,
        task_id,
        "boundaries.allowed",
        &boundaries.allowed,
    )?;
    let forbidden_patterns = compile_patterns(
        repo_root,
        task_id,
        "boundaries.forbidden_patterns",
        &boundaries.forbidden_patterns,
    )?;

    for path in &changed_paths {
        let path_str = path_to_unix(path);
        if forbidden_patterns
            .iter()
            .any(|pattern| pattern.pattern.matches(&path_str))
        {
            violations.push(CheckViolation {
                path: path_str.clone(),
                message: format!("matches forbidden pattern(s) for {task_id}"),
                fix: format!(
                    "Remove the change from the forbidden path or update {} in the task spec.",
                    "boundaries.forbidden_patterns"
                ),
            });
            continue;
        }
        if !allowed_patterns
            .iter()
            .any(|pattern| pattern.pattern.matches(&path_str))
        {
            violations.push(CheckViolation {
                path: path_str.clone(),
                message: format!("is not in boundaries.allowed for {task_id}"),
                fix: format!(
                    "Keep changes within the task scope. Allowed: {}",
                    boundaries.allowed.join(", ")
                ),
            });
        }
    }

    let pass_summary = if changed_paths.is_empty() {
        format!("no changed files to validate for {task_id}")
    } else {
        format!(
            "all {} changed file{} respect task boundaries",
            changed_paths.len(),
            plural(changed_paths.len())
        )
    };

    Ok(CheckReport {
        label: format!("check boundaries {task_id}"),
        pass_summary,
        violations,
    })
}

/// Renders reports using the CLI conventions expected by `specmate check`.
pub fn render_reports(reports: &[CheckReport]) -> String {
    let mut output = String::new();
    for report in reports {
        if report.passed() {
            output.push_str(&format!(
                "[pass] {:<20} {}\n",
                report.label, report.pass_summary
            ));
            continue;
        }

        output.push_str(&format!(
            "[fail] {:<20} {} violation{}\n",
            report.label,
            report.violations.len(),
            plural(report.violations.len())
        ));
        for violation in &report.violations {
            output.push_str(&format!("       {}\n", violation.path));
            output.push_str(&format!("       {}\n", violation.message));
            output.push_str(&format!("       -> {}\n", violation.fix));
        }
    }

    let failed = reports.iter().filter(|report| !report.passed()).count();
    if failed > 0 {
        output.push('\n');
        output.push_str(&format!(
            "{} check{} failed. Fix violations before running specmate run.\n",
            failed,
            plural(failed)
        ));
    }

    output
}

fn run_named_with_index(index: &DocumentIndex, name: CheckName) -> CheckReport {
    match name {
        CheckName::Names => names_report(index),
        CheckName::Frontmatter => frontmatter_report(index),
        CheckName::Status => status_report(index),
        CheckName::Refs => refs_report(index),
        CheckName::Conflicts => conflicts_report(index),
    }
}

fn names_report(index: &DocumentIndex) -> CheckReport {
    let violations = index
        .invalid_entries
        .iter()
        .filter(|entry| invalid_entry_kind(&entry.reason) == InvalidKind::Names)
        .map(|entry| CheckViolation {
            path: make_relative(index, &entry.path),
            message: entry.reason.clone(),
            fix: "Rename the file to match the managed naming convention for this directory."
                .to_string(),
        })
        .collect::<Vec<_>>();

    build_report(
        CheckName::Names,
        format!("all {} documents pass", index.documents.len()),
        violations,
    )
}

fn frontmatter_report(index: &DocumentIndex) -> CheckReport {
    let mut violations = index
        .invalid_entries
        .iter()
        .filter(|entry| invalid_entry_kind(&entry.reason) == InvalidKind::Frontmatter)
        .map(|entry| CheckViolation {
            path: make_relative(index, &entry.path),
            message: entry.reason.clone(),
            fix: "Repair the frontmatter fields so the document satisfies its document type contract."
                .to_string(),
        })
        .collect::<Vec<_>>();

    violations.extend(
        validate_index(index)
            .into_iter()
            .filter(|violation| is_frontmatter_violation(&violation.message))
            .map(|violation| CheckViolation {
                path: make_relative(index, &violation.path),
                message: violation.message,
                fix: "Fix the referenced frontmatter field so the repository-level rule passes."
                    .to_string(),
            }),
    );

    build_report(
        CheckName::Frontmatter,
        format!("all {} documents pass", index.documents.len()),
        violations,
    )
}

fn status_report(index: &DocumentIndex) -> CheckReport {
    let violations = index
        .invalid_entries
        .iter()
        .filter(|entry| invalid_entry_kind(&entry.reason) == InvalidKind::Status)
        .map(|entry| CheckViolation {
            path: make_relative(index, &entry.path),
            message: status_message(&entry.reason),
            fix: status_fix(&entry.reason),
        })
        .collect::<Vec<_>>();

    build_report(
        CheckName::Status,
        format!("all {} documents pass", index.documents.len()),
        violations,
    )
}

fn refs_report(index: &DocumentIndex) -> CheckReport {
    let violations = validate_index(index)
        .into_iter()
        .filter(|violation| is_reference_violation(&violation.message))
        .map(|violation| CheckViolation {
            path: make_relative(index, &violation.path),
            message: violation.message,
            fix: "Update the reference so it points to an existing document whose status is valid for the current source document."
                .to_string(),
        })
        .collect::<Vec<_>>();

    build_report(
        CheckName::Refs,
        "all references valid".to_string(),
        violations,
    )
}

fn conflicts_report(index: &DocumentIndex) -> CheckReport {
    let mut violations = Vec::new();
    let candidates = index
        .documents
        .values()
        .filter(|document| document.doc_type == DocType::TaskSpec)
        .filter(|document| document.status == Status::Candidate)
        .collect::<Vec<_>>();
    for (left_index, left) in candidates.iter().enumerate() {
        let left_allowed = match left.frontmatter.boundaries.as_ref() {
            Some(boundaries) => &boundaries.allowed,
            None => continue,
        };
        for right in candidates.iter().skip(left_index + 1) {
            let right_allowed = match right.frontmatter.boundaries.as_ref() {
                Some(boundaries) => &boundaries.allowed,
                None => continue,
            };

            for left_pattern in left_allowed {
                for right_pattern in right_allowed {
                    if patterns_overlap(left_pattern, right_pattern) {
                        violations.push(CheckViolation {
                            path: format!("{} <-> {}", left.id, right.id),
                            message: format!("'{}' overlaps '{}'", left_pattern, right_pattern),
                            fix: "Resolve by serialising the tasks or splitting the boundary patterns."
                                .to_string(),
                        });
                    }
                }
            }
        }
    }

    build_report(
        CheckName::Conflicts,
        "no boundary conflicts".to_string(),
        dedup_violations(violations),
    )
}

fn build_report(
    name: CheckName,
    pass_summary: String,
    violations: Vec<CheckViolation>,
) -> CheckReport {
    if violations.is_empty() {
        CheckReport::pass(name, pass_summary)
    } else {
        CheckReport::fail(name, pass_summary, violations)
    }
}

fn patterns_overlap(left: &str, right: &str) -> bool {
    let left_pattern = parse_path_pattern(left);
    let right_pattern = parse_path_pattern(right);
    match (left_pattern, right_pattern) {
        (Some(left_pattern), Some(right_pattern)) => {
            path_patterns_intersect(&left_pattern, &right_pattern)
        }
        _ => false,
    }
}

fn dedup_violations(violations: Vec<CheckViolation>) -> Vec<CheckViolation> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for violation in violations {
        let key = (
            violation.path.clone(),
            violation.message.clone(),
            violation.fix.clone(),
        );
        if seen.insert(key) {
            deduped.push(violation);
        }
    }
    deduped
}

fn invalid_entry_kind(reason: &str) -> InvalidKind {
    if reason.starts_with("invalid filename")
        || reason.starts_with("invalid managed document path")
        || reason.contains("unsupported markdown location")
    {
        InvalidKind::Names
    } else if reason.contains("expected directory ") {
        InvalidKind::Status
    } else {
        InvalidKind::Frontmatter
    }
}

fn is_frontmatter_violation(message: &str) -> bool {
    !is_reference_violation(message)
}

fn is_reference_violation(message: &str) -> bool {
    message.starts_with("prd ")
        || message.starts_with("parent ")
        || message.starts_with("merged-into ")
        || message.starts_with("superseded-by ")
        || message.starts_with("exec-plan ")
        || message.starts_with("design-docs ")
        || message.starts_with("design patch ")
}

fn status_message(reason: &str) -> String {
    if let Some((expected, actual)) = parse_expected_actual_directory(reason) {
        return format!("status directory mismatch: expected {expected}, found {actual}");
    }
    reason.to_string()
}

fn status_fix(reason: &str) -> String {
    if let Some((expected, _)) = parse_expected_actual_directory(reason) {
        return format!(
            "Move the file into {} or update its status field.",
            expected
        );
    }
    "Move the file into the directory that matches its status.".to_string()
}

fn parse_expected_actual_directory(reason: &str) -> Option<(&str, &str)> {
    let marker = "expected directory ";
    let start = reason.find(marker)? + marker.len();
    let rest = &reason[start..];
    let (expected, actual) = rest.split_once(", found ")?;
    Some((expected.trim(), actual.trim()))
}

fn make_relative(index: &DocumentIndex, path: &Path) -> String {
    path.strip_prefix(&index.repo_root)
        .map(path_to_unix)
        .unwrap_or_else(|_| path_to_unix(path))
}

fn path_to_unix(path: &Path) -> String {
    path.components()
        .map(|component| component.as_os_str().to_string_lossy().into_owned())
        .collect::<Vec<_>>()
        .join("/")
}

fn plural(count: usize) -> &'static str {
    if count == 1 {
        ""
    } else {
        "s"
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum InvalidKind {
    Names,
    Frontmatter,
    Status,
}

#[derive(Debug)]
struct CompiledPattern {
    pattern: Pattern,
}

fn compile_patterns(
    repo_root: &Path,
    task_id: &str,
    field: &str,
    patterns: &[String],
) -> Result<Vec<CompiledPattern>> {
    patterns
        .iter()
        .map(|raw| {
            Pattern::new(raw)
                .map(|pattern| CompiledPattern { pattern })
                .map_err(|error| {
                    anyhow!(
                        "{}: invalid glob in {} at {}: {}",
                        task_id,
                        field,
                        repo_root.display(),
                        error
                    )
                })
        })
        .collect()
}

fn find_task_spec<'a>(index: &'a DocumentIndex, task_id: &str) -> Result<&'a Document> {
    if let Some(document) = index
        .documents
        .values()
        .find(|document| document.id.as_string() == task_id)
    {
        return Ok(document);
    }

    if let Some(entry) = index.invalid_entries.iter().find(|entry| {
        make_relative(index, &entry.path)
            .rsplit('/')
            .next()
            .is_some_and(|file_name| file_name.starts_with(&format!("{task_id}-")))
    }) {
        bail!(
            "task spec {} is invalid: {} ({})",
            task_id,
            make_relative(index, &entry.path),
            entry.reason
        );
    }

    bail!("task spec {task_id} does not exist")
}

fn path_patterns_intersect(left: &[PathToken], right: &[PathToken]) -> bool {
    let mut stack = vec![(0usize, 0usize)];
    let mut visited = BTreeSet::new();

    while let Some(state) = stack.pop() {
        for closure in path_epsilon_closure(left, right, state) {
            if !visited.insert(closure) {
                continue;
            }

            let (left_index, right_index) = closure;
            if left_index == left.len() && right_index == right.len() {
                return true;
            }

            for left_transition in path_transitions(left, left_index) {
                for right_transition in path_transitions(right, right_index) {
                    if segment_labels_intersect(left_transition.label, right_transition.label) {
                        stack.push((left_transition.next, right_transition.next));
                    }
                }
            }
        }
    }

    false
}

fn path_epsilon_closure(
    left: &[PathToken],
    right: &[PathToken],
    start: (usize, usize),
) -> Vec<(usize, usize)> {
    let mut stack = vec![start];
    let mut visited = BTreeSet::new();
    let mut closure = Vec::new();

    while let Some((left_index, right_index)) = stack.pop() {
        if !visited.insert((left_index, right_index)) {
            continue;
        }
        closure.push((left_index, right_index));

        if matches!(left.get(left_index), Some(PathToken::AnySegments)) {
            stack.push((left_index + 1, right_index));
        }
        if matches!(right.get(right_index), Some(PathToken::AnySegments)) {
            stack.push((left_index, right_index + 1));
        }
    }

    closure
}

fn path_transitions(pattern: &[PathToken], index: usize) -> Vec<PathTransition<'_>> {
    match pattern.get(index) {
        Some(PathToken::AnySegments) => vec![PathTransition {
            label: SegmentLabel::AnySegment,
            next: index,
        }],
        Some(PathToken::Segment(tokens)) => vec![PathTransition {
            label: SegmentLabel::Pattern(tokens),
            next: index + 1,
        }],
        None => Vec::new(),
    }
}

fn segment_labels_intersect(left: SegmentLabel<'_>, right: SegmentLabel<'_>) -> bool {
    match (left, right) {
        (SegmentLabel::AnySegment, SegmentLabel::AnySegment) => true,
        (SegmentLabel::AnySegment, SegmentLabel::Pattern(tokens))
        | (SegmentLabel::Pattern(tokens), SegmentLabel::AnySegment) => {
            segment_patterns_intersect(tokens, &[SegmentToken::AnySeq])
        }
        (SegmentLabel::Pattern(left), SegmentLabel::Pattern(right)) => {
            segment_patterns_intersect(left, right)
        }
    }
}

fn segment_patterns_intersect(left: &[SegmentToken], right: &[SegmentToken]) -> bool {
    let mut stack = vec![(0usize, 0usize)];
    let mut visited = BTreeSet::new();

    while let Some(state) = stack.pop() {
        for closure in segment_epsilon_closure(left, right, state) {
            if !visited.insert(closure) {
                continue;
            }

            let (left_index, right_index) = closure;
            if left_index == left.len() && right_index == right.len() {
                return true;
            }

            for left_transition in segment_transitions(left, left_index) {
                for right_transition in segment_transitions(right, right_index) {
                    if left_transition
                        .charset
                        .intersects(&right_transition.charset)
                    {
                        stack.push((left_transition.next, right_transition.next));
                    }
                }
            }
        }
    }

    false
}

fn segment_epsilon_closure(
    left: &[SegmentToken],
    right: &[SegmentToken],
    start: (usize, usize),
) -> Vec<(usize, usize)> {
    let mut stack = vec![start];
    let mut visited = BTreeSet::new();
    let mut closure = Vec::new();

    while let Some((left_index, right_index)) = stack.pop() {
        if !visited.insert((left_index, right_index)) {
            continue;
        }
        closure.push((left_index, right_index));

        if matches!(left.get(left_index), Some(SegmentToken::AnySeq)) {
            stack.push((left_index + 1, right_index));
        }
        if matches!(right.get(right_index), Some(SegmentToken::AnySeq)) {
            stack.push((left_index, right_index + 1));
        }
    }

    closure
}

fn segment_transitions(pattern: &[SegmentToken], index: usize) -> Vec<SegmentTransition> {
    match pattern.get(index) {
        Some(SegmentToken::Literal(ch)) => vec![SegmentTransition {
            charset: CharSet::from_char(*ch),
            next: index + 1,
        }],
        Some(SegmentToken::AnyChar) => vec![SegmentTransition {
            charset: CharSet::any_non_separator(),
            next: index + 1,
        }],
        Some(SegmentToken::AnySeq) => vec![SegmentTransition {
            charset: CharSet::any_non_separator(),
            next: index,
        }],
        Some(SegmentToken::Class(charset)) => vec![SegmentTransition {
            charset: charset.clone(),
            next: index + 1,
        }],
        None => Vec::new(),
    }
}

fn parse_path_pattern(pattern: &str) -> Option<Vec<PathToken>> {
    pattern
        .split('/')
        .map(|segment| {
            if segment == "**" {
                Some(PathToken::AnySegments)
            } else {
                parse_segment_pattern(segment).map(PathToken::Segment)
            }
        })
        .collect()
}

fn parse_segment_pattern(pattern: &str) -> Option<Vec<SegmentToken>> {
    let mut chars = pattern.chars().peekable();
    let mut tokens = Vec::new();

    while let Some(ch) = chars.next() {
        match ch {
            '*' => tokens.push(SegmentToken::AnySeq),
            '?' => tokens.push(SegmentToken::AnyChar),
            '[' => tokens.push(SegmentToken::Class(parse_char_class(&mut chars)?)),
            '\\' => tokens.push(SegmentToken::Literal(chars.next()?)),
            _ => tokens.push(SegmentToken::Literal(ch)),
        }
    }

    Some(tokens)
}

fn parse_char_class<I>(chars: &mut std::iter::Peekable<I>) -> Option<CharSet>
where
    I: Iterator<Item = char>,
{
    let mut negated = false;
    if matches!(chars.peek(), Some('!') | Some('^')) {
        negated = true;
        chars.next();
    }

    let mut ranges = Vec::new();
    let mut first_item = true;
    let mut closed = false;

    while let Some(ch) = chars.next() {
        if ch == ']' && !first_item {
            closed = true;
            break;
        }

        let start = if ch == '\\' { chars.next()? } else { ch };
        if matches!(chars.peek(), Some('-')) {
            chars.next();
            if let Some(end_raw) = chars.peek().copied() {
                if end_raw != ']' {
                    let end = if end_raw == '\\' {
                        chars.next();
                        chars.next()?
                    } else {
                        chars.next()?
                    };
                    let (range_start, range_end) = ordered_range(start, end);
                    ranges.push((range_start as u32, range_end as u32));
                    first_item = false;
                    continue;
                }
            }
            ranges.push((start as u32, start as u32));
            ranges.push(('-' as u32, '-' as u32));
            first_item = false;
            continue;
        }

        ranges.push((start as u32, start as u32));
        first_item = false;
    }

    if !closed {
        return None;
    }

    Some(CharSet::from_ranges(ranges, negated))
}

fn ordered_range(start: char, end: char) -> (char, char) {
    if start <= end {
        (start, end)
    } else {
        (end, start)
    }
}

#[derive(Debug, Clone)]
enum PathToken {
    AnySegments,
    Segment(Vec<SegmentToken>),
}

#[derive(Debug, Clone)]
enum SegmentToken {
    Literal(char),
    AnyChar,
    AnySeq,
    Class(CharSet),
}

#[derive(Clone, Copy)]
enum SegmentLabel<'a> {
    AnySegment,
    Pattern(&'a [SegmentToken]),
}

struct PathTransition<'a> {
    label: SegmentLabel<'a>,
    next: usize,
}

struct SegmentTransition {
    charset: CharSet,
    next: usize,
}

#[derive(Debug, Clone)]
struct CharSet {
    ranges: Vec<(u32, u32)>,
}

impl CharSet {
    fn from_char(ch: char) -> Self {
        Self {
            ranges: vec![(ch as u32, ch as u32)],
        }
    }

    fn any_non_separator() -> Self {
        Self {
            ranges: domain_ranges(),
        }
    }

    fn from_ranges(ranges: Vec<(u32, u32)>, negated: bool) -> Self {
        let normalized = normalize_ranges(
            ranges
                .into_iter()
                .filter(|(start, end)| !(*start <= '/' as u32 && '/' as u32 <= *end))
                .collect(),
        );
        if negated {
            Self {
                ranges: complement_ranges(&normalized),
            }
        } else {
            Self { ranges: normalized }
        }
    }

    fn intersects(&self, other: &Self) -> bool {
        let mut left_index = 0;
        let mut right_index = 0;
        while left_index < self.ranges.len() && right_index < other.ranges.len() {
            let (left_start, left_end) = self.ranges[left_index];
            let (right_start, right_end) = other.ranges[right_index];
            if left_end < right_start {
                left_index += 1;
                continue;
            }
            if right_end < left_start {
                right_index += 1;
                continue;
            }
            return true;
        }
        false
    }
}

fn normalize_ranges(mut ranges: Vec<(u32, u32)>) -> Vec<(u32, u32)> {
    ranges.sort_unstable();
    let mut normalized: Vec<(u32, u32)> = Vec::new();
    for (start, end) in ranges {
        if let Some((_, last_end)) = normalized.last_mut() {
            if start <= last_end.saturating_add(1) {
                *last_end = (*last_end).max(end);
                continue;
            }
        }
        normalized.push((start, end));
    }
    normalized
}

fn complement_ranges(ranges: &[(u32, u32)]) -> Vec<(u32, u32)> {
    let mut complement = Vec::new();
    let mut cursor_ranges = domain_ranges().into_iter();
    let mut current_domain = cursor_ranges.next();
    let mut index = 0;

    while let Some((domain_start, domain_end)) = current_domain {
        while index < ranges.len() && ranges[index].1 < domain_start {
            index += 1;
        }

        let mut local_start = domain_start;
        while index < ranges.len() && ranges[index].0 <= domain_end {
            let (range_start, range_end) = ranges[index];
            if local_start < range_start {
                complement.push((local_start, range_start - 1));
            }
            if range_end == u32::MAX {
                local_start = u32::MAX;
                break;
            }
            local_start = range_end.saturating_add(1);
            if local_start > domain_end {
                break;
            }
            index += 1;
        }

        if local_start <= domain_end {
            complement.push((local_start, domain_end));
        }
        current_domain = cursor_ranges.next();
    }

    complement
}

fn domain_ranges() -> Vec<(u32, u32)> {
    vec![
        (0, ('/' as u32).saturating_sub(1)),
        (('/' as u32).saturating_add(1), 0xD7FF),
        (0xE000, 0x10FFFF),
    ]
}
