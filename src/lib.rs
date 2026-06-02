use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SUITE_SCHEMA_VERSION: u32 = 1;
pub const TRACE_SCHEMA_VERSION: u32 = 1;
pub const REPORT_SCHEMA_VERSION: u32 = 1;
pub const AUTOPSY_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TaskSuite {
    pub schema_version: u32,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tasks: Vec<BenchTask>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct BenchTask {
    pub id: String,
    pub prompt: String,
    #[serde(default)]
    pub expected_files: Vec<String>,
    #[serde(default)]
    pub expected_tests: Vec<String>,
    pub success_command: Option<String>,
    #[serde(default)]
    pub tags: Vec<String>,
    pub timeout_seconds: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentVariant {
    Native,
    CtxhelmPlan,
    CtxhelmMcp,
    CtxhelmPack,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum TaskStatus {
    Success,
    Failure,
    Skipped,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum CommandClass {
    Test,
    Build,
    Lint,
    Typecheck,
    Other,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PrivacyStatus {
    pub source_free: bool,
    pub raw_source_logged: bool,
    pub raw_prompt_logged: bool,
    pub raw_transcript_logged: bool,
    pub raw_terminal_logged: bool,
}

impl PrivacyStatus {
    pub fn source_free() -> Self {
        Self {
            source_free: true,
            raw_source_logged: false,
            raw_prompt_logged: false,
            raw_transcript_logged: false,
            raw_terminal_logged: false,
        }
    }
}

impl Default for PrivacyStatus {
    fn default() -> Self {
        Self::source_free()
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentTrace {
    pub schema_version: u32,
    pub task_id: String,
    pub agent: String,
    pub variant: AgentVariant,
    pub status: TaskStatus,
    #[serde(default)]
    pub recommended_files: Vec<PathObservation>,
    #[serde(default)]
    pub files_read: Vec<PathObservation>,
    #[serde(default)]
    pub files_edited: Vec<PathObservation>,
    #[serde(default)]
    pub commands: Vec<CommandObservation>,
    #[serde(default)]
    pub tool_call_count: u32,
    pub token_estimate: Option<u64>,
    pub elapsed_millis: Option<u64>,
    pub time_to_first_relevant_file_millis: Option<u64>,
    pub privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct PathObservation {
    pub path: String,
    pub path_hash: Option<String>,
    pub observed_at_millis: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct CommandObservation {
    pub command_class: CommandClass,
    pub command_hash: Option<String>,
    #[serde(default)]
    pub touched_tests: Vec<String>,
    pub exit_status: Option<i32>,
    pub elapsed_millis: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AgentEventKind {
    RecommendedFile,
    FileRead,
    FileEdit,
    Command,
    Status,
    Usage,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AgentEvent {
    pub schema_version: u32,
    pub task_id: String,
    pub event_kind: AgentEventKind,
    pub path: Option<String>,
    pub command_class: Option<CommandClass>,
    pub command_hash: Option<String>,
    #[serde(default)]
    pub touched_tests: Vec<String>,
    pub exit_status: Option<i32>,
    pub status: Option<TaskStatus>,
    pub token_estimate: Option<u64>,
    pub elapsed_millis: Option<u64>,
    pub observed_at_millis: Option<u64>,
    #[serde(default)]
    pub privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct RunReport {
    pub schema_version: u32,
    pub suite_name: String,
    pub agent: String,
    pub variant: AgentVariant,
    pub summary: ReportSummary,
    pub tasks: Vec<TaskReport>,
    pub privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct ReportSummary {
    pub task_count: usize,
    pub success_count: usize,
    pub success_rate: f32,
    pub total_files_read: usize,
    pub total_irrelevant_file_reads: usize,
    pub irrelevant_read_rate: f32,
    pub average_recommendation_precision: f32,
    pub average_recommendation_recall: f32,
    pub average_context_precision: f32,
    pub average_edited_file_recall: f32,
    pub validation_coverage_rate: f32,
    pub average_time_to_first_relevant_file_millis: Option<f32>,
    pub total_tool_calls: u32,
    pub total_token_estimate: u64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct TaskReport {
    pub task_id: String,
    pub status: TaskStatus,
    pub expected_file_count: usize,
    pub expected_test_count: usize,
    pub recommended_file_count: usize,
    pub relevant_recommended_file_count: usize,
    pub irrelevant_recommended_file_count: usize,
    pub recommendation_precision: f32,
    pub recommendation_recall: f32,
    pub files_read_count: usize,
    pub relevant_files_read_count: usize,
    pub irrelevant_file_read_count: usize,
    pub files_edited_count: usize,
    pub expected_files_edited_count: usize,
    pub context_precision: f32,
    pub edited_file_recall: f32,
    pub validation_covered: bool,
    pub tool_call_count: u32,
    pub token_estimate: u64,
    pub elapsed_millis: Option<u64>,
    pub time_to_first_relevant_file_millis: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct CompareReport {
    pub schema_version: u32,
    pub base_agent: String,
    pub base_variant: AgentVariant,
    pub head_agent: String,
    pub head_variant: AgentVariant,
    pub task_count_delta: isize,
    pub success_rate_delta: f32,
    pub irrelevant_read_rate_delta: f32,
    pub average_recommendation_precision_delta: f32,
    pub average_recommendation_recall_delta: f32,
    pub average_context_precision_delta: f32,
    pub average_edited_file_recall_delta: f32,
    pub validation_coverage_rate_delta: f32,
    pub total_tool_calls_delta: i64,
    pub total_token_estimate_delta: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutopsyReport {
    pub schema_version: u32,
    pub suite_name: String,
    pub agent: String,
    pub variant: AgentVariant,
    pub summary: AutopsySummary,
    pub tasks: Vec<AutopsyTask>,
    pub privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutopsySummary {
    pub task_count: usize,
    pub failed_task_count: usize,
    pub validation_gap_count: usize,
    pub overbroad_edit_count: usize,
    pub missing_expected_inspection_count: usize,
    pub changed_without_read_count: usize,
    pub high_risk_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum AutopsyRisk {
    Low,
    Medium,
    High,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct AutopsyTask {
    pub task_id: String,
    pub status: TaskStatus,
    pub risk: AutopsyRisk,
    pub changed_files: Vec<String>,
    pub expected_files: Vec<String>,
    pub missing_expected_inspections: Vec<String>,
    pub changed_without_read: Vec<String>,
    pub overbroad_edits: Vec<String>,
    pub validation_gap: bool,
    pub notes: Vec<String>,
}

pub fn load_suite(path: &Path) -> Result<TaskSuite> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let suite = serde_json::from_str::<TaskSuite>(&raw)
        .with_context(|| format!("parse suite {}", path.display()))?;
    validate_suite(&suite)?;
    Ok(suite)
}

pub fn validate_suite(suite: &TaskSuite) -> Result<()> {
    if suite.schema_version != SUITE_SCHEMA_VERSION {
        bail!(
            "unsupported suite schema version {}; expected {}",
            suite.schema_version,
            SUITE_SCHEMA_VERSION
        );
    }
    if suite.name.trim().is_empty() {
        bail!("suite name is required");
    }
    if suite.tasks.is_empty() {
        bail!("suite must contain at least one task");
    }
    let mut ids = BTreeSet::new();
    for task in &suite.tasks {
        if task.id.trim().is_empty() {
            bail!("task id is required");
        }
        if !ids.insert(task.id.as_str()) {
            bail!("duplicate task id `{}`", task.id);
        }
        if task.prompt.trim().is_empty() {
            bail!("task `{}` prompt is required", task.id);
        }
        for path in task.expected_files.iter().chain(task.expected_tests.iter()) {
            validate_safe_relative_path(path)
                .with_context(|| format!("task `{}` unsafe path `{}`", task.id, path))?;
        }
    }
    Ok(())
}

pub fn load_traces(trace_dir: &Path) -> Result<Vec<AgentTrace>> {
    let mut traces = Vec::new();
    for entry in fs::read_dir(trace_dir).with_context(|| format!("read {}", trace_dir.display()))? {
        let entry = entry?;
        let path = entry.path();
        if path.extension().and_then(|ext| ext.to_str()) != Some("json") {
            continue;
        }
        let raw = fs::read_to_string(&path).with_context(|| format!("read {}", path.display()))?;
        let trace = serde_json::from_str::<AgentTrace>(&raw)
            .with_context(|| format!("parse trace {}", path.display()))?;
        validate_trace(&trace).with_context(|| format!("trace {}", path.display()))?;
        traces.push(trace);
    }
    traces.sort_by(|left, right| left.task_id.cmp(&right.task_id));
    Ok(traces)
}

pub fn load_agent_events(path: &Path) -> Result<Vec<AgentEvent>> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let mut events = Vec::new();
    for (index, line) in raw.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let event = serde_json::from_str::<AgentEvent>(line)
            .with_context(|| format!("parse event {} in {}", index + 1, path.display()))?;
        validate_agent_event(&event)
            .with_context(|| format!("event {} in {}", index + 1, path.display()))?;
        events.push(event);
    }
    Ok(events)
}

pub fn events_from_agent_stream_jsonl(
    task_id: &str,
    jsonl: &str,
    repo_root: Option<&Path>,
    expected_tests: &[String],
) -> Result<Vec<AgentEvent>> {
    if task_id.trim().is_empty() {
        bail!("stream task id is required");
    }
    let mut events = Vec::new();
    let mut seen = BTreeSet::new();
    for (index, line) in jsonl.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let value = serde_json::from_str::<serde_json::Value>(line)
            .with_context(|| format!("parse stream line {}", index + 1))?;
        collect_stream_events(
            task_id,
            &value,
            repo_root,
            expected_tests,
            index as u64,
            &mut seen,
            &mut events,
        )?;
    }
    Ok(events)
}

pub fn validate_agent_event(event: &AgentEvent) -> Result<()> {
    if event.schema_version != TRACE_SCHEMA_VERSION {
        bail!(
            "unsupported event schema version {}; expected {}",
            event.schema_version,
            TRACE_SCHEMA_VERSION
        );
    }
    if event.task_id.trim().is_empty() {
        bail!("event task id is required");
    }
    if !event.privacy.source_free
        || event.privacy.raw_source_logged
        || event.privacy.raw_prompt_logged
        || event.privacy.raw_transcript_logged
        || event.privacy.raw_terminal_logged
    {
        bail!("event is not source-free");
    }
    if let Some(path) = &event.path {
        validate_safe_relative_path(path)?;
    }
    for test in &event.touched_tests {
        validate_safe_relative_path(test)?;
    }
    match event.event_kind {
        AgentEventKind::RecommendedFile | AgentEventKind::FileRead | AgentEventKind::FileEdit => {
            if event.path.is_none() {
                bail!("{:?} event requires path", event.event_kind);
            }
        }
        AgentEventKind::Command => {
            if event.command_class.is_none() {
                bail!("command event requires commandClass");
            }
        }
        AgentEventKind::Status => {
            if event.status.is_none() {
                bail!("status event requires status");
            }
        }
        AgentEventKind::Usage => {}
    }
    Ok(())
}

pub fn traces_from_agent_events(
    suite: &TaskSuite,
    events: &[AgentEvent],
    agent: &str,
    variant: AgentVariant,
) -> Result<Vec<AgentTrace>> {
    validate_suite(suite)?;
    for event in events {
        validate_agent_event(event)?;
    }
    let task_ids = suite
        .tasks
        .iter()
        .map(|task| task.id.as_str())
        .collect::<BTreeSet<_>>();
    let mut by_task = BTreeMap::<String, Vec<&AgentEvent>>::new();
    for event in events {
        if !task_ids.contains(event.task_id.as_str()) {
            bail!("event references unknown task `{}`", event.task_id);
        }
        by_task
            .entry(event.task_id.clone())
            .or_default()
            .push(event);
    }

    let mut traces = Vec::new();
    for task in &suite.tasks {
        let task_events = by_task.remove(&task.id).unwrap_or_default();
        if task_events.is_empty() {
            continue;
        }
        traces.push(trace_from_task_events(
            task,
            &task_events,
            agent,
            variant.clone(),
        )?);
    }
    Ok(traces)
}

pub fn validate_trace(trace: &AgentTrace) -> Result<()> {
    if trace.schema_version != TRACE_SCHEMA_VERSION {
        bail!(
            "unsupported trace schema version {}; expected {}",
            trace.schema_version,
            TRACE_SCHEMA_VERSION
        );
    }
    if !trace.privacy.source_free
        || trace.privacy.raw_source_logged
        || trace.privacy.raw_prompt_logged
        || trace.privacy.raw_transcript_logged
        || trace.privacy.raw_terminal_logged
    {
        bail!("trace is not source-free");
    }
    for obs in trace
        .recommended_files
        .iter()
        .chain(trace.files_read.iter())
        .chain(trace.files_edited.iter())
    {
        validate_safe_relative_path(&obs.path)?;
    }
    for command in &trace.commands {
        for path in &command.touched_tests {
            validate_safe_relative_path(path)?;
        }
    }
    Ok(())
}

pub fn build_report(suite: &TaskSuite, traces: &[AgentTrace]) -> Result<RunReport> {
    validate_suite(suite)?;
    if traces.is_empty() {
        bail!("at least one trace is required");
    }
    for trace in traces {
        validate_trace(trace)?;
    }

    let first = &traces[0];
    let mut tasks_by_id = BTreeMap::new();
    for task in &suite.tasks {
        tasks_by_id.insert(task.id.as_str(), task);
    }

    let mut task_reports = Vec::new();
    for trace in traces {
        if trace.agent != first.agent || trace.variant != first.variant {
            bail!("all traces in a run report must use one agent and variant");
        }
        let Some(task) = tasks_by_id.get(trace.task_id.as_str()) else {
            bail!("trace references unknown task `{}`", trace.task_id);
        };
        task_reports.push(task_report(task, trace));
    }

    let summary = summarize(&task_reports);
    Ok(RunReport {
        schema_version: REPORT_SCHEMA_VERSION,
        suite_name: suite.name.clone(),
        agent: first.agent.clone(),
        variant: first.variant.clone(),
        summary,
        tasks: task_reports,
        privacy: PrivacyStatus::source_free(),
    })
}

pub fn compare_reports(base: &RunReport, head: &RunReport) -> CompareReport {
    CompareReport {
        schema_version: REPORT_SCHEMA_VERSION,
        base_agent: base.agent.clone(),
        base_variant: base.variant.clone(),
        head_agent: head.agent.clone(),
        head_variant: head.variant.clone(),
        task_count_delta: head.summary.task_count as isize - base.summary.task_count as isize,
        success_rate_delta: head.summary.success_rate - base.summary.success_rate,
        irrelevant_read_rate_delta: head.summary.irrelevant_read_rate
            - base.summary.irrelevant_read_rate,
        average_recommendation_precision_delta: head.summary.average_recommendation_precision
            - base.summary.average_recommendation_precision,
        average_recommendation_recall_delta: head.summary.average_recommendation_recall
            - base.summary.average_recommendation_recall,
        average_context_precision_delta: head.summary.average_context_precision
            - base.summary.average_context_precision,
        average_edited_file_recall_delta: head.summary.average_edited_file_recall
            - base.summary.average_edited_file_recall,
        validation_coverage_rate_delta: head.summary.validation_coverage_rate
            - base.summary.validation_coverage_rate,
        total_tool_calls_delta: head.summary.total_tool_calls as i64
            - base.summary.total_tool_calls as i64,
        total_token_estimate_delta: head.summary.total_token_estimate as i64
            - base.summary.total_token_estimate as i64,
    }
}

pub fn build_autopsy(suite: &TaskSuite, traces: &[AgentTrace]) -> Result<AutopsyReport> {
    validate_suite(suite)?;
    if traces.is_empty() {
        bail!("at least one trace is required");
    }
    for trace in traces {
        validate_trace(trace)?;
    }

    let first = &traces[0];
    let mut tasks_by_id = BTreeMap::new();
    for task in &suite.tasks {
        tasks_by_id.insert(task.id.as_str(), task);
    }

    let mut tasks = Vec::new();
    for trace in traces {
        if trace.agent != first.agent || trace.variant != first.variant {
            bail!("all traces in an autopsy must use one agent and variant");
        }
        let Some(task) = tasks_by_id.get(trace.task_id.as_str()) else {
            bail!("trace references unknown task `{}`", trace.task_id);
        };
        tasks.push(autopsy_task(task, trace));
    }

    let summary = summarize_autopsy(&tasks);
    Ok(AutopsyReport {
        schema_version: AUTOPSY_SCHEMA_VERSION,
        suite_name: suite.name.clone(),
        agent: first.agent.clone(),
        variant: first.variant.clone(),
        summary,
        tasks,
        privacy: PrivacyStatus::source_free(),
    })
}

pub fn render_markdown_report(report: &RunReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# HelmBench Report: {} / {:?}\n\n",
        report.agent, report.variant
    ));
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "- Suite: `{}`\n- Tasks: `{}`\n- Success rate: `{:.1}%`\n- Irrelevant read rate: `{:.1}%`\n- Recommendation precision: `{:.1}%`\n- Recommendation recall: `{:.1}%`\n- Context precision: `{:.1}%`\n- Edited-file recall: `{:.1}%`\n- Validation coverage: `{:.1}%`\n- Tool calls: `{}`\n- Token estimate: `{}`\n- Source-free: `{}`\n\n",
        report.suite_name,
        report.summary.task_count,
        pct(report.summary.success_rate),
        pct(report.summary.irrelevant_read_rate),
        pct(report.summary.average_recommendation_precision),
        pct(report.summary.average_recommendation_recall),
        pct(report.summary.average_context_precision),
        pct(report.summary.average_edited_file_recall),
        pct(report.summary.validation_coverage_rate),
        report.summary.total_tool_calls,
        report.summary.total_token_estimate,
        report.privacy.source_free
    ));
    out.push_str("## Tasks\n\n");
    out.push_str("| Task | Status | Recommendations | Rec recall | Reads | Irrelevant reads | Context precision | Validation | Tool calls |\n");
    out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | --- | ---: |\n");
    for task in &report.tasks {
        out.push_str(&format!(
            "| `{}` | {:?} | {} | {:.1}% | {} | {} | {:.1}% | {} | {} |\n",
            task.task_id,
            task.status,
            task.recommended_file_count,
            pct(task.recommendation_recall),
            task.files_read_count,
            task.irrelevant_file_read_count,
            pct(task.context_precision),
            if task.validation_covered { "yes" } else { "no" },
            task.tool_call_count
        ));
    }
    out.push_str("\n## Privacy\n\n");
    out.push_str("- Raw source logged: `false`\n- Raw prompts logged: `false`\n- Raw transcripts logged: `false`\n- Raw terminal logs logged: `false`\n");
    out
}

pub fn render_markdown_compare(compare: &CompareReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# HelmBench Compare: {:?} -> {:?}\n\n",
        compare.base_variant, compare.head_variant
    ));
    out.push_str("| Metric | Delta |\n| --- | ---: |\n");
    out.push_str(&format!(
        "| Task success rate | {:+.1}% |\n",
        pct(compare.success_rate_delta)
    ));
    out.push_str(&format!(
        "| Irrelevant read rate | {:+.1}% |\n",
        pct(compare.irrelevant_read_rate_delta)
    ));
    out.push_str(&format!(
        "| Recommendation precision | {:+.1}% |\n",
        pct(compare.average_recommendation_precision_delta)
    ));
    out.push_str(&format!(
        "| Recommendation recall | {:+.1}% |\n",
        pct(compare.average_recommendation_recall_delta)
    ));
    out.push_str(&format!(
        "| Context precision | {:+.1}% |\n",
        pct(compare.average_context_precision_delta)
    ));
    out.push_str(&format!(
        "| Edited-file recall | {:+.1}% |\n",
        pct(compare.average_edited_file_recall_delta)
    ));
    out.push_str(&format!(
        "| Validation coverage | {:+.1}% |\n",
        pct(compare.validation_coverage_rate_delta)
    ));
    out.push_str(&format!(
        "| Tool calls | {:+} |\n",
        compare.total_tool_calls_delta
    ));
    out.push_str(&format!(
        "| Token estimate | {:+} |\n",
        compare.total_token_estimate_delta
    ));
    out
}

pub fn render_markdown_autopsy(report: &AutopsyReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# HelmBench Autopsy: {} / {:?}\n\n",
        report.agent, report.variant
    ));
    out.push_str("## Summary\n\n");
    out.push_str(&format!(
        "- Suite: `{}`\n- Tasks: `{}`\n- Failed tasks: `{}`\n- Validation gaps: `{}`\n- Overbroad edits: `{}`\n- Missing expected inspections: `{}`\n- Changed without read: `{}`\n- High risk tasks: `{}`\n- Source-free: `{}`\n\n",
        report.suite_name,
        report.summary.task_count,
        report.summary.failed_task_count,
        report.summary.validation_gap_count,
        report.summary.overbroad_edit_count,
        report.summary.missing_expected_inspection_count,
        report.summary.changed_without_read_count,
        report.summary.high_risk_count,
        report.privacy.source_free
    ));
    out.push_str("## Tasks\n\n");
    out.push_str(
        "| Task | Status | Risk | Changed | Overbroad | Missing inspections | Validation gap |\n",
    );
    out.push_str("| --- | --- | --- | ---: | ---: | ---: | --- |\n");
    for task in &report.tasks {
        out.push_str(&format!(
            "| `{}` | {:?} | {:?} | {} | {} | {} | {} |\n",
            task.task_id,
            task.status,
            task.risk,
            task.changed_files.len(),
            task.overbroad_edits.len(),
            task.missing_expected_inspections.len(),
            if task.validation_gap { "yes" } else { "no" }
        ));
    }

    for task in &report.tasks {
        out.push_str(&format!("\n### `{}`\n\n", task.task_id));
        out.push_str(&format!("- Status: `{:?}`\n", task.status));
        out.push_str(&format!("- Risk: `{:?}`\n", task.risk));
        out.push_str(&markdown_path_list("Changed files", &task.changed_files));
        out.push_str(&markdown_path_list(
            "Overbroad edits",
            &task.overbroad_edits,
        ));
        out.push_str(&markdown_path_list(
            "Missing expected inspections",
            &task.missing_expected_inspections,
        ));
        out.push_str(&markdown_path_list(
            "Changed without recorded read",
            &task.changed_without_read,
        ));
        if !task.notes.is_empty() {
            out.push_str("- Notes:\n");
            for note in &task.notes {
                out.push_str(&format!("  - {}\n", note));
            }
        }
    }

    out.push_str("\n## Privacy\n\n");
    out.push_str("- Raw source logged: `false`\n- Raw prompts logged: `false`\n- Raw transcripts logged: `false`\n- Raw terminal logs logged: `false`\n");
    out
}

fn markdown_path_list(label: &str, paths: &[String]) -> String {
    if paths.is_empty() {
        format!("- {label}: none\n")
    } else {
        format!(
            "- {label}: {}\n",
            paths
                .iter()
                .map(|path| format!("`{}`", path.replace('`', "\\`")))
                .collect::<Vec<_>>()
                .join(", ")
        )
    }
}

pub fn render_html_dashboard(reports: &[RunReport]) -> Result<String> {
    if reports.is_empty() {
        bail!("at least one report is required");
    }
    for report in reports {
        if !report.privacy.source_free
            || report.privacy.raw_source_logged
            || report.privacy.raw_prompt_logged
            || report.privacy.raw_transcript_logged
            || report.privacy.raw_terminal_logged
        {
            bail!("dashboard report is not source-free");
        }
    }

    let mut out = String::new();
    out.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str("<title>HelmBench Dashboard</title>\n");
    out.push_str("<style>\n");
    out.push_str(DASHBOARD_CSS);
    out.push_str("\n</style>\n</head>\n<body>\n");
    out.push_str("<main class=\"shell\">\n");
    out.push_str("<header class=\"hero\">\n");
    out.push_str("<div><p class=\"eyebrow\">Source-free agent evaluation</p>\n");
    out.push_str("<h1>HelmBench Dashboard</h1>\n");
    out.push_str("<p class=\"lede\">Measure how coding agents navigate repositories, validate changes, and spend context.</p></div>\n");
    out.push_str("<div class=\"privacy-badge\">Source-free reports</div>\n");
    out.push_str("</header>\n");

    out.push_str("<section class=\"summary-grid\" aria-label=\"Run summaries\">\n");
    for report in reports {
        out.push_str("<article class=\"run-card\">\n");
        out.push_str(&format!(
            "<div class=\"run-title\"><span>{}</span><code>{:?}</code></div>\n",
            html_escape(&report.agent),
            report.variant
        ));
        out.push_str(&format!(
            "<p class=\"suite\">Suite: <strong>{}</strong></p>\n",
            html_escape(&report.suite_name)
        ));
        out.push_str("<div class=\"metric-row\">\n");
        out.push_str(&metric_tile(
            "Success",
            pct(report.summary.success_rate),
            "%",
        ));
        out.push_str(&metric_tile(
            "Validation",
            pct(report.summary.validation_coverage_rate),
            "%",
        ));
        out.push_str(&metric_tile(
            "Context precision",
            pct(report.summary.average_context_precision),
            "%",
        ));
        out.push_str(&metric_tile(
            "Irrelevant reads",
            pct(report.summary.irrelevant_read_rate),
            "%",
        ));
        out.push_str("</div>\n");
        out.push_str("<dl class=\"facts\">\n");
        out.push_str(&fact("Tasks", report.summary.task_count.to_string()));
        out.push_str(&fact(
            "Files read",
            report.summary.total_files_read.to_string(),
        ));
        out.push_str(&fact(
            "Tool calls",
            report.summary.total_tool_calls.to_string(),
        ));
        out.push_str(&fact(
            "Token estimate",
            report.summary.total_token_estimate.to_string(),
        ));
        out.push_str("</dl>\n");
        out.push_str("</article>\n");
    }
    out.push_str("</section>\n");

    out.push_str("<section class=\"panel\">\n");
    out.push_str("<h2>Run Comparison</h2>\n");
    out.push_str("<div class=\"table-wrap\"><table>\n");
    out.push_str("<thead><tr><th>Run</th><th>Suite</th><th>Tasks</th><th>Success</th><th>Validation</th><th>Context precision</th><th>Edited recall</th><th>Irrelevant reads</th><th>Tools</th><th>Tokens</th></tr></thead>\n<tbody>\n");
    for report in reports {
        out.push_str(&format!(
            "<tr><td><strong>{}</strong><br><code>{:?}</code></td><td>{}</td><td>{}</td><td>{:.1}%</td><td>{:.1}%</td><td>{:.1}%</td><td>{:.1}%</td><td>{:.1}%</td><td>{}</td><td>{}</td></tr>\n",
            html_escape(&report.agent),
            report.variant,
            html_escape(&report.suite_name),
            report.summary.task_count,
            pct(report.summary.success_rate),
            pct(report.summary.validation_coverage_rate),
            pct(report.summary.average_context_precision),
            pct(report.summary.average_edited_file_recall),
            pct(report.summary.irrelevant_read_rate),
            report.summary.total_tool_calls,
            report.summary.total_token_estimate,
        ));
    }
    out.push_str("</tbody></table></div>\n</section>\n");

    out.push_str("<section class=\"panel\">\n");
    out.push_str("<h2>Task Detail</h2>\n");
    out.push_str("<div class=\"table-wrap\"><table>\n");
    out.push_str("<thead><tr><th>Run</th><th>Task</th><th>Status</th><th>Recommendations</th><th>Reads</th><th>Irrelevant</th><th>Validation</th><th>First relevant file</th></tr></thead>\n<tbody>\n");
    for report in reports {
        for task in &report.tasks {
            out.push_str(&format!(
                "<tr><td>{}<br><code>{:?}</code></td><td><code>{}</code></td><td><span class=\"status status-{}\">{:?}</span></td><td>{} / {} relevant</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
                html_escape(&report.agent),
                report.variant,
                html_escape(&task.task_id),
                status_class(&task.status),
                task.status,
                task.relevant_recommended_file_count,
                task.recommended_file_count,
                task.files_read_count,
                task.irrelevant_file_read_count,
                if task.validation_covered { "yes" } else { "no" },
                task.time_to_first_relevant_file_millis
                    .map(|millis| format!("{millis} ms"))
                    .unwrap_or_else(|| "n/a".to_string())
            ));
        }
    }
    out.push_str("</tbody></table></div>\n</section>\n");

    out.push_str("<section class=\"privacy\">\n");
    out.push_str("<h2>Privacy Contract</h2>\n");
    out.push_str("<p>This dashboard is generated from source-free HelmBench reports. It contains paths/count-derived metrics only; raw source, raw prompts, raw transcripts, raw terminal logs, and raw MCP payloads are not included.</p>\n");
    out.push_str("</section>\n");
    out.push_str("</main>\n</body>\n</html>\n");
    Ok(out)
}

pub fn write_json<T: Serialize>(value: &T, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json).with_context(|| format!("write {}", path.display()))
}

const DASHBOARD_CSS: &str = r#"
:root {
  color-scheme: light;
  --bg: #f5f7fb;
  --ink: #172033;
  --muted: #5c667a;
  --line: #d8deea;
  --card: #ffffff;
  --accent: #0f766e;
  --accent-weak: #e0f2f1;
  --danger: #b42318;
  --warning: #b54708;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: Inter, ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.shell { width: min(1180px, calc(100% - 32px)); margin: 0 auto; padding: 40px 0; }
.hero { display: flex; justify-content: space-between; gap: 24px; align-items: flex-start; margin-bottom: 28px; }
.eyebrow { margin: 0 0 8px; color: var(--accent); font-size: 13px; font-weight: 700; text-transform: uppercase; letter-spacing: 0; }
h1 { margin: 0; font-size: 44px; line-height: 1.05; letter-spacing: 0; }
h2 { margin: 0 0 16px; font-size: 22px; letter-spacing: 0; }
.lede { margin: 12px 0 0; color: var(--muted); max-width: 680px; font-size: 17px; line-height: 1.5; }
.privacy-badge { border: 1px solid var(--line); background: var(--card); color: var(--accent); padding: 10px 12px; border-radius: 8px; font-weight: 700; white-space: nowrap; }
.summary-grid { display: grid; grid-template-columns: repeat(auto-fit, minmax(280px, 1fr)); gap: 16px; margin-bottom: 20px; }
.run-card, .panel, .privacy { background: var(--card); border: 1px solid var(--line); border-radius: 8px; padding: 18px; box-shadow: 0 8px 28px rgba(23, 32, 51, .06); }
.run-title { display: flex; justify-content: space-between; gap: 12px; align-items: center; font-size: 18px; font-weight: 800; }
code { background: #eef2f7; border-radius: 5px; padding: 2px 5px; font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace; font-size: .88em; }
.suite { margin: 10px 0 16px; color: var(--muted); }
.metric-row { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 10px; }
.metric { background: #f8fafc; border: 1px solid #e6ebf3; border-radius: 8px; padding: 12px; }
.metric span { display: block; color: var(--muted); font-size: 12px; font-weight: 700; text-transform: uppercase; letter-spacing: 0; }
.metric strong { display: block; margin-top: 8px; font-size: 24px; }
.facts { display: grid; grid-template-columns: repeat(2, minmax(0, 1fr)); gap: 8px 14px; margin: 16px 0 0; }
.facts div { display: flex; justify-content: space-between; gap: 12px; border-top: 1px solid #edf1f7; padding-top: 8px; }
dt { color: var(--muted); }
dd { margin: 0; font-weight: 700; }
.panel { margin-top: 20px; }
.table-wrap { overflow-x: auto; }
table { width: 100%; border-collapse: collapse; min-width: 860px; }
th, td { border-bottom: 1px solid #edf1f7; padding: 11px 10px; text-align: left; vertical-align: top; }
th { color: var(--muted); font-size: 12px; text-transform: uppercase; letter-spacing: 0; }
.status { display: inline-block; border-radius: 5px; padding: 3px 7px; font-weight: 800; font-size: 12px; }
.status-success { background: var(--accent-weak); color: var(--accent); }
.status-failure { background: #fee4e2; color: var(--danger); }
.status-skipped { background: #fff4e5; color: var(--warning); }
.privacy { margin-top: 20px; color: var(--muted); line-height: 1.55; }
.privacy p { margin: 0; }
@media (max-width: 720px) {
  .shell { width: min(100% - 20px, 1180px); padding: 24px 0; }
  .hero { display: block; }
  .privacy-badge { display: inline-block; margin-top: 16px; }
  h1 { font-size: 34px; }
}
"#;

fn metric_tile(label: &str, value: f32, suffix: &str) -> String {
    format!(
        "<div class=\"metric\"><span>{}</span><strong>{:.1}{}</strong></div>\n",
        html_escape(label),
        value,
        html_escape(suffix)
    )
}

fn fact(label: &str, value: String) -> String {
    format!(
        "<div><dt>{}</dt><dd>{}</dd></div>\n",
        html_escape(label),
        html_escape(&value)
    )
}

fn status_class(status: &TaskStatus) -> &'static str {
    match status {
        TaskStatus::Success => "success",
        TaskStatus::Failure => "failure",
        TaskStatus::Skipped => "skipped",
    }
}

fn html_escape(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '&' => escaped.push_str("&amp;"),
            '<' => escaped.push_str("&lt;"),
            '>' => escaped.push_str("&gt;"),
            '"' => escaped.push_str("&quot;"),
            '\'' => escaped.push_str("&#39;"),
            _ => escaped.push(ch),
        }
    }
    escaped
}

pub fn read_report(path: &Path) -> Result<RunReport> {
    let raw = fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let report = serde_json::from_str::<RunReport>(&raw)
        .with_context(|| format!("parse report {}", path.display()))?;
    if !report.privacy.source_free
        || report.privacy.raw_source_logged
        || report.privacy.raw_prompt_logged
        || report.privacy.raw_transcript_logged
        || report.privacy.raw_terminal_logged
    {
        bail!("report is not source-free");
    }
    Ok(report)
}

pub fn trace_from_ctxhelm_prepare_json(
    task: &BenchTask,
    json: &str,
    agent: &str,
    variant: AgentVariant,
    elapsed_millis: Option<u64>,
) -> Result<AgentTrace> {
    let value = serde_json::from_str::<serde_json::Value>(json).context("parse ctxhelm JSON")?;
    let mut recommended = Vec::new();
    collect_path_observations(&value, "targetFiles", &mut recommended)?;
    collect_path_observations(&value, "relatedTests", &mut recommended)?;
    dedupe_observations(&mut recommended);
    let tool_call_count = usize::from(value.get("taskId").is_some())
        + value
            .get("targetFiles")
            .and_then(|items| items.as_array())
            .map_or(0, Vec::len)
        + value
            .get("relatedTests")
            .and_then(|items| items.as_array())
            .map_or(0, Vec::len);
    let trace = AgentTrace {
        schema_version: TRACE_SCHEMA_VERSION,
        task_id: task.id.clone(),
        agent: agent.to_string(),
        variant,
        status: TaskStatus::Skipped,
        recommended_files: recommended,
        files_read: Vec::new(),
        files_edited: Vec::new(),
        commands: Vec::new(),
        tool_call_count: tool_call_count as u32,
        token_estimate: None,
        elapsed_millis,
        time_to_first_relevant_file_millis: None,
        privacy: PrivacyStatus::source_free(),
    };
    validate_trace(&trace)?;
    Ok(trace)
}

pub fn example_suite() -> TaskSuite {
    TaskSuite {
        schema_version: SUITE_SCHEMA_VERSION,
        name: "example-auth-bugs".to_string(),
        description: "Small source-free fixture suite for HelmBench smoke tests.".to_string(),
        tasks: vec![BenchTask {
            id: "auth-redirect-001".to_string(),
            prompt: "Fix the bug where expired sessions redirect incorrectly.".to_string(),
            expected_files: vec![
                "src/auth/session.ts".to_string(),
                "src/auth/middleware.ts".to_string(),
            ],
            expected_tests: vec!["tests/auth/session.test.ts".to_string()],
            success_command: Some("pnpm vitest run tests/auth/session.test.ts".to_string()),
            tags: vec!["bug_fix".to_string(), "auth".to_string()],
            timeout_seconds: Some(600),
        }],
    }
}

fn collect_path_observations(
    value: &serde_json::Value,
    key: &str,
    output: &mut Vec<PathObservation>,
) -> Result<()> {
    let Some(items) = value.get(key).and_then(|items| items.as_array()) else {
        return Ok(());
    };
    for item in items {
        let Some(path) = item.get("path").and_then(|path| path.as_str()) else {
            continue;
        };
        validate_safe_relative_path(path).with_context(|| format!("ctxhelm path `{path}`"))?;
        output.push(PathObservation {
            path: path.to_string(),
            path_hash: Some(format!("path:{}", stable_hash(path))),
            observed_at_millis: None,
        });
    }
    Ok(())
}

fn collect_stream_events(
    task_id: &str,
    value: &serde_json::Value,
    repo_root: Option<&Path>,
    expected_tests: &[String],
    observed_at_millis: u64,
    seen: &mut BTreeSet<String>,
    events: &mut Vec<AgentEvent>,
) -> Result<()> {
    if let Some(object) = value.as_object() {
        if let Some(tool_name) = stream_tool_name(object) {
            let input = object
                .get("input")
                .or_else(|| object.get("parameters"))
                .or_else(|| object.get("args"))
                .or_else(|| object.get("arguments"))
                .unwrap_or(value);
            collect_tool_event(
                task_id,
                tool_name,
                input,
                repo_root,
                expected_tests,
                observed_at_millis,
                seen,
                events,
            )?;
        } else if let Some(kind) = object
            .get("eventKind")
            .or_else(|| object.get("event_kind"))
            .and_then(|value| value.as_str())
        {
            collect_explicit_stream_event(
                task_id,
                kind,
                value,
                repo_root,
                observed_at_millis,
                seen,
                events,
            )?;
        }
        for child in object.values() {
            collect_stream_events(
                task_id,
                child,
                repo_root,
                expected_tests,
                observed_at_millis,
                seen,
                events,
            )?;
        }
    } else if let Some(items) = value.as_array() {
        for child in items {
            collect_stream_events(
                task_id,
                child,
                repo_root,
                expected_tests,
                observed_at_millis,
                seen,
                events,
            )?;
        }
    }
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn collect_tool_event(
    task_id: &str,
    tool_name: &str,
    input: &serde_json::Value,
    repo_root: Option<&Path>,
    expected_tests: &[String],
    observed_at_millis: u64,
    seen: &mut BTreeSet<String>,
    events: &mut Vec<AgentEvent>,
) -> Result<()> {
    let normalized_name = normalize_tool_name(tool_name);
    if is_read_tool(&normalized_name) {
        if let Some(path) = stream_path(input, repo_root) {
            push_unique_path_event(
                task_id,
                AgentEventKind::FileRead,
                path,
                observed_at_millis,
                seen,
                events,
            )?;
        }
    } else if is_edit_tool(&normalized_name) {
        if let Some(path) = stream_path(input, repo_root) {
            push_unique_path_event(
                task_id,
                AgentEventKind::FileEdit,
                path,
                observed_at_millis,
                seen,
                events,
            )?;
        }
    } else if is_command_tool(&normalized_name) {
        if let Some(command) = stream_command(input) {
            let event = AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: task_id.to_string(),
                event_kind: AgentEventKind::Command,
                path: None,
                command_class: Some(classify_command_text(command)),
                command_hash: Some(format!("cmd:{}", stable_hash(command))),
                touched_tests: expected_tests
                    .iter()
                    .filter(|path| command.contains(path.as_str()))
                    .cloned()
                    .collect(),
                exit_status: stream_exit_status(input),
                status: None,
                token_estimate: None,
                elapsed_millis: None,
                observed_at_millis: Some(observed_at_millis),
                privacy: PrivacyStatus::source_free(),
            };
            validate_agent_event(&event)?;
            let key = format!(
                "command:{}:{}",
                event.command_hash.as_deref().unwrap_or_default(),
                observed_at_millis
            );
            if seen.insert(key) {
                events.push(event);
            }
        }
    }
    Ok(())
}

fn collect_explicit_stream_event(
    task_id: &str,
    kind: &str,
    value: &serde_json::Value,
    repo_root: Option<&Path>,
    observed_at_millis: u64,
    seen: &mut BTreeSet<String>,
    events: &mut Vec<AgentEvent>,
) -> Result<()> {
    let event_kind = match kind {
        "recommended_file" | "recommended-file" | "recommendedFile" => {
            Some(AgentEventKind::RecommendedFile)
        }
        "file_read" | "file-read" | "fileRead" => Some(AgentEventKind::FileRead),
        "file_edit" | "file-edit" | "fileEdit" => Some(AgentEventKind::FileEdit),
        _ => None,
    };
    if let Some(event_kind) = event_kind {
        if let Some(path) = stream_path(value, repo_root) {
            push_unique_path_event(task_id, event_kind, path, observed_at_millis, seen, events)?;
        }
    }
    Ok(())
}

fn stream_tool_name(object: &serde_json::Map<String, serde_json::Value>) -> Option<&str> {
    object
        .get("name")
        .or_else(|| object.get("toolName"))
        .or_else(|| object.get("tool_name"))
        .or_else(|| object.get("tool"))
        .and_then(|value| value.as_str())
}

fn stream_path(value: &serde_json::Value, repo_root: Option<&Path>) -> Option<String> {
    let path = value
        .get("file_path")
        .or_else(|| value.get("filePath"))
        .or_else(|| value.get("filepath"))
        .or_else(|| value.get("path"))
        .and_then(|value| value.as_str())?;
    normalize_stream_path(path, repo_root)
}

fn normalize_stream_path(path: &str, repo_root: Option<&Path>) -> Option<String> {
    let path = Path::new(path);
    let relative = if path.is_absolute() {
        let root = repo_root?;
        path.strip_prefix(root).ok()?.to_string_lossy().to_string()
    } else {
        path.to_string_lossy().to_string()
    };
    validate_safe_relative_path(&relative).ok()?;
    Some(relative)
}

fn stream_command(value: &serde_json::Value) -> Option<&str> {
    value
        .get("command")
        .or_else(|| value.get("cmd"))
        .or_else(|| value.get("shellCommand"))
        .and_then(|value| value.as_str())
}

fn stream_exit_status(value: &serde_json::Value) -> Option<i32> {
    value
        .get("exit_status")
        .or_else(|| value.get("exitStatus"))
        .or_else(|| value.get("status"))
        .and_then(|value| value.as_i64())
        .and_then(|value| i32::try_from(value).ok())
}

fn push_unique_path_event(
    task_id: &str,
    event_kind: AgentEventKind,
    path: String,
    observed_at_millis: u64,
    seen: &mut BTreeSet<String>,
    events: &mut Vec<AgentEvent>,
) -> Result<()> {
    let key = format!("{event_kind:?}:{path}");
    if !seen.insert(key) {
        return Ok(());
    }
    let event = AgentEvent {
        schema_version: TRACE_SCHEMA_VERSION,
        task_id: task_id.to_string(),
        event_kind,
        path: Some(path),
        command_class: None,
        command_hash: None,
        touched_tests: Vec::new(),
        exit_status: None,
        status: None,
        token_estimate: None,
        elapsed_millis: None,
        observed_at_millis: Some(observed_at_millis),
        privacy: PrivacyStatus::source_free(),
    };
    validate_agent_event(&event)?;
    events.push(event);
    Ok(())
}

fn normalize_tool_name(name: &str) -> String {
    name.chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect()
}

fn is_read_tool(name: &str) -> bool {
    matches!(name, "read" | "view" | "open")
}

fn is_edit_tool(name: &str) -> bool {
    matches!(
        name,
        "edit" | "multiedit" | "write" | "create" | "applypatch"
    )
}

fn is_command_tool(name: &str) -> bool {
    matches!(
        name,
        "bash" | "shell" | "exec" | "runcommand" | "executecommand" | "terminal"
    )
}

fn classify_command_text(command: &str) -> CommandClass {
    let lower = command.to_ascii_lowercase();
    if lower.contains("test")
        || lower.contains("vitest")
        || lower.contains("pytest")
        || lower.contains("cargo test")
    {
        CommandClass::Test
    } else if lower.contains("typecheck") || lower.contains("tsc") {
        CommandClass::Typecheck
    } else if lower.contains("lint") || lower.contains("clippy") {
        CommandClass::Lint
    } else if lower.contains("build") {
        CommandClass::Build
    } else {
        CommandClass::Other
    }
}

fn trace_from_task_events(
    task: &BenchTask,
    events: &[&AgentEvent],
    agent: &str,
    variant: AgentVariant,
) -> Result<AgentTrace> {
    let mut recommended_files = Vec::new();
    let mut files_read = Vec::new();
    let mut files_edited = Vec::new();
    let mut commands = Vec::new();
    let mut status = TaskStatus::Skipped;
    let mut token_estimate = 0u64;
    let mut has_token_estimate = false;
    let mut elapsed_millis = None;

    for event in events {
        match event.event_kind {
            AgentEventKind::RecommendedFile => {
                recommended_files.push(event_path_observation(event)?);
            }
            AgentEventKind::FileRead => {
                files_read.push(event_path_observation(event)?);
            }
            AgentEventKind::FileEdit => {
                files_edited.push(event_path_observation(event)?);
            }
            AgentEventKind::Command => {
                commands.push(CommandObservation {
                    command_class: event.command_class.clone().expect("validated commandClass"),
                    command_hash: event.command_hash.clone(),
                    touched_tests: event.touched_tests.clone(),
                    exit_status: event.exit_status,
                    elapsed_millis: event.elapsed_millis,
                });
            }
            AgentEventKind::Status => {
                status = event.status.clone().expect("validated status");
            }
            AgentEventKind::Usage => {
                if let Some(tokens) = event.token_estimate {
                    token_estimate = token_estimate.saturating_add(tokens);
                    has_token_estimate = true;
                }
            }
        }
        if let Some(observed) = event.observed_at_millis {
            elapsed_millis =
                Some(elapsed_millis.map_or(observed, |current: u64| current.max(observed)));
        }
    }
    dedupe_observations(&mut recommended_files);
    dedupe_observations(&mut files_read);
    dedupe_observations(&mut files_edited);
    let expected_files = task.expected_files.iter().cloned().collect::<BTreeSet<_>>();
    let trace = AgentTrace {
        schema_version: TRACE_SCHEMA_VERSION,
        task_id: task.id.clone(),
        agent: agent.to_string(),
        variant,
        status,
        recommended_files,
        files_read,
        files_edited,
        commands,
        tool_call_count: events.len() as u32,
        token_estimate: has_token_estimate.then_some(token_estimate),
        elapsed_millis,
        time_to_first_relevant_file_millis: infer_time_to_first_relevant_file_from_events(
            events,
            &expected_files,
        ),
        privacy: PrivacyStatus::source_free(),
    };
    validate_trace(&trace)?;
    Ok(trace)
}

fn event_path_observation(event: &AgentEvent) -> Result<PathObservation> {
    let path = event.path.as_ref().context("event path")?;
    validate_safe_relative_path(path)?;
    Ok(PathObservation {
        path: path.clone(),
        path_hash: Some(format!("path:{}", stable_hash(path))),
        observed_at_millis: event.observed_at_millis,
    })
}

fn infer_time_to_first_relevant_file_from_events(
    events: &[&AgentEvent],
    expected_files: &BTreeSet<String>,
) -> Option<u64> {
    events
        .iter()
        .filter(|event| event.event_kind == AgentEventKind::FileRead)
        .filter(|event| {
            event
                .path
                .as_ref()
                .is_some_and(|path| expected_files.contains(path))
        })
        .filter_map(|event| event.observed_at_millis)
        .min()
}

fn dedupe_observations(observations: &mut Vec<PathObservation>) {
    let mut seen = BTreeSet::new();
    observations.retain(|observation| seen.insert(observation.path.clone()));
}

pub fn project_root_for_cli(path: Option<PathBuf>) -> Result<PathBuf> {
    match path {
        Some(path) => Ok(path),
        None => std::env::current_dir().context("resolve current directory"),
    }
}

pub fn validate_safe_relative_path_for_cli(path: &str) -> Result<()> {
    validate_safe_relative_path(path)
}

fn autopsy_task(task: &BenchTask, trace: &AgentTrace) -> AutopsyTask {
    let expected_files = task.expected_files.iter().cloned().collect::<BTreeSet<_>>();
    let expected_tests = task.expected_tests.iter().cloned().collect::<BTreeSet<_>>();
    let read = trace
        .files_read
        .iter()
        .map(|obs| obs.path.clone())
        .collect::<BTreeSet<_>>();
    let changed = trace
        .files_edited
        .iter()
        .map(|obs| obs.path.clone())
        .collect::<BTreeSet<_>>();
    let changed_files = changed.iter().cloned().collect::<Vec<_>>();
    let expected_files_vec = expected_files.iter().cloned().collect::<Vec<_>>();
    let missing_expected_inspections = expected_files
        .difference(&read)
        .filter(|path| !changed.contains(*path))
        .cloned()
        .collect::<Vec<_>>();
    let changed_without_read = changed.difference(&read).cloned().collect::<Vec<_>>();
    let overbroad_edits = changed
        .difference(&expected_files)
        .cloned()
        .collect::<Vec<_>>();
    let validation_gap = !validation_covered(trace, &expected_tests);
    let mut notes = Vec::new();
    if trace.status != TaskStatus::Success {
        notes.push("Task did not end in success.".to_string());
    }
    if validation_gap {
        notes.push("No successful expected validation was recorded.".to_string());
    }
    if !overbroad_edits.is_empty() {
        notes.push("Agent edited files outside the expected target set.".to_string());
    }
    if !changed_without_read.is_empty() {
        notes.push("Some edited files had no recorded read event.".to_string());
    }
    if !missing_expected_inspections.is_empty() {
        notes.push("Expected files were neither read nor edited.".to_string());
    }
    if notes.is_empty() {
        notes.push("No source-free autopsy issues detected.".to_string());
    }
    let risk = if trace.status != TaskStatus::Success
        || validation_gap
        || !overbroad_edits.is_empty()
        || !changed_without_read.is_empty()
    {
        AutopsyRisk::High
    } else if !missing_expected_inspections.is_empty() {
        AutopsyRisk::Medium
    } else {
        AutopsyRisk::Low
    };

    AutopsyTask {
        task_id: task.id.clone(),
        status: trace.status.clone(),
        risk,
        changed_files,
        expected_files: expected_files_vec,
        missing_expected_inspections,
        changed_without_read,
        overbroad_edits,
        validation_gap,
        notes,
    }
}

fn summarize_autopsy(tasks: &[AutopsyTask]) -> AutopsySummary {
    AutopsySummary {
        task_count: tasks.len(),
        failed_task_count: tasks
            .iter()
            .filter(|task| task.status != TaskStatus::Success)
            .count(),
        validation_gap_count: tasks.iter().filter(|task| task.validation_gap).count(),
        overbroad_edit_count: tasks.iter().map(|task| task.overbroad_edits.len()).sum(),
        missing_expected_inspection_count: tasks
            .iter()
            .map(|task| task.missing_expected_inspections.len())
            .sum(),
        changed_without_read_count: tasks
            .iter()
            .map(|task| task.changed_without_read.len())
            .sum(),
        high_risk_count: tasks
            .iter()
            .filter(|task| task.risk == AutopsyRisk::High)
            .count(),
    }
}

fn task_report(task: &BenchTask, trace: &AgentTrace) -> TaskReport {
    let expected_files = task.expected_files.iter().cloned().collect::<BTreeSet<_>>();
    let expected_tests = task.expected_tests.iter().cloned().collect::<BTreeSet<_>>();
    let expected_evidence = task
        .expected_files
        .iter()
        .chain(task.expected_tests.iter())
        .cloned()
        .collect::<BTreeSet<_>>();
    let recommended = trace
        .recommended_files
        .iter()
        .map(|obs| obs.path.clone())
        .collect::<BTreeSet<_>>();
    let read = trace
        .files_read
        .iter()
        .map(|obs| obs.path.clone())
        .collect::<BTreeSet<_>>();
    let edited = trace
        .files_edited
        .iter()
        .map(|obs| obs.path.clone())
        .collect::<BTreeSet<_>>();
    let relevant_files_read = read.intersection(&expected_files).count();
    let irrelevant_file_read_count = read.difference(&expected_files).count();
    let relevant_recommended_file_count = recommended.intersection(&expected_evidence).count();
    let irrelevant_recommended_file_count = recommended.difference(&expected_evidence).count();
    let expected_files_edited_count = edited.intersection(&expected_files).count();
    let validation_covered = validation_covered(trace, &expected_tests);
    let files_read_count = read.len();
    let context_precision = if files_read_count == 0 {
        0.0
    } else {
        relevant_files_read as f32 / files_read_count as f32
    };
    let edited_file_recall = if expected_files.is_empty() {
        0.0
    } else {
        expected_files_edited_count as f32 / expected_files.len() as f32
    };
    let recommendation_precision = if recommended.is_empty() {
        0.0
    } else {
        relevant_recommended_file_count as f32 / recommended.len() as f32
    };
    let recommendation_recall = if expected_evidence.is_empty() {
        0.0
    } else {
        relevant_recommended_file_count as f32 / expected_evidence.len() as f32
    };
    TaskReport {
        task_id: task.id.clone(),
        status: trace.status.clone(),
        expected_file_count: expected_files.len(),
        expected_test_count: expected_tests.len(),
        recommended_file_count: unique_count(&trace.recommended_files),
        relevant_recommended_file_count,
        irrelevant_recommended_file_count,
        recommendation_precision,
        recommendation_recall,
        files_read_count,
        relevant_files_read_count: relevant_files_read,
        irrelevant_file_read_count,
        files_edited_count: edited.len(),
        expected_files_edited_count,
        context_precision,
        edited_file_recall,
        validation_covered,
        tool_call_count: trace.tool_call_count,
        token_estimate: trace.token_estimate.unwrap_or(0),
        elapsed_millis: trace.elapsed_millis,
        time_to_first_relevant_file_millis: trace
            .time_to_first_relevant_file_millis
            .or_else(|| infer_time_to_first_relevant_file(&trace.files_read, &expected_files)),
    }
}

fn validation_covered(trace: &AgentTrace, expected_tests: &BTreeSet<String>) -> bool {
    trace.commands.iter().any(|command| {
        let class_counts = matches!(
            command.command_class,
            CommandClass::Test | CommandClass::Build | CommandClass::Typecheck
        );
        let touched_expected_test = !expected_tests.is_empty()
            && command
                .touched_tests
                .iter()
                .any(|path| expected_tests.contains(path));
        let successful = command.exit_status.is_none_or(|status| status == 0);
        successful && (touched_expected_test || class_counts)
    })
}

fn summarize(tasks: &[TaskReport]) -> ReportSummary {
    let task_count = tasks.len();
    let success_count = tasks
        .iter()
        .filter(|task| task.status == TaskStatus::Success)
        .count();
    let total_files_read = tasks
        .iter()
        .map(|task| task.files_read_count)
        .sum::<usize>();
    let total_irrelevant_file_reads = tasks
        .iter()
        .map(|task| task.irrelevant_file_read_count)
        .sum::<usize>();
    let average_recommendation_precision =
        average(tasks.iter().map(|task| task.recommendation_precision));
    let average_recommendation_recall =
        average(tasks.iter().map(|task| task.recommendation_recall));
    let average_context_precision = average(tasks.iter().map(|task| task.context_precision));
    let average_edited_file_recall = average(tasks.iter().map(|task| task.edited_file_recall));
    let validation_coverage_rate = if task_count == 0 {
        0.0
    } else {
        tasks.iter().filter(|task| task.validation_covered).count() as f32 / task_count as f32
    };
    let times = tasks
        .iter()
        .filter_map(|task| {
            task.time_to_first_relevant_file_millis
                .map(|time| time as f32)
        })
        .collect::<Vec<_>>();
    ReportSummary {
        task_count,
        success_count,
        success_rate: if task_count == 0 {
            0.0
        } else {
            success_count as f32 / task_count as f32
        },
        total_files_read,
        total_irrelevant_file_reads,
        irrelevant_read_rate: if total_files_read == 0 {
            0.0
        } else {
            total_irrelevant_file_reads as f32 / total_files_read as f32
        },
        average_recommendation_precision,
        average_recommendation_recall,
        average_context_precision,
        average_edited_file_recall,
        validation_coverage_rate,
        average_time_to_first_relevant_file_millis: (!times.is_empty())
            .then(|| average(times.into_iter())),
        total_tool_calls: tasks.iter().map(|task| task.tool_call_count).sum(),
        total_token_estimate: tasks.iter().map(|task| task.token_estimate).sum(),
    }
}

fn validate_safe_relative_path(path: &str) -> Result<()> {
    if path.trim().is_empty() {
        bail!("path is empty");
    }
    let path_obj = Path::new(path);
    if path_obj.is_absolute() {
        bail!("absolute paths are not allowed");
    }
    if path_obj
        .components()
        .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        bail!("parent traversal is not allowed");
    }
    Ok(())
}

fn unique_count(paths: &[PathObservation]) -> usize {
    paths
        .iter()
        .map(|obs| obs.path.as_str())
        .collect::<BTreeSet<_>>()
        .len()
}

fn infer_time_to_first_relevant_file(
    reads: &[PathObservation],
    expected_files: &BTreeSet<String>,
) -> Option<u64> {
    reads
        .iter()
        .filter(|obs| expected_files.contains(&obs.path))
        .filter_map(|obs| obs.observed_at_millis)
        .min()
}

fn average(values: impl Iterator<Item = f32>) -> f32 {
    let mut total = 0.0f32;
    let mut count = 0usize;
    for value in values {
        total += value;
        count += 1;
    }
    if count == 0 {
        0.0
    } else {
        total / count as f32
    }
}

fn pct(value: f32) -> f32 {
    value * 100.0
}

fn stable_hash(value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{hash:016x}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn suite_validation_rejects_unsafe_paths() {
        let mut suite = example_suite();
        suite.tasks[0]
            .expected_files
            .push("../secret.env".to_string());

        let error = validate_suite(&suite).expect_err("unsafe path should fail");
        assert!(error.to_string().contains("unsafe path"));
    }

    #[test]
    fn report_counts_irrelevant_reads_and_validation() {
        let suite = example_suite();
        let trace = AgentTrace {
            schema_version: TRACE_SCHEMA_VERSION,
            task_id: "auth-redirect-001".to_string(),
            agent: "claude-code".to_string(),
            variant: AgentVariant::CtxhelmMcp,
            status: TaskStatus::Success,
            recommended_files: vec![path("src/auth/session.ts")],
            files_read: vec![
                timed_path("README.md", 10),
                timed_path("src/auth/session.ts", 20),
                timed_path("src/auth/middleware.ts", 30),
            ],
            files_edited: vec![path("src/auth/session.ts")],
            commands: vec![CommandObservation {
                command_class: CommandClass::Test,
                command_hash: Some("hash:test".to_string()),
                touched_tests: vec!["tests/auth/session.test.ts".to_string()],
                exit_status: Some(0),
                elapsed_millis: Some(1000),
            }],
            tool_call_count: 6,
            token_estimate: Some(1200),
            elapsed_millis: Some(2000),
            time_to_first_relevant_file_millis: None,
            privacy: PrivacyStatus::source_free(),
        };

        let report = build_report(&suite, &[trace]).expect("report");
        assert_eq!(report.summary.success_count, 1);
        assert_eq!(report.summary.total_irrelevant_file_reads, 1);
        assert_eq!(report.tasks[0].relevant_files_read_count, 2);
        assert_eq!(report.tasks[0].time_to_first_relevant_file_millis, Some(20));
        assert!(report.tasks[0].validation_covered);
        assert_eq!(report.summary.total_tool_calls, 6);
    }

    #[test]
    fn report_counts_successful_validation_command_class_without_touched_tests() {
        let suite = example_suite();
        let trace = AgentTrace {
            schema_version: TRACE_SCHEMA_VERSION,
            task_id: "auth-redirect-001".to_string(),
            agent: "local-script".to_string(),
            variant: AgentVariant::Native,
            status: TaskStatus::Success,
            recommended_files: Vec::new(),
            files_read: vec![timed_path("src/auth/session.ts", 20)],
            files_edited: vec![path("src/auth/session.ts")],
            commands: vec![CommandObservation {
                command_class: CommandClass::Test,
                command_hash: Some("cmd:targeted".to_string()),
                touched_tests: Vec::new(),
                exit_status: Some(0),
                elapsed_millis: Some(1000),
            }],
            tool_call_count: 3,
            token_estimate: None,
            elapsed_millis: Some(1200),
            time_to_first_relevant_file_millis: None,
            privacy: PrivacyStatus::source_free(),
        };

        let report = build_report(&suite, &[trace]).expect("report");
        assert!(report.tasks[0].validation_covered);
        assert_eq!(report.summary.validation_coverage_rate, 1.0);
    }

    #[test]
    fn ctxhelm_prepare_json_becomes_source_free_recommendation_trace() {
        let suite = example_suite();
        let task = &suite.tasks[0];
        let json = r#"{
          "taskId": "task-1",
          "targetFiles": [
            {"path": "src/auth/session.ts"},
            {"path": "src/auth/middleware.ts"}
          ],
          "relatedTests": [
            {"path": "tests/auth/session.test.ts"}
          ]
        }"#;

        let trace = trace_from_ctxhelm_prepare_json(
            task,
            json,
            "ctxhelm",
            AgentVariant::CtxhelmPlan,
            Some(42),
        )
        .expect("ctxhelm trace");
        assert_eq!(trace.task_id, "auth-redirect-001");
        assert_eq!(trace.status, TaskStatus::Skipped);
        assert_eq!(trace.recommended_files.len(), 3);
        assert!(trace.privacy.source_free);

        let report = build_report(&suite, &[trace]).expect("report");
        assert_eq!(report.tasks[0].relevant_recommended_file_count, 3);
        assert_eq!(report.tasks[0].recommendation_precision, 1.0);
        assert_eq!(report.tasks[0].recommendation_recall, 1.0);
        assert_eq!(report.summary.average_recommendation_recall, 1.0);
    }

    #[test]
    fn ctxhelm_prepare_json_rejects_unsafe_recommended_path() {
        let suite = example_suite();
        let error = trace_from_ctxhelm_prepare_json(
            &suite.tasks[0],
            r#"{"targetFiles":[{"path":"../secret.env"}]}"#,
            "ctxhelm",
            AgentVariant::CtxhelmPlan,
            None,
        )
        .expect_err("unsafe path should fail");
        assert!(error.to_string().contains("ctxhelm path"));
    }

    #[test]
    fn source_free_agent_events_become_claude_trace() {
        let suite = example_suite();
        let events = vec![
            event(
                "auth-redirect-001",
                AgentEventKind::RecommendedFile,
                Some("src/auth/session.ts"),
                None,
                None,
                Some(50),
            ),
            event(
                "auth-redirect-001",
                AgentEventKind::FileRead,
                Some("README.md"),
                None,
                None,
                Some(100),
            ),
            event(
                "auth-redirect-001",
                AgentEventKind::FileRead,
                Some("src/auth/session.ts"),
                None,
                None,
                Some(200),
            ),
            event(
                "auth-redirect-001",
                AgentEventKind::FileEdit,
                Some("src/auth/session.ts"),
                None,
                None,
                Some(300),
            ),
            AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: "auth-redirect-001".to_string(),
                event_kind: AgentEventKind::Command,
                path: None,
                command_class: Some(CommandClass::Test),
                command_hash: Some("cmd:targeted".to_string()),
                touched_tests: vec!["tests/auth/session.test.ts".to_string()],
                exit_status: Some(0),
                status: None,
                token_estimate: None,
                elapsed_millis: Some(1200),
                observed_at_millis: Some(400),
                privacy: PrivacyStatus::source_free(),
            },
            AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: "auth-redirect-001".to_string(),
                event_kind: AgentEventKind::Usage,
                path: None,
                command_class: None,
                command_hash: None,
                touched_tests: Vec::new(),
                exit_status: None,
                status: None,
                token_estimate: Some(2400),
                elapsed_millis: None,
                observed_at_millis: Some(450),
                privacy: PrivacyStatus::source_free(),
            },
            AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: "auth-redirect-001".to_string(),
                event_kind: AgentEventKind::Status,
                path: None,
                command_class: None,
                command_hash: None,
                touched_tests: Vec::new(),
                exit_status: None,
                status: Some(TaskStatus::Success),
                token_estimate: None,
                elapsed_millis: None,
                observed_at_millis: Some(500),
                privacy: PrivacyStatus::source_free(),
            },
        ];

        let traces =
            traces_from_agent_events(&suite, &events, "claude-code", AgentVariant::CtxhelmMcp)
                .expect("traces");
        assert_eq!(traces.len(), 1);
        let trace = &traces[0];
        assert_eq!(trace.status, TaskStatus::Success);
        assert_eq!(trace.files_read.len(), 2);
        assert_eq!(trace.files_edited.len(), 1);
        assert_eq!(trace.commands.len(), 1);
        assert_eq!(trace.token_estimate, Some(2400));
        assert_eq!(trace.time_to_first_relevant_file_millis, Some(200));

        let report = build_report(&suite, &traces).expect("report");
        assert_eq!(report.summary.success_rate, 1.0);
        assert_eq!(report.summary.validation_coverage_rate, 1.0);
        assert_eq!(report.tasks[0].irrelevant_file_read_count, 1);
    }

    #[test]
    fn stream_jsonl_becomes_source_free_events_without_raw_commands() {
        let temp = tempfile::tempdir().expect("tempdir");
        let root = temp.path();
        let absolute = root.join("src/auth/session.ts");
        let jsonl = format!(
            "{}\n{}\n{}\n{}\n",
            serde_json::json!({
                "type": "tool_use",
                "name": "Read",
                "input": {"file_path": absolute}
            }),
            serde_json::json!({
                "type": "tool_use",
                "name": "Edit",
                "input": {"file_path": "src/auth/session.ts"}
            }),
            serde_json::json!({
                "type": "tool_call",
                "tool_name": "Bash",
                "input": {
                    "command": "pnpm vitest run tests/auth/session.test.ts",
                    "exit_status": 0
                }
            }),
            serde_json::json!({
                "eventKind": "recommended_file",
                "path": "src/auth/middleware.ts"
            })
        );

        let events = events_from_agent_stream_jsonl(
            "auth-redirect-001",
            &jsonl,
            Some(root),
            &["tests/auth/session.test.ts".to_string()],
        )
        .expect("stream events");
        assert_eq!(events.len(), 4);
        assert!(events.iter().any(|event| {
            event.event_kind == AgentEventKind::FileRead
                && event.path.as_deref() == Some("src/auth/session.ts")
        }));
        assert!(events.iter().any(|event| {
            event.event_kind == AgentEventKind::FileEdit
                && event.path.as_deref() == Some("src/auth/session.ts")
        }));
        let command = events
            .iter()
            .find(|event| event.event_kind == AgentEventKind::Command)
            .expect("command");
        assert_eq!(command.command_class, Some(CommandClass::Test));
        assert_eq!(command.exit_status, Some(0));
        assert_eq!(
            command.touched_tests,
            vec!["tests/auth/session.test.ts".to_string()]
        );
        assert!(command
            .command_hash
            .as_deref()
            .is_some_and(|hash| hash.starts_with("cmd:")));
        assert!(serde_json::to_string(&events)
            .expect("json")
            .contains("session.test.ts"));
        assert!(!serde_json::to_string(&events)
            .expect("json")
            .contains("pnpm vitest run"));
    }

    #[test]
    fn agent_event_validation_rejects_raw_transcript_flag() {
        let mut unsafe_event = event(
            "auth-redirect-001",
            AgentEventKind::FileRead,
            Some("src/auth/session.ts"),
            None,
            None,
            Some(1),
        );
        unsafe_event.privacy.raw_transcript_logged = true;

        let error = validate_agent_event(&unsafe_event).expect_err("unsafe event should fail");
        assert!(error.to_string().contains("not source-free"));
    }

    #[test]
    fn compare_reports_reports_directional_deltas() {
        let suite = example_suite();
        let base = build_report(
            &suite,
            &[trace_with_reads(
                AgentVariant::Native,
                TaskStatus::Failure,
                vec!["README.md", "docs/auth.md"],
            )],
        )
        .expect("base report");
        let head = build_report(
            &suite,
            &[trace_with_reads(
                AgentVariant::CtxhelmMcp,
                TaskStatus::Success,
                vec!["src/auth/session.ts", "src/auth/middleware.ts"],
            )],
        )
        .expect("head report");

        let compare = compare_reports(&base, &head);
        assert!(compare.success_rate_delta > 0.0);
        assert!(compare.irrelevant_read_rate_delta < 0.0);
        assert!(render_markdown_compare(&compare).contains("Task success rate"));
    }

    #[test]
    fn autopsy_reports_overbroad_edits_and_validation_gaps() {
        let suite = example_suite();
        let trace = AgentTrace {
            schema_version: TRACE_SCHEMA_VERSION,
            task_id: "auth-redirect-001".to_string(),
            agent: "codex".to_string(),
            variant: AgentVariant::Native,
            status: TaskStatus::Failure,
            recommended_files: Vec::new(),
            files_read: vec![path("src/auth/session.ts")],
            files_edited: vec![
                path("src/auth/session.ts"),
                path("README.md"),
                path("src/auth/middleware.ts"),
            ],
            commands: Vec::new(),
            tool_call_count: 4,
            token_estimate: None,
            elapsed_millis: Some(1000),
            time_to_first_relevant_file_millis: None,
            privacy: PrivacyStatus::source_free(),
        };

        let autopsy = build_autopsy(&suite, &[trace]).expect("autopsy");
        assert_eq!(autopsy.summary.task_count, 1);
        assert_eq!(autopsy.summary.failed_task_count, 1);
        assert_eq!(autopsy.summary.validation_gap_count, 1);
        assert_eq!(autopsy.summary.overbroad_edit_count, 1);
        assert_eq!(autopsy.summary.changed_without_read_count, 2);
        assert_eq!(autopsy.summary.high_risk_count, 1);
        assert_eq!(autopsy.tasks[0].risk, AutopsyRisk::High);
        assert_eq!(autopsy.tasks[0].overbroad_edits, vec!["README.md"]);
        assert!(autopsy.tasks[0]
            .changed_without_read
            .contains(&"src/auth/middleware.ts".to_string()));
        assert!(render_markdown_autopsy(&autopsy).contains("Overbroad edits"));
    }

    #[test]
    fn html_dashboard_escapes_report_content() {
        let suite = example_suite();
        let mut report = build_report(
            &suite,
            &[trace_with_reads(
                AgentVariant::Native,
                TaskStatus::Failure,
                vec!["README.md", "docs/auth.md"],
            )],
        )
        .expect("report");
        report.agent = "claude<script>".to_string();
        report.suite_name = "suite & private".to_string();

        let html = render_html_dashboard(&[report]).expect("dashboard");
        assert!(html.contains("HelmBench Dashboard"));
        assert!(html.contains("claude&lt;script&gt;"));
        assert!(html.contains("suite &amp; private"));
        assert!(!html.contains("claude<script>"));
        assert!(html.contains("raw source"));
    }

    fn path(path: &str) -> PathObservation {
        PathObservation {
            path: path.to_string(),
            path_hash: None,
            observed_at_millis: None,
        }
    }

    fn timed_path(path: &str, observed_at_millis: u64) -> PathObservation {
        PathObservation {
            path: path.to_string(),
            path_hash: None,
            observed_at_millis: Some(observed_at_millis),
        }
    }

    fn trace_with_reads(variant: AgentVariant, status: TaskStatus, reads: Vec<&str>) -> AgentTrace {
        AgentTrace {
            schema_version: TRACE_SCHEMA_VERSION,
            task_id: "auth-redirect-001".to_string(),
            agent: "claude-code".to_string(),
            variant,
            status,
            recommended_files: Vec::new(),
            files_read: reads.into_iter().map(path).collect(),
            files_edited: Vec::new(),
            commands: Vec::new(),
            tool_call_count: 1,
            token_estimate: Some(100),
            elapsed_millis: None,
            time_to_first_relevant_file_millis: None,
            privacy: PrivacyStatus::source_free(),
        }
    }

    fn event(
        task_id: &str,
        event_kind: AgentEventKind,
        path: Option<&str>,
        command_class: Option<CommandClass>,
        status: Option<TaskStatus>,
        observed_at_millis: Option<u64>,
    ) -> AgentEvent {
        AgentEvent {
            schema_version: TRACE_SCHEMA_VERSION,
            task_id: task_id.to_string(),
            event_kind,
            path: path.map(str::to_string),
            command_class,
            command_hash: None,
            touched_tests: Vec::new(),
            exit_status: None,
            status,
            token_estimate: None,
            elapsed_millis: None,
            observed_at_millis,
            privacy: PrivacyStatus::source_free(),
        }
    }
}
