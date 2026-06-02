use anyhow::{bail, Context, Result};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};

pub const SUITE_SCHEMA_VERSION: u32 = 1;
pub const TRACE_SCHEMA_VERSION: u32 = 1;
pub const REPORT_SCHEMA_VERSION: u32 = 1;

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

pub fn write_json<T: Serialize>(value: &T, path: &Path) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    let json = serde_json::to_string_pretty(value)?;
    fs::write(path, json).with_context(|| format!("write {}", path.display()))
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
    let validation_covered = validation_covered(task, trace, &expected_tests);
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

fn validation_covered(
    task: &BenchTask,
    trace: &AgentTrace,
    expected_tests: &BTreeSet<String>,
) -> bool {
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
        successful && (touched_expected_test || (class_counts && task.success_command.is_none()))
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
}
