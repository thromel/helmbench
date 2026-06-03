use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use helmbench::{
    build_autopsy, build_benchmark_summary, build_diff_autopsy, build_report, compare_reports,
    evaluate_quality_gate, events_from_agent_stream_jsonl, example_suite, load_agent_events,
    load_suite, load_traces, project_root_for_cli, read_benchmark_summary, read_report,
    render_html_dashboard, render_markdown_autopsy, render_markdown_benchmark_summary,
    render_markdown_compare, render_markdown_diff_autopsy, render_markdown_quality_gate,
    render_markdown_report, trace_from_ctxhelm_prepare_json, traces_from_agent_events,
    validate_agent_event, validate_comparable_reports, validate_suite, write_json, AgentEvent,
    AgentEventKind, AgentVariant, BenchmarkRunSummary, BenchmarkSummaryReport, CommandClass,
    PrivacyStatus, QualityGateConfig, TaskStatus, TRACE_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, BTreeSet};
use std::io::Read;
use std::path::{Path, PathBuf};
use std::process::{Command as ProcessCommand, Stdio};
use std::time::{Duration, Instant};

const RUN_MATRIX_MANIFEST_SCHEMA_VERSION: u32 = 6;
const RUN_MATRIX_PRIVACY_REPORT_SCHEMA_VERSION: u32 = 1;
const MATRIX_HISTORY_SCHEMA_VERSION: u32 = 2;
const SUITE_HEALTH_SCHEMA_VERSION: u32 = 1;
const EVIDENCE_BUNDLE_SCHEMA_VERSION: u32 = 1;
const DOCTOR_REPORT_SCHEMA_VERSION: u32 = 1;
const LAUNCH_READINESS_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Parser)]
#[command(
    name = "helmbench",
    version,
    about = "Source-free benchmark and observability harness for AI coding agents"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Write an example source-free task suite JSON.
    InitSuite {
        #[arg(long, default_value = "suites/example-auth-bugs.json")]
        out: PathBuf,
    },
    /// Write a published JSON Schema contract.
    Schema {
        #[arg(long, value_enum)]
        kind: Option<SchemaKind>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        all: bool,
        #[arg(long)]
        out_dir: Option<PathBuf>,
    },
    /// Create a tiny reproducible benchmark repo and matching suite.
    InitDemoRepo {
        #[arg(long, default_value = ".helmbench/demo-repo")]
        repo_out: PathBuf,
        #[arg(long, default_value = "suites/demo-tiny-repo.json")]
        suite_out: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Run the full deterministic demo pipeline and write source-free artifacts.
    DemoRun {
        #[arg(long, default_value = ".helmbench/demo-run")]
        out_dir: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Validate a run-matrix JSON config without executing agents.
    ValidateMatrix {
        #[arg(long)]
        config: PathBuf,
    },
    /// Run a baseline and one or more local adapter variants, then write comparison artifacts.
    RunMatrix {
        #[arg(long)]
        config: Option<PathBuf>,
        #[arg(long)]
        suite: Option<PathBuf>,
        #[arg(long)]
        repo: Option<PathBuf>,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        /// Run spec: name=<id>,agent=<agent>,variant=<native|native_search|ctxhelm_plan|ctxhelm_mcp|ctxhelm_pack|other>[,command=<adapter command>|preset=<claude-code|codex>]
        #[arg(long)]
        baseline: Option<String>,
        /// Repeated run spec with the same format as --baseline.
        #[arg(long)]
        head: Vec<String>,
        #[arg(long)]
        setup_command: Vec<String>,
        #[arg(long)]
        force: bool,
        #[arg(long)]
        keep_workdirs: bool,
        #[arg(long)]
        fail_on_regression: bool,
        #[arg(long)]
        min_task_count: Option<usize>,
        #[arg(long)]
        max_average_time_to_first_relevant_file_millis_delta: Option<f32>,
        #[arg(long)]
        max_total_tool_calls_delta: Option<i64>,
        #[arg(long)]
        max_total_token_estimate_delta: Option<i64>,
        #[arg(long)]
        max_tool_calls_per_success_delta: Option<f32>,
        #[arg(long)]
        max_token_estimate_per_success_delta: Option<f32>,
        #[arg(long, default_value_t = 1)]
        health_min_commits: u64,
        #[arg(long)]
        allow_dirty_health: bool,
        #[arg(long)]
        health_check_success_commands: bool,
        #[arg(long)]
        health_fail_fast_success_commands: bool,
        #[arg(long)]
        health_require_setup_commands: bool,
    },
    /// Compare verified run-matrix outputs across time.
    MatrixHistory {
        #[arg(long, required = true)]
        matrix: Vec<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = MatrixHistoryFormat::Markdown)]
        format: MatrixHistoryFormat,
    },
    /// Generate a source-free suite from a known public repository fixture.
    InitPublicSuite {
        #[arg(long, value_enum)]
        preset: PublicSuitePreset,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        suite_out: Option<PathBuf>,
        #[arg(long)]
        health_out: Option<PathBuf>,
        #[arg(long, default_value_t = 1000)]
        min_commits: u64,
        #[arg(long)]
        force: bool,
    },
    /// Generate a repeatable public-repo run-matrix config for a real agent proof.
    InitPublicMatrix {
        #[arg(long, value_enum)]
        preset: PublicSuitePreset,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        suite: Option<PathBuf>,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long)]
        out_dir: Option<PathBuf>,
        #[arg(long)]
        health_out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = AdapterPreset::ClaudeCode)]
        agent_preset: AdapterPreset,
        #[arg(long)]
        agent_bin: Option<PathBuf>,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        agent_arg: Vec<String>,
        #[arg(long)]
        dangerously_skip_permissions: bool,
        #[arg(long)]
        dangerously_bypass_approvals_and_sandbox: bool,
        #[arg(long, default_value = "ctxhelm")]
        ctxhelm_bin: PathBuf,
        #[arg(long, default_value = "bug-fix")]
        mode: String,
        #[arg(long)]
        target_agent: Option<String>,
        #[arg(long)]
        pack: bool,
        #[arg(long)]
        fail_on_regression: bool,
        #[arg(long, default_value_t = 1000)]
        health_min_commits: u64,
        #[arg(long)]
        allow_dirty_health: bool,
        #[arg(long)]
        health_check_success_commands: bool,
        #[arg(long)]
        health_fail_fast_success_commands: bool,
        #[arg(long)]
        health_require_setup_commands: bool,
        #[arg(long)]
        force: bool,
    },
    /// Check that a source-free task suite is usable against a local git repo.
    SuiteHealth {
        #[arg(long, value_enum)]
        preset: Option<PublicSuitePreset>,
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
        #[arg(long, default_value_t = 1)]
        min_commits: u64,
        #[arg(long)]
        allow_dirty: bool,
        #[arg(long)]
        check_success_commands: bool,
        #[arg(long)]
        fail_fast_success_commands: bool,
        #[arg(long)]
        require_setup_commands: bool,
    },
    /// Validate a HelmBench suite contract.
    ValidateSuite { suite: PathBuf },
    /// Build a run report from source-free trace JSON files.
    Run {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        trace_dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Json)]
        format: OutputFormat,
    },
    /// Generate source-free traces by calling ctxhelm prepare-task for each suite task.
    CtxhelmTrace {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, default_value = "ctxhelm")]
        ctxhelm_bin: PathBuf,
        #[arg(long, default_value = "explain")]
        mode: String,
        #[arg(long, default_value = "generic")]
        target_agent: String,
        #[arg(long)]
        semantic: bool,
        #[arg(long, default_value = "traces/ctxhelm-plan")]
        out_dir: PathBuf,
    },
    /// Convert sanitized Claude Code event JSONL into source-free HelmBench traces.
    ClaudeTrace {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        events: PathBuf,
        #[arg(long, value_enum, default_value_t = TraceVariant::Native)]
        variant: TraceVariant,
        #[arg(long, default_value = "traces/claude-code")]
        out_dir: PathBuf,
    },
    /// Convert a structured agent JSONL stream into source-free HelmBench traces.
    StreamTrace {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        stream: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long, default_value = "agent-stream")]
        agent: String,
        #[arg(long, value_enum, default_value_t = TraceVariant::Native)]
        variant: TraceVariant,
        #[arg(long)]
        repo_root: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = CliTaskStatus::Skipped)]
        status: CliTaskStatus,
        #[arg(long, default_value = "traces/agent-stream")]
        out_dir: PathBuf,
    },
    /// Run a source-free local adapter command in isolated per-task repo clones.
    LocalRun {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, default_value = ".helmbench/workdirs")]
        work_dir: PathBuf,
        #[arg(long, default_value = "traces/local-run")]
        out_dir: PathBuf,
        #[arg(long, default_value = "local-script")]
        agent: String,
        #[arg(long, value_enum, default_value_t = TraceVariant::Native)]
        variant: TraceVariant,
        #[arg(long)]
        setup_command: Vec<String>,
        #[arg(long)]
        adapter_command: Option<String>,
        #[arg(long)]
        capture_stream: bool,
        #[arg(long)]
        keep_workdirs: bool,
    },
    /// Run ctxhelm recommendations before an isolated local adapter command.
    CtxhelmRun {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, default_value = ".helmbench/workdirs")]
        work_dir: PathBuf,
        #[arg(long, default_value = "traces/ctxhelm-run")]
        out_dir: PathBuf,
        #[arg(long, default_value = "ctxhelm")]
        ctxhelm_bin: PathBuf,
        #[arg(long, default_value = "bug-fix")]
        mode: String,
        #[arg(long, default_value = "generic")]
        target_agent: String,
        #[arg(long)]
        semantic: bool,
        #[arg(long)]
        semantic_provider: Option<String>,
        #[arg(long)]
        semantic_model: Option<String>,
        #[arg(long)]
        semantic_dimensions: Option<u16>,
        #[arg(long)]
        pack: bool,
        #[arg(long, default_value = "brief")]
        pack_budget: String,
        #[arg(long, default_value = "ctxhelm-local")]
        agent: String,
        #[arg(long, value_enum, default_value_t = TraceVariant::CtxhelmMcp)]
        variant: TraceVariant,
        #[arg(long)]
        adapter_command: Option<String>,
        #[arg(long)]
        capture_stream: bool,
        #[arg(long)]
        keep_workdirs: bool,
    },
    /// Run Claude Code non-interactively through the isolated local runner.
    ClaudeRun {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, default_value = ".helmbench/workdirs")]
        work_dir: PathBuf,
        #[arg(long, default_value = "traces/claude-run")]
        out_dir: PathBuf,
        #[arg(long, default_value = "claude")]
        claude_bin: PathBuf,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        claude_arg: Vec<String>,
        #[arg(long)]
        dangerously_skip_permissions: bool,
        #[arg(long)]
        capture_stream: bool,
        #[arg(long)]
        keep_workdirs: bool,
    },
    /// Run Codex non-interactively through the isolated local runner.
    CodexRun {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, default_value = ".helmbench/workdirs")]
        work_dir: PathBuf,
        #[arg(long, default_value = "traces/codex-run")]
        out_dir: PathBuf,
        #[arg(long, default_value = "codex")]
        codex_bin: PathBuf,
        #[arg(long)]
        model: Option<String>,
        #[arg(long)]
        codex_arg: Vec<String>,
        #[arg(long)]
        dangerously_bypass_approvals_and_sandbox: bool,
        #[arg(long)]
        capture_stream: bool,
        #[arg(long)]
        keep_workdirs: bool,
    },
    /// Append one validated source-free event to a JSONL file.
    RecordEvent {
        #[arg(long)]
        events: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long, value_enum)]
        event_kind: CliEventKind,
        #[arg(long)]
        path: Option<String>,
        #[arg(long, value_enum)]
        command_class: Option<CliCommandClass>,
        #[arg(long)]
        command_hash: Option<String>,
        #[arg(long)]
        touched_test: Vec<String>,
        #[arg(long)]
        exit_status: Option<i32>,
        #[arg(long, value_enum)]
        status: Option<CliTaskStatus>,
        #[arg(long)]
        token_estimate: Option<u64>,
        #[arg(long)]
        elapsed_millis: Option<u64>,
        #[arg(long)]
        observed_at_millis: Option<u64>,
    },
    /// Compare two source-free run reports.
    Compare {
        #[arg(long)]
        base: PathBuf,
        #[arg(long)]
        head: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        #[arg(long)]
        out: Option<PathBuf>,
    },
    /// Summarize one baseline against one or more source-free run reports.
    BenchmarkSummary {
        #[arg(long)]
        base: PathBuf,
        #[arg(long, required = true)]
        head: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Summarize whether source-free artifacts are launch-ready or still smoke proof.
    LaunchReadiness {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        base_report: PathBuf,
        #[arg(long, required = true)]
        head_report: Vec<PathBuf>,
        #[arg(long)]
        health: Vec<PathBuf>,
        #[arg(long)]
        matrix: Vec<PathBuf>,
        #[arg(long, default_value_t = helmbench::MIN_RECOMMENDED_BENCHMARK_TASKS)]
        min_task_count: usize,
        #[arg(long, default_value_t = 1)]
        min_real_agent_rows: usize,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Build a source-free evidence bundle from suite, health, and run reports.
    EvidenceBundle {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        health: Option<PathBuf>,
        #[arg(long)]
        base_report: PathBuf,
        #[arg(long, required = true)]
        head_report: Vec<PathBuf>,
        #[arg(long)]
        out_dir: PathBuf,
        #[arg(long)]
        force: bool,
    },
    /// Verify a source-free evidence bundle manifest and artifact hashes.
    VerifyBundle {
        #[arg(long)]
        bundle: PathBuf,
    },
    /// Verify a run-matrix output directory and nested evidence bundle.
    VerifyMatrix {
        #[arg(long)]
        matrix: PathBuf,
    },
    /// Fail if a benchmark summary violates source-free quality thresholds.
    QualityGate {
        #[arg(long)]
        summary: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        #[arg(long)]
        min_task_count: Option<usize>,
        #[arg(long, default_value_t = 0.0)]
        min_success_rate_delta: f32,
        #[arg(long, default_value_t = 0.0)]
        min_validation_coverage_rate_delta: f32,
        #[arg(long, default_value_t = 0.0)]
        max_irrelevant_read_rate_delta: f32,
        #[arg(long, default_value_t = 0.0)]
        min_recommendation_recall_delta: f32,
        #[arg(long, default_value_t = 0.0)]
        min_context_precision_delta: f32,
        #[arg(long, default_value_t = 0.0)]
        min_edited_file_recall_delta: f32,
        #[arg(long)]
        max_average_time_to_first_relevant_file_millis_delta: Option<f32>,
        #[arg(long)]
        max_total_tool_calls_delta: Option<i64>,
        #[arg(long)]
        max_total_token_estimate_delta: Option<i64>,
        #[arg(long)]
        max_tool_calls_per_success_delta: Option<f32>,
        #[arg(long)]
        max_token_estimate_per_success_delta: Option<f32>,
    },
    /// Diagnose source-free agent behavior from task traces.
    Autopsy {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        trace_dir: PathBuf,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Diagnose a source-free git diff against one benchmark task.
    DiffAutopsy {
        #[arg(long)]
        suite: PathBuf,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long)]
        task_id: String,
        #[arg(long)]
        base_ref: Option<String>,
        #[arg(long)]
        head_ref: Option<String>,
        #[arg(long)]
        pr: Option<String>,
        #[arg(long)]
        github_repo: Option<String>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
    },
    /// Render a static source-free HTML dashboard from run reports.
    Dashboard {
        #[arg(long, required = true)]
        report: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
    },
    /// Validate local CLI and show supported variants.
    Doctor {
        #[arg(long)]
        repo: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
        #[arg(long)]
        out: Option<PathBuf>,
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum MatrixHistoryFormat {
    Json,
    Markdown,
    Html,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum SchemaKind {
    TaskSuite,
    AgentTrace,
    AgentEvent,
    RunReport,
    CompareReport,
    BenchmarkSummary,
    LaunchReadiness,
    QualityGate,
    RunMatrixConfig,
    MatrixHistory,
    DoctorReport,
    Autopsy,
    DiffAutopsy,
    SuiteHealth,
    EvidenceBundle,
    RunMatrixManifest,
    RunMatrixPrivacyReport,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TraceVariant {
    Native,
    NativeSearch,
    CtxhelmMcp,
    CtxhelmPack,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PublicSuitePreset {
    RefactoringMiner,
    Flask,
    Ripgrep,
    Express,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliEventKind {
    RecommendedFile,
    FileRead,
    FileEdit,
    Command,
    Status,
    Usage,
}

impl From<CliEventKind> for AgentEventKind {
    fn from(value: CliEventKind) -> Self {
        match value {
            CliEventKind::RecommendedFile => AgentEventKind::RecommendedFile,
            CliEventKind::FileRead => AgentEventKind::FileRead,
            CliEventKind::FileEdit => AgentEventKind::FileEdit,
            CliEventKind::Command => AgentEventKind::Command,
            CliEventKind::Status => AgentEventKind::Status,
            CliEventKind::Usage => AgentEventKind::Usage,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliCommandClass {
    Test,
    Build,
    Lint,
    Typecheck,
    Other,
}

impl From<CliCommandClass> for CommandClass {
    fn from(value: CliCommandClass) -> Self {
        match value {
            CliCommandClass::Test => CommandClass::Test,
            CliCommandClass::Build => CommandClass::Build,
            CliCommandClass::Lint => CommandClass::Lint,
            CliCommandClass::Typecheck => CommandClass::Typecheck,
            CliCommandClass::Other => CommandClass::Other,
        }
    }
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum CliTaskStatus {
    Success,
    Failure,
    Skipped,
}

impl From<CliTaskStatus> for TaskStatus {
    fn from(value: CliTaskStatus) -> Self {
        match value {
            CliTaskStatus::Success => TaskStatus::Success,
            CliTaskStatus::Failure => TaskStatus::Failure,
            CliTaskStatus::Skipped => TaskStatus::Skipped,
        }
    }
}

impl From<TraceVariant> for AgentVariant {
    fn from(value: TraceVariant) -> Self {
        match value {
            TraceVariant::Native => AgentVariant::Native,
            TraceVariant::NativeSearch => AgentVariant::NativeSearch,
            TraceVariant::CtxhelmMcp => AgentVariant::CtxhelmMcp,
            TraceVariant::CtxhelmPack => AgentVariant::CtxhelmPack,
        }
    }
}

fn main() -> Result<()> {
    let cli = Cli::parse();
    match cli.command {
        Command::InitSuite { out } => {
            let suite = example_suite();
            validate_suite(&suite)?;
            write_json(&suite, &out)?;
            println!("wrote {}", out.display());
        }
        Command::Schema {
            kind,
            out,
            all,
            out_dir,
        } => {
            if all {
                if kind.is_some() || out.is_some() {
                    anyhow::bail!("schema --all cannot be combined with --kind or --out");
                }
                let out_dir = out_dir.context("schema --all requires --out-dir")?;
                write_all_schema_contracts(&out_dir)?;
                println!("wrote {}", out_dir.display());
            } else {
                if out_dir.is_some() {
                    anyhow::bail!("schema --out-dir requires --all");
                }
                let kind = kind.context("schema requires --kind or --all")?;
                let schema = schema_contract(kind);
                match out {
                    Some(path) => {
                        write_text(schema, &path)?;
                        println!("wrote {}", path.display());
                    }
                    None => println!("{schema}"),
                }
            }
        }
        Command::InitDemoRepo {
            repo_out,
            suite_out,
            force,
        } => {
            init_demo_repo(&repo_out, &suite_out, force)?;
            println!("wrote {}", repo_out.display());
            println!("wrote {}", suite_out.display());
        }
        Command::DemoRun { out_dir, force } => {
            run_demo_pipeline(&out_dir, force)?;
            println!("wrote {}", out_dir.display());
        }
        Command::ValidateMatrix { config } => {
            let request = build_run_matrix_request(
                Some(&config),
                None,
                None,
                None,
                None,
                Vec::new(),
                Vec::new(),
                false,
                false,
                false,
                1,
                false,
                false,
                false,
                false,
                None,
                None,
                None,
                None,
                None,
                None,
            )?;
            let suite = validate_run_matrix_request(&request)?;
            println!(
                "matrix config `{}` is valid: suite `{}` has {} task(s), {} run(s), repo `{}` is a git checkout",
                config.display(),
                suite.name,
                suite.tasks.len(),
                request.heads.len() + 1,
                request.repo.display()
            );
        }
        Command::RunMatrix {
            config,
            suite,
            repo,
            out_dir,
            baseline,
            head,
            setup_command,
            force,
            keep_workdirs,
            fail_on_regression,
            min_task_count,
            max_average_time_to_first_relevant_file_millis_delta,
            max_total_tool_calls_delta,
            max_total_token_estimate_delta,
            max_tool_calls_per_success_delta,
            max_token_estimate_per_success_delta,
            health_min_commits,
            allow_dirty_health,
            health_check_success_commands,
            health_fail_fast_success_commands,
            health_require_setup_commands,
        } => {
            let request = build_run_matrix_request(
                config.as_deref(),
                suite,
                repo,
                out_dir,
                baseline,
                head,
                setup_command,
                force,
                keep_workdirs,
                fail_on_regression,
                health_min_commits,
                allow_dirty_health,
                health_check_success_commands,
                health_fail_fast_success_commands,
                health_require_setup_commands,
                min_task_count,
                max_average_time_to_first_relevant_file_millis_delta,
                max_total_tool_calls_delta,
                max_total_token_estimate_delta,
                max_tool_calls_per_success_delta,
                max_token_estimate_per_success_delta,
            )?;
            run_matrix(&request)?;
            println!("wrote {}", request.out_dir.display());
        }
        Command::MatrixHistory {
            matrix,
            out,
            format,
        } => {
            let history = build_matrix_history_report(&matrix)?;
            let rendered = match format {
                MatrixHistoryFormat::Json => serde_json::to_string_pretty(&history)?,
                MatrixHistoryFormat::Markdown => render_markdown_matrix_history(&history),
                MatrixHistoryFormat::Html => render_html_matrix_history(&history),
            };
            if let Some(out) = out {
                write_text(&rendered, &out)?;
                println!("wrote {}", out.display());
            } else {
                println!("{rendered}");
            }
        }
        Command::InitPublicSuite {
            preset,
            repo,
            suite_out,
            health_out,
            min_commits,
            force,
        } => {
            let suite_out = suite_out.unwrap_or_else(|| default_public_suite_out(preset));
            let health_out = health_out.unwrap_or_else(|| default_public_health_out(preset));
            init_public_suite(preset, &repo, &suite_out, &health_out, min_commits, force)?;
            println!("wrote {}", suite_out.display());
            println!("wrote {}", health_out.display());
        }
        Command::InitPublicMatrix {
            preset,
            repo,
            suite,
            out,
            out_dir,
            health_out,
            agent_preset,
            agent_bin,
            model,
            agent_arg,
            dangerously_skip_permissions,
            dangerously_bypass_approvals_and_sandbox,
            ctxhelm_bin,
            mode,
            target_agent,
            pack,
            fail_on_regression,
            health_min_commits,
            allow_dirty_health,
            health_check_success_commands,
            health_fail_fast_success_commands,
            health_require_setup_commands,
            force,
        } => {
            let out = out.unwrap_or_else(|| default_public_matrix_config_out(preset));
            init_public_matrix_config(InitPublicMatrixOptions {
                preset,
                repo,
                suite_path: suite,
                out,
                out_dir,
                health_out,
                agent_preset,
                agent_bin,
                model,
                agent_args: agent_arg,
                dangerously_skip_permissions,
                dangerously_bypass_approvals_and_sandbox,
                ctxhelm_bin,
                mode,
                target_agent,
                pack,
                fail_on_regression,
                health_min_commits,
                allow_dirty_health,
                health_check_success_commands,
                health_fail_fast_success_commands,
                health_require_setup_commands,
                force,
            })?;
        }
        Command::SuiteHealth {
            preset,
            suite,
            repo,
            out,
            format,
            min_commits,
            allow_dirty,
            check_success_commands,
            fail_fast_success_commands,
            require_setup_commands,
        } => {
            let suite = load_suite(&suite)?;
            let preset_name = preset.map(public_suite_preset_name);
            let anchor_files = preset.map(public_suite_anchor_files).unwrap_or(&[]);
            let health = suite_health_report(
                preset_name,
                &repo,
                min_commits,
                allow_dirty,
                check_success_commands,
                fail_fast_success_commands,
                require_setup_commands,
                &suite,
                anchor_files,
            )?;
            match format {
                OutputFormat::Json => write_json(&health, &out)?,
                OutputFormat::Markdown => write_text(&render_markdown_suite_health(&health), &out)?,
            }
            if !health.ok {
                anyhow::bail!(
                    "suite health check failed; wrote source-free health report to {}",
                    out.display()
                );
            }
            println!("wrote {}", out.display());
        }
        Command::ValidateSuite { suite } => {
            let suite = load_suite(&suite)?;
            println!(
                "suite `{}` is valid: {} task(s), source-free path contract ok",
                suite.name,
                suite.tasks.len()
            );
        }
        Command::Run {
            suite,
            trace_dir,
            out,
            format,
        } => {
            let suite = load_suite(&suite)?;
            let traces = load_traces(&trace_dir)?;
            let report = build_report(&suite, &traces)?;
            match format {
                OutputFormat::Json => write_json(&report, &out)?,
                OutputFormat::Markdown => write_text(&render_markdown_report(&report), &out)?,
            }
            println!("wrote {}", out.display());
        }
        Command::CtxhelmTrace {
            suite,
            repo,
            ctxhelm_bin,
            mode,
            target_agent,
            semantic,
            out_dir,
        } => {
            let suite = load_suite(&suite)?;
            std::fs::create_dir_all(&out_dir)
                .with_context(|| format!("create {}", out_dir.display()))?;
            for task in &suite.tasks {
                let started = Instant::now();
                let mut command = ProcessCommand::new(&ctxhelm_bin);
                command
                    .arg("prepare-task")
                    .arg("--repo")
                    .arg(&repo)
                    .arg("--mode")
                    .arg(&mode)
                    .arg("--target-agent")
                    .arg(&target_agent)
                    .arg("--no-trace");
                if semantic {
                    command.arg("--semantic");
                }
                command.arg(&task.prompt);
                let output = command
                    .output()
                    .with_context(|| format!("run {}", ctxhelm_bin.display()))?;
                if !output.status.success() {
                    anyhow::bail!(
                        "ctxhelm prepare-task failed for `{}` with status {:?}",
                        task.id,
                        output.status.code()
                    );
                }
                let stdout = String::from_utf8(output.stdout).context("ctxhelm stdout utf8")?;
                let trace = trace_from_ctxhelm_prepare_json(
                    task,
                    &stdout,
                    "ctxhelm",
                    AgentVariant::CtxhelmPlan,
                    Some(started.elapsed().as_millis() as u64),
                )?;
                let out = out_dir.join(format!("{}.json", task.id));
                write_json(&trace, &out)?;
                println!("wrote {}", out.display());
            }
        }
        Command::ClaudeTrace {
            suite,
            events,
            variant,
            out_dir,
        } => {
            let suite = load_suite(&suite)?;
            let events = load_agent_events(&events)?;
            let traces = traces_from_agent_events(&suite, &events, "claude-code", variant.into())?;
            std::fs::create_dir_all(&out_dir)
                .with_context(|| format!("create {}", out_dir.display()))?;
            for trace in traces {
                let out = out_dir.join(format!("{}.json", trace.task_id));
                write_json(&trace, &out)?;
                println!("wrote {}", out.display());
            }
        }
        Command::StreamTrace {
            suite,
            stream,
            task_id,
            agent,
            variant,
            repo_root,
            status,
            out_dir,
        } => {
            let suite = load_suite(&suite)?;
            let task = suite
                .tasks
                .iter()
                .find(|task| task.id == task_id)
                .with_context(|| format!("suite task `{}`", task_id))?;
            let raw = std::fs::read_to_string(&stream)
                .with_context(|| format!("read {}", stream.display()))?;
            let repo_root = repo_root
                .as_deref()
                .map(std::fs::canonicalize)
                .transpose()
                .context("resolve repo root")?;
            let mut events = events_from_agent_stream_jsonl(
                &task_id,
                &raw,
                repo_root.as_deref(),
                &task.expected_tests,
            )?;
            events.push(AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: task_id.clone(),
                event_kind: AgentEventKind::Status,
                path: None,
                command_class: None,
                command_hash: None,
                touched_tests: Vec::new(),
                exit_status: None,
                status: Some(status.into()),
                token_estimate: None,
                elapsed_millis: None,
                observed_at_millis: Some(events.len() as u64),
                privacy: PrivacyStatus::source_free(),
            });
            let traces = traces_from_agent_events(&suite, &events, &agent, variant.into())?;
            std::fs::create_dir_all(&out_dir)
                .with_context(|| format!("create {}", out_dir.display()))?;
            for trace in traces {
                let out = out_dir.join(format!("{}.json", trace.task_id));
                write_json(&trace, &out)?;
                println!("wrote {}", out.display());
            }
        }
        Command::LocalRun {
            suite,
            repo,
            work_dir,
            out_dir,
            agent,
            variant,
            setup_command,
            adapter_command,
            capture_stream,
            keep_workdirs,
        } => {
            let suite = load_suite(&suite)?;
            run_local_suite(
                &suite,
                &repo,
                &work_dir,
                &out_dir,
                &agent,
                variant.into(),
                &setup_command,
                None,
                adapter_command.as_deref(),
                capture_stream,
                keep_workdirs,
            )?;
        }
        Command::CtxhelmRun {
            suite,
            repo,
            work_dir,
            out_dir,
            ctxhelm_bin,
            mode,
            target_agent,
            semantic,
            semantic_provider,
            semantic_model,
            semantic_dimensions,
            pack,
            pack_budget,
            agent,
            variant,
            adapter_command,
            capture_stream,
            keep_workdirs,
        } => {
            let suite = load_suite(&suite)?;
            let ctxhelm = CtxhelmRunConfig {
                ctxhelm_bin,
                mode,
                target_agent,
                semantic,
                semantic_provider,
                semantic_model,
                semantic_dimensions,
                include_pack: pack,
                pack_budget,
            };
            run_local_suite(
                &suite,
                &repo,
                &work_dir,
                &out_dir,
                &agent,
                variant.into(),
                &[],
                Some(&ctxhelm),
                adapter_command.as_deref(),
                capture_stream,
                keep_workdirs,
            )?;
        }
        Command::ClaudeRun {
            suite,
            repo,
            work_dir,
            out_dir,
            claude_bin,
            model,
            claude_arg,
            dangerously_skip_permissions,
            capture_stream,
            keep_workdirs,
        } => {
            let suite = load_suite(&suite)?;
            let command = claude_adapter_command(
                &current_helmbench_bin()?,
                &claude_bin,
                model.as_deref(),
                &claude_arg,
                dangerously_skip_permissions,
                !capture_stream,
            );
            run_local_suite(
                &suite,
                &repo,
                &work_dir,
                &out_dir,
                "claude-code",
                AgentVariant::Native,
                &[],
                None,
                Some(&command),
                capture_stream,
                keep_workdirs,
            )?;
        }
        Command::CodexRun {
            suite,
            repo,
            work_dir,
            out_dir,
            codex_bin,
            model,
            codex_arg,
            dangerously_bypass_approvals_and_sandbox,
            capture_stream,
            keep_workdirs,
        } => {
            let suite = load_suite(&suite)?;
            let command = codex_adapter_command(
                &current_helmbench_bin()?,
                &codex_bin,
                model.as_deref(),
                &codex_arg,
                dangerously_bypass_approvals_and_sandbox,
                !capture_stream,
            );
            run_local_suite(
                &suite,
                &repo,
                &work_dir,
                &out_dir,
                "codex",
                AgentVariant::Native,
                &[],
                None,
                Some(&command),
                capture_stream,
                keep_workdirs,
            )?;
        }
        Command::RecordEvent {
            events,
            task_id,
            event_kind,
            path,
            command_class,
            command_hash,
            touched_test,
            exit_status,
            status,
            token_estimate,
            elapsed_millis,
            observed_at_millis,
        } => {
            let event = AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id,
                event_kind: event_kind.into(),
                path,
                command_class: command_class.map(Into::into),
                command_hash,
                touched_tests: touched_test,
                exit_status,
                status: status.map(Into::into),
                token_estimate,
                elapsed_millis,
                observed_at_millis,
                privacy: PrivacyStatus::source_free(),
            };
            validate_agent_event(&event)?;
            append_event(&events, &event)?;
            println!("appended {}", events.display());
        }
        Command::Compare {
            base,
            head,
            format,
            out,
        } => {
            let base_report = read_report(&base)?;
            let head_report = read_report(&head)?;
            validate_comparable_reports(&base_report, &head_report)?;
            let compare = compare_reports(&base_report, &head_report);
            let rendered = match format {
                OutputFormat::Json => serde_json::to_string_pretty(&compare)?,
                OutputFormat::Markdown => render_markdown_compare(&compare),
            };
            if let Some(out) = out {
                write_text(&rendered, &out)?;
                println!("wrote {}", out.display());
            } else {
                print!("{rendered}");
            }
        }
        Command::BenchmarkSummary {
            base,
            head,
            out,
            format,
        } => {
            let base_report = read_report(&base)?;
            let head_reports = head
                .iter()
                .map(|path| read_report(path))
                .collect::<Result<Vec<_>>>()?;
            let summary = build_benchmark_summary(&base_report, &head_reports)?;
            match format {
                OutputFormat::Json => write_json(&summary, &out)?,
                OutputFormat::Markdown => {
                    write_text(&render_markdown_benchmark_summary(&summary), &out)?
                }
            }
            println!("wrote {}", out.display());
        }
        Command::LaunchReadiness {
            suite,
            base_report,
            head_report,
            health,
            matrix,
            min_task_count,
            min_real_agent_rows,
            out,
            format,
        } => {
            let report = build_launch_readiness_report(
                &suite,
                &base_report,
                &head_report,
                &health,
                &matrix,
                min_task_count,
                min_real_agent_rows,
            )?;
            match format {
                OutputFormat::Json => write_json(&report, &out)?,
                OutputFormat::Markdown => {
                    write_text(&render_markdown_launch_readiness(&report), &out)?
                }
            }
            println!("wrote {}", out.display());
        }
        Command::EvidenceBundle {
            suite,
            health,
            base_report,
            head_report,
            out_dir,
            force,
        } => {
            write_evidence_bundle(
                &suite,
                health.as_deref(),
                &base_report,
                &head_report,
                &out_dir,
                force,
            )?;
            println!("wrote {}", out_dir.display());
        }
        Command::VerifyBundle { bundle } => {
            verify_evidence_bundle(&bundle)?;
            println!(
                "bundle `{}` is valid: source-free manifest and artifact hashes ok",
                bundle.display()
            );
        }
        Command::VerifyMatrix { matrix } => {
            let manifest = verify_run_matrix(&matrix)?;
            println!(
                "matrix `{}` is valid: {} head run(s), evidence bundle verified, quality gate passed: {}",
                matrix.display(),
                manifest.heads.len(),
                manifest.quality_gate_passed
            );
        }
        Command::QualityGate {
            summary,
            out,
            format,
            min_task_count,
            min_success_rate_delta,
            min_validation_coverage_rate_delta,
            max_irrelevant_read_rate_delta,
            min_recommendation_recall_delta,
            min_context_precision_delta,
            min_edited_file_recall_delta,
            max_average_time_to_first_relevant_file_millis_delta,
            max_total_tool_calls_delta,
            max_total_token_estimate_delta,
            max_tool_calls_per_success_delta,
            max_token_estimate_per_success_delta,
        } => {
            let summary = read_benchmark_summary(&summary)?;
            let gate = evaluate_quality_gate(
                &summary,
                &QualityGateConfig {
                    min_task_count,
                    min_success_rate_delta,
                    min_validation_coverage_rate_delta,
                    max_irrelevant_read_rate_delta,
                    min_recommendation_recall_delta,
                    min_context_precision_delta,
                    min_edited_file_recall_delta,
                    max_average_time_to_first_relevant_file_millis_delta,
                    max_total_tool_calls_delta,
                    max_total_token_estimate_delta,
                    max_tool_calls_per_success_delta,
                    max_token_estimate_per_success_delta,
                },
            )?;
            let rendered = match format {
                OutputFormat::Json => serde_json::to_string_pretty(&gate)?,
                OutputFormat::Markdown => render_markdown_quality_gate(&gate),
            };
            if let Some(out) = out {
                write_text(&rendered, &out)?;
                println!("wrote {}", out.display());
            } else {
                print!("{rendered}");
            }
            if !gate.passed {
                anyhow::bail!("quality gate failed");
            }
        }
        Command::Autopsy {
            suite,
            trace_dir,
            out,
            format,
        } => {
            let suite = load_suite(&suite)?;
            let traces = load_traces(&trace_dir)?;
            let autopsy = build_autopsy(&suite, &traces)?;
            match format {
                OutputFormat::Json => write_json(&autopsy, &out)?,
                OutputFormat::Markdown => write_text(&render_markdown_autopsy(&autopsy), &out)?,
            }
            println!("wrote {}", out.display());
        }
        Command::DiffAutopsy {
            suite,
            repo,
            task_id,
            base_ref,
            head_ref,
            pr,
            github_repo,
            out,
            format,
        } => {
            let suite = load_suite(&suite)?;
            let (changed_files, base_ref, head_ref) = if let Some(pr) = &pr {
                if base_ref.is_some() || head_ref.is_some() {
                    anyhow::bail!("--pr cannot be combined with --base-ref or --head-ref");
                }
                (
                    gh_pr_diff_paths(&repo, pr, github_repo.as_deref())?,
                    "github-pr".to_string(),
                    Some(source_free_pr_label(pr)),
                )
            } else {
                if github_repo.is_some() {
                    anyhow::bail!("--github-repo requires --pr");
                }
                let base_ref = base_ref.unwrap_or_else(|| "HEAD".to_string());
                let changed_files = if let Some(head_ref) = &head_ref {
                    git_diff_paths(&repo, &base_ref, head_ref)?
                } else {
                    git_changed_paths(&repo)?
                };
                (changed_files, base_ref, head_ref)
            };
            let autopsy = build_diff_autopsy(
                &suite,
                &task_id,
                &changed_files,
                &base_ref,
                head_ref.as_deref(),
            )?;
            match format {
                OutputFormat::Json => write_json(&autopsy, &out)?,
                OutputFormat::Markdown => {
                    write_text(&render_markdown_diff_autopsy(&autopsy), &out)?
                }
            }
            println!("wrote {}", out.display());
        }
        Command::Dashboard { report, out } => {
            let reports = report
                .iter()
                .map(|path| read_report(path))
                .collect::<Result<Vec<_>>>()?;
            let rendered = render_html_dashboard(&reports)?;
            write_text(&rendered, &out)?;
            println!("wrote {}", out.display());
        }
        Command::Doctor { repo, format, out } => {
            let root = project_root_for_cli(repo)?;
            write_doctor_report(&root, format, out.as_ref())?;
        }
    }
    Ok(())
}

fn write_text(content: &str, path: &PathBuf) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(path, content).with_context(|| format!("write {}", path.display()))
}

fn schema_contract(kind: SchemaKind) -> &'static str {
    match kind {
        SchemaKind::TaskSuite => include_str!("../schemas/task-suite.schema.json"),
        SchemaKind::AgentTrace => include_str!("../schemas/agent-trace.schema.json"),
        SchemaKind::AgentEvent => include_str!("../schemas/agent-event.schema.json"),
        SchemaKind::RunReport => include_str!("../schemas/run-report.schema.json"),
        SchemaKind::CompareReport => include_str!("../schemas/compare-report.schema.json"),
        SchemaKind::BenchmarkSummary => include_str!("../schemas/benchmark-summary.schema.json"),
        SchemaKind::LaunchReadiness => include_str!("../schemas/launch-readiness.schema.json"),
        SchemaKind::QualityGate => include_str!("../schemas/quality-gate.schema.json"),
        SchemaKind::RunMatrixConfig => include_str!("../schemas/run-matrix-config.schema.json"),
        SchemaKind::MatrixHistory => include_str!("../schemas/matrix-history.schema.json"),
        SchemaKind::DoctorReport => include_str!("../schemas/doctor-report.schema.json"),
        SchemaKind::Autopsy => include_str!("../schemas/autopsy.schema.json"),
        SchemaKind::DiffAutopsy => include_str!("../schemas/diff-autopsy.schema.json"),
        SchemaKind::SuiteHealth => include_str!("../schemas/suite-health.schema.json"),
        SchemaKind::EvidenceBundle => include_str!("../schemas/evidence-bundle.schema.json"),
        SchemaKind::RunMatrixManifest => {
            include_str!("../schemas/run-matrix-manifest.schema.json")
        }
        SchemaKind::RunMatrixPrivacyReport => {
            include_str!("../schemas/run-matrix-privacy-report.schema.json")
        }
    }
}

fn all_schema_kinds() -> &'static [SchemaKind] {
    &[
        SchemaKind::TaskSuite,
        SchemaKind::AgentTrace,
        SchemaKind::AgentEvent,
        SchemaKind::RunReport,
        SchemaKind::CompareReport,
        SchemaKind::BenchmarkSummary,
        SchemaKind::LaunchReadiness,
        SchemaKind::QualityGate,
        SchemaKind::RunMatrixConfig,
        SchemaKind::MatrixHistory,
        SchemaKind::DoctorReport,
        SchemaKind::Autopsy,
        SchemaKind::DiffAutopsy,
        SchemaKind::SuiteHealth,
        SchemaKind::EvidenceBundle,
        SchemaKind::RunMatrixManifest,
        SchemaKind::RunMatrixPrivacyReport,
    ]
}

fn schema_contract_filename(kind: SchemaKind) -> &'static str {
    match kind {
        SchemaKind::TaskSuite => "task-suite.schema.json",
        SchemaKind::AgentTrace => "agent-trace.schema.json",
        SchemaKind::AgentEvent => "agent-event.schema.json",
        SchemaKind::RunReport => "run-report.schema.json",
        SchemaKind::CompareReport => "compare-report.schema.json",
        SchemaKind::BenchmarkSummary => "benchmark-summary.schema.json",
        SchemaKind::LaunchReadiness => "launch-readiness.schema.json",
        SchemaKind::QualityGate => "quality-gate.schema.json",
        SchemaKind::RunMatrixConfig => "run-matrix-config.schema.json",
        SchemaKind::MatrixHistory => "matrix-history.schema.json",
        SchemaKind::DoctorReport => "doctor-report.schema.json",
        SchemaKind::Autopsy => "autopsy.schema.json",
        SchemaKind::DiffAutopsy => "diff-autopsy.schema.json",
        SchemaKind::SuiteHealth => "suite-health.schema.json",
        SchemaKind::EvidenceBundle => "evidence-bundle.schema.json",
        SchemaKind::RunMatrixManifest => "run-matrix-manifest.schema.json",
        SchemaKind::RunMatrixPrivacyReport => "run-matrix-privacy-report.schema.json",
    }
}

fn write_all_schema_contracts(out_dir: &Path) -> Result<()> {
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    for kind in all_schema_kinds() {
        let path = out_dir.join(schema_contract_filename(*kind));
        write_text(schema_contract(*kind), &path)?;
    }
    Ok(())
}

#[cfg(test)]
fn run_doctor(root: &Path) -> Result<()> {
    write_doctor_report(root, OutputFormat::Markdown, None)
}

fn write_doctor_report(root: &Path, format: OutputFormat, out: Option<&PathBuf>) -> Result<()> {
    let report = build_doctor_report(root);
    let rendered = match format {
        OutputFormat::Json => serde_json::to_string_pretty(&report)?,
        OutputFormat::Markdown => render_markdown_doctor_report(root, &report),
    };

    if let Some(out) = out {
        write_text(&rendered, out)?;
        println!("wrote {}", out.display());
    } else {
        println!("{rendered}");
    }

    if !report.ok {
        anyhow::bail!("doctor found missing required HelmBench prerequisites");
    }
    Ok(())
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctorReport {
    schema_version: u32,
    repo_name: String,
    required_checks: Vec<DoctorCheck>,
    optional_integrations: Vec<DoctorIntegration>,
    direct_runners: Vec<DoctorDirectRunner>,
    observation_modes: Vec<DoctorObservationMode>,
    supported_variants: Vec<AgentVariant>,
    privacy: PrivacyStatus,
    ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctorCheck {
    name: String,
    ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctorIntegration {
    name: String,
    command: String,
    available: bool,
    version_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctorDirectRunner {
    name: String,
    command: String,
    available: bool,
    isolated_clones: bool,
    injects_source_free_event_contract: bool,
    capture_stream_supported: bool,
    suppresses_raw_output_by_default: bool,
    unrestricted_flag: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct DoctorObservationMode {
    name: String,
    source_free: bool,
    persists_raw_stream: bool,
    description: String,
}

fn build_doctor_report(root: &Path) -> DoctorReport {
    let required_checks = vec![
        doctor_check("git available", command_available("git")),
        doctor_check("cargo available", command_available("cargo")),
        doctor_check("repo is a git checkout", git_repo_ok(root)),
        doctor_check("Cargo.toml exists", root.join("Cargo.toml").exists()),
        doctor_check(
            "verification script exists",
            root.join("scripts/verify.sh").exists(),
        ),
        doctor_check(
            "CI workflow exists",
            root.join(".github/workflows/ci.yml").exists(),
        ),
        doctor_check(
            "release workflow exists",
            root.join(".github/workflows/release.yml").exists(),
        ),
        doctor_check(
            "example suite loads",
            load_suite(&root.join("suites/example-auth-bugs.json")).is_ok(),
        ),
        doctor_check(
            "example native report is source-free",
            read_report(&root.join("reports/example-native.json")).is_ok(),
        ),
        doctor_check(
            "example ctxhelm report is source-free",
            read_report(&root.join("reports/example-ctxhelm.json")).is_ok(),
        ),
    ];
    let optional_integrations = vec![
        doctor_integration("ctxhelm", "ctxhelm"),
        doctor_integration("claude-code", "claude"),
        doctor_integration("codex", "codex"),
    ];
    let direct_runners = vec![
        doctor_direct_runner("claude-run", "claude", "--dangerously-skip-permissions"),
        doctor_direct_runner(
            "codex-run",
            "codex",
            "--dangerously-bypass-approvals-and-sandbox",
        ),
    ];
    let observation_modes = vec![
        DoctorObservationMode {
            name: "record-event".to_string(),
            source_free: true,
            persists_raw_stream: false,
            description: "agent or hook appends validated source-free events".to_string(),
        },
        DoctorObservationMode {
            name: "capture-stream".to_string(),
            source_free: true,
            persists_raw_stream: false,
            description: "structured stdout is parsed in memory and discarded".to_string(),
        },
        DoctorObservationMode {
            name: "git-diff-inference".to_string(),
            source_free: true,
            persists_raw_stream: false,
            description: "edited files are inferred from git status after each isolated run"
                .to_string(),
        },
        DoctorObservationMode {
            name: "validation-command-summary".to_string(),
            source_free: true,
            persists_raw_stream: false,
            description: "success commands are stored by class/hash/exit status".to_string(),
        },
    ];
    let supported_variants = vec![
        AgentVariant::Native,
        AgentVariant::NativeSearch,
        AgentVariant::CtxhelmPlan,
        AgentVariant::CtxhelmMcp,
        AgentVariant::CtxhelmPack,
    ];
    let ok = required_checks.iter().all(|check| check.ok);

    DoctorReport {
        schema_version: DOCTOR_REPORT_SCHEMA_VERSION,
        repo_name: repo_name(root),
        required_checks,
        optional_integrations,
        direct_runners,
        observation_modes,
        supported_variants,
        privacy: PrivacyStatus::source_free(),
        ok,
    }
}

fn doctor_check(name: &str, ok: bool) -> DoctorCheck {
    DoctorCheck {
        name: name.to_string(),
        ok,
    }
}

fn doctor_integration(name: &str, command: &str) -> DoctorIntegration {
    let version_hash = command_version_hash(command);
    DoctorIntegration {
        name: name.to_string(),
        command: command.to_string(),
        available: version_hash.is_some(),
        version_hash,
    }
}

fn doctor_direct_runner(name: &str, command: &str, unrestricted_flag: &str) -> DoctorDirectRunner {
    DoctorDirectRunner {
        name: name.to_string(),
        command: command.to_string(),
        available: command_available(command),
        isolated_clones: true,
        injects_source_free_event_contract: true,
        capture_stream_supported: true,
        suppresses_raw_output_by_default: true,
        unrestricted_flag: Some(unrestricted_flag.to_string()),
    }
}

fn render_markdown_doctor_report(root: &Path, report: &DoctorReport) -> String {
    let mut out = String::new();
    out.push_str("# HelmBench Doctor\n\n");
    out.push_str(&format!("Repo: `{}`\n\n", root.display()));
    out.push_str("Privacy: source-free reports enforced\n\n");
    out.push_str(&format!(
        "Status: **{}**\n\n",
        if report.ok { "ok" } else { "error" }
    ));

    out.push_str("## Required Checks\n\n");
    for check in &report.required_checks {
        out.push_str(&format!(
            "- {}: `{}`\n",
            check.name,
            if check.ok { "ok" } else { "error" }
        ));
    }

    out.push_str("\n## Optional Integrations\n\n");
    for integration in &report.optional_integrations {
        out.push_str(&format!(
            "- {} (`{}`): `{}`",
            integration.name,
            integration.command,
            if integration.available { "ok" } else { "warn" }
        ));
        if let Some(hash) = &integration.version_hash {
            out.push_str(&format!(" ({hash})"));
        }
        out.push('\n');
    }

    out.push_str("\n## Direct Runner Readiness\n\n");
    out.push_str("| Runner | Command | Available | Event contract | Capture stream | Raw output suppressed | Isolated clones |\n");
    out.push_str("| --- | --- | --- | --- | --- | --- | --- |\n");
    for runner in &report.direct_runners {
        out.push_str(&format!(
            "| `{}` | `{}` | {} | {} | {} | {} | {} |\n",
            runner.name,
            runner.command,
            yes_no(runner.available),
            yes_no(runner.injects_source_free_event_contract),
            yes_no(runner.capture_stream_supported),
            yes_no(runner.suppresses_raw_output_by_default),
            yes_no(runner.isolated_clones)
        ));
    }

    out.push_str("\n## Observation Modes\n\n");
    for mode in &report.observation_modes {
        out.push_str(&format!(
            "- `{}`: {}; source-free `{}`, persists raw stream `{}`\n",
            mode.name,
            mode.description,
            yes_no(mode.source_free),
            yes_no(mode.persists_raw_stream)
        ));
    }

    out.push_str("\n## Supported Variants\n\n");
    for variant in &report.supported_variants {
        out.push_str(&format!("- `{:?}`\n", variant));
    }

    out.push_str("\n## Privacy\n\n");
    out.push_str("- Source-free: `true`\n");
    out.push_str("- Raw source logged: `false`\n");
    out.push_str("- Raw prompts logged: `false`\n");
    out.push_str("- Raw transcripts logged: `false`\n");
    out.push_str("- Raw terminal logs logged: `false`\n");
    out
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchReadinessReport {
    schema_version: u32,
    suite_name: String,
    status: LaunchReadinessStatus,
    min_task_count: usize,
    min_real_agent_rows: usize,
    benchmark: LaunchReadinessBenchmark,
    artifacts: Vec<LaunchReadinessArtifact>,
    checks: Vec<LaunchReadinessCheck>,
    privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LaunchReadinessStatus {
    LaunchReady,
    SmokeProof,
    NotReady,
}

impl std::fmt::Display for LaunchReadinessStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::LaunchReady => write!(f, "launch_ready"),
            Self::SmokeProof => write!(f, "smoke_proof"),
            Self::NotReady => write!(f, "not_ready"),
        }
    }
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum LaunchReadinessCheckStatus {
    Pass,
    Warn,
    Fail,
}

impl std::fmt::Display for LaunchReadinessCheckStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Pass => write!(f, "pass"),
            Self::Warn => write!(f, "warn"),
            Self::Fail => write!(f, "fail"),
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchReadinessCheck {
    name: String,
    status: LaunchReadinessCheckStatus,
    evidence: String,
    detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchReadinessArtifact {
    kind: String,
    label: String,
    source_free: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct LaunchReadinessBenchmark {
    task_count: usize,
    run_count: usize,
    real_agent_run_count: usize,
    low_sample_warning: bool,
    best_success_rate: f32,
    best_validation_coverage_rate: f32,
    best_recommendation_recall: f32,
    best_context_precision: f32,
    best_edited_file_recall: f32,
}

fn build_launch_readiness_report(
    suite_path: &Path,
    base_report_path: &Path,
    head_report_paths: &[PathBuf],
    health_paths: &[PathBuf],
    matrix_paths: &[PathBuf],
    min_task_count: usize,
    min_real_agent_rows: usize,
) -> Result<LaunchReadinessReport> {
    let suite = load_suite(suite_path)?;
    validate_suite(&suite)?;

    let base_report = read_report(base_report_path)?;
    let head_reports = head_report_paths
        .iter()
        .map(|path| read_report(path))
        .collect::<Result<Vec<_>>>()?;
    let summary = build_benchmark_summary(&base_report, &head_reports)?;

    let mut artifacts = vec![launch_artifact("suite", suite_path)];
    artifacts.push(launch_artifact("base_report", base_report_path));
    artifacts.extend(
        head_report_paths
            .iter()
            .map(|path| launch_artifact("head_report", path)),
    );

    let mut checks = Vec::new();
    checks.push(launch_check(
        "suite contract",
        LaunchReadinessCheckStatus::Pass,
        "suite".to_string(),
        format!(
            "suite `{}` validates with {} task(s)",
            suite.name,
            suite.tasks.len()
        ),
    ));
    checks.push(launch_check(
        "source-free reports",
        LaunchReadinessCheckStatus::Pass,
        "run_reports".to_string(),
        format!(
            "{} report(s) accepted by benchmark-summary",
            summary.runs.len()
        ),
    ));

    let real_agent_run_count = summary
        .runs
        .iter()
        .filter(|run| launch_agent_label_is_real(&run.agent))
        .count();
    let best = summary
        .runs
        .iter()
        .max_by(|left, right| {
            left.success_rate
                .partial_cmp(&right.success_rate)
                .unwrap_or(std::cmp::Ordering::Equal)
                .then_with(|| {
                    left.validation_coverage_rate
                        .partial_cmp(&right.validation_coverage_rate)
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
        })
        .context("benchmark summary has no runs")?;

    checks.push(launch_check(
        "recommended task count",
        if summary.confidence.task_count >= min_task_count {
            LaunchReadinessCheckStatus::Pass
        } else {
            LaunchReadinessCheckStatus::Warn
        },
        "benchmark_summary".to_string(),
        format!(
            "{} task(s) observed; launch target is {}",
            summary.confidence.task_count, min_task_count
        ),
    ));
    checks.push(launch_check(
        "real-agent rows",
        if real_agent_run_count >= min_real_agent_rows {
            LaunchReadinessCheckStatus::Pass
        } else {
            LaunchReadinessCheckStatus::Warn
        },
        "benchmark_summary".to_string(),
        format!(
            "{} real-agent row(s) observed; launch target is {}",
            real_agent_run_count, min_real_agent_rows
        ),
    ));

    let mut matching_health = Vec::new();
    for path in health_paths {
        let health = read_public_suite_health(path)?;
        validate_public_suite_health_report(&health)?;
        if !privacy_status_is_source_free(&health.privacy) {
            anyhow::bail!("launch readiness health artifact is not source-free");
        }
        artifacts.push(launch_artifact("health", path));
        if health.suite_name == suite.name {
            matching_health.push(health);
        }
    }
    let health_status = if matching_health.is_empty() {
        LaunchReadinessCheckStatus::Warn
    } else if matching_health
        .iter()
        .any(|health| health.evidence_use == SuiteEvidenceUse::Unhealthy || !health.ok)
    {
        LaunchReadinessCheckStatus::Fail
    } else if matching_health
        .iter()
        .any(|health| health.evidence_use == SuiteEvidenceUse::OutcomeReady)
    {
        LaunchReadinessCheckStatus::Pass
    } else {
        LaunchReadinessCheckStatus::Warn
    };
    let health_detail = if matching_health.is_empty() {
        "no matching suite-health artifact was supplied".to_string()
    } else {
        let uses = matching_health
            .iter()
            .map(|health| health.evidence_use.to_string())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>()
            .join(", ");
        format!("matching suite-health evidenceUse: {uses}")
    };
    checks.push(launch_check(
        "outcome-health evidence",
        health_status,
        "suite_health".to_string(),
        health_detail,
    ));

    let mut verified_matrices = 0usize;
    let mut verified_matching_matrices = 0usize;
    let mut matrix_suite_mismatches = 0usize;
    let mut matrix_failures = 0usize;
    for path in matrix_paths {
        match verify_run_matrix(path) {
            Ok(manifest) => {
                verified_matrices += 1;
                let matrix_summary_path =
                    matrix_path(path, &manifest.artifacts.benchmark_summary_json)?;
                let matrix_summary = read_benchmark_summary(&matrix_summary_path)?;
                if matrix_summary.suite_name == suite.name {
                    verified_matching_matrices += 1;
                } else {
                    matrix_suite_mismatches += 1;
                }
                artifacts.push(launch_artifact("matrix", path));
                artifacts.push(LaunchReadinessArtifact {
                    kind: "matrix_evidence_use".to_string(),
                    label: manifest.suite_evidence_use.to_string(),
                    source_free: true,
                });
            }
            Err(_) => {
                matrix_failures += 1;
                artifacts.push(launch_artifact("matrix", path));
            }
        }
    }
    checks.push(launch_check(
        "verified run matrix",
        if matrix_failures > 0 {
            LaunchReadinessCheckStatus::Fail
        } else if verified_matching_matrices > 0 {
            LaunchReadinessCheckStatus::Pass
        } else {
            LaunchReadinessCheckStatus::Warn
        },
        "run_matrix".to_string(),
        if verified_matrices > 0 || matrix_failures > 0 {
            format!(
                "{verified_matching_matrices} matching verified matrix output(s), {matrix_suite_mismatches} suite mismatch(es), {matrix_failures} failure(s)"
            )
        } else {
            "no verified run-matrix artifact was supplied".to_string()
        },
    ));

    checks.push(launch_check(
        "privacy boundary",
        LaunchReadinessCheckStatus::Pass,
        "privacy".to_string(),
        "artifacts store paths, counts, statuses, hashes, and source-free flags only".to_string(),
    ));

    let status = if checks
        .iter()
        .any(|check| check.status == LaunchReadinessCheckStatus::Fail)
    {
        LaunchReadinessStatus::NotReady
    } else if checks
        .iter()
        .any(|check| check.status == LaunchReadinessCheckStatus::Warn)
    {
        LaunchReadinessStatus::SmokeProof
    } else {
        LaunchReadinessStatus::LaunchReady
    };

    Ok(LaunchReadinessReport {
        schema_version: LAUNCH_READINESS_SCHEMA_VERSION,
        suite_name: suite.name,
        status,
        min_task_count,
        min_real_agent_rows,
        benchmark: LaunchReadinessBenchmark {
            task_count: summary.confidence.task_count,
            run_count: summary.runs.len(),
            real_agent_run_count,
            low_sample_warning: summary.confidence.low_sample_warning,
            best_success_rate: best.success_rate,
            best_validation_coverage_rate: best.validation_coverage_rate,
            best_recommendation_recall: best.recommendation_recall,
            best_context_precision: best.context_precision,
            best_edited_file_recall: best.edited_file_recall,
        },
        artifacts,
        checks,
        privacy: PrivacyStatus::source_free(),
    })
}

fn render_markdown_launch_readiness(report: &LaunchReadinessReport) -> String {
    let mut out = String::new();
    out.push_str("# HelmBench Launch Readiness\n\n");
    out.push_str(&format!("Suite: `{}`\n\n", report.suite_name));
    out.push_str(&format!("Status: **{}**\n\n", report.status));

    out.push_str("## Benchmark\n\n");
    out.push_str("| Metric | Value |\n");
    out.push_str("| --- | ---: |\n");
    out.push_str(&format!("| Tasks | {} |\n", report.benchmark.task_count));
    out.push_str(&format!("| Runs | {} |\n", report.benchmark.run_count));
    out.push_str(&format!(
        "| Real-agent rows | {} |\n",
        report.benchmark.real_agent_run_count
    ));
    out.push_str(&format!(
        "| Best success rate | {} |\n",
        launch_pct(report.benchmark.best_success_rate)
    ));
    out.push_str(&format!(
        "| Best validation coverage | {} |\n",
        launch_pct(report.benchmark.best_validation_coverage_rate)
    ));
    out.push_str(&format!(
        "| Best recommendation recall | {} |\n",
        launch_pct(report.benchmark.best_recommendation_recall)
    ));
    out.push_str(&format!(
        "| Best context precision | {} |\n",
        launch_pct(report.benchmark.best_context_precision)
    ));
    out.push_str(&format!(
        "| Best edited-file recall | {} |\n",
        launch_pct(report.benchmark.best_edited_file_recall)
    ));

    out.push_str("\n## Checks\n\n");
    out.push_str("| Check | Status | Detail |\n");
    out.push_str("| --- | --- | --- |\n");
    for check in &report.checks {
        out.push_str(&format!(
            "| {} | `{}` | {} |\n",
            check.name, check.status, check.detail
        ));
    }

    out.push_str("\n## Artifacts\n\n");
    out.push_str("| Kind | Label | Source-free |\n");
    out.push_str("| --- | --- | --- |\n");
    for artifact in &report.artifacts {
        out.push_str(&format!(
            "| `{}` | `{}` | {} |\n",
            artifact.kind,
            artifact.label,
            yes_no(artifact.source_free)
        ));
    }

    out.push_str("\n## Privacy\n\n");
    out.push_str("- Source-free: `true`\n");
    out.push_str("- Raw source logged: `false`\n");
    out.push_str("- Raw prompts logged: `false`\n");
    out.push_str("- Raw transcripts logged: `false`\n");
    out.push_str("- Raw terminal logs logged: `false`\n");
    out
}

fn launch_check(
    name: &str,
    status: LaunchReadinessCheckStatus,
    evidence: String,
    detail: String,
) -> LaunchReadinessCheck {
    LaunchReadinessCheck {
        name: name.to_string(),
        status,
        evidence,
        detail,
    }
}

fn launch_artifact(kind: &str, path: &Path) -> LaunchReadinessArtifact {
    LaunchReadinessArtifact {
        kind: kind.to_string(),
        label: source_free_hash(kind, &path.display().to_string()),
        source_free: true,
    }
}

fn launch_agent_label_is_real(agent: &str) -> bool {
    let normalized = agent.to_ascii_lowercase();
    matches!(
        normalized.as_str(),
        "claude-code" | "claude" | "codex" | "cursor" | "devin" | "copilot"
    )
}

fn launch_pct(value: f32) -> String {
    format!("{:.1}%", value * 100.0)
}

fn repo_name(root: &Path) -> String {
    root.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string()
}

fn command_available(command: &str) -> bool {
    ProcessCommand::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

fn command_version_hash(command: &str) -> Option<String> {
    let output = ProcessCommand::new(command)
        .arg("--version")
        .output()
        .ok()?;
    if !output.status.success() {
        return None;
    }
    let mut version = String::from_utf8(output.stdout).ok()?;
    if version.trim().is_empty() {
        version = String::from_utf8(output.stderr).ok()?;
    }
    let version = version.trim();
    (!version.is_empty()).then(|| source_free_hash("version", version))
}

fn git_repo_ok(root: &Path) -> bool {
    ProcessCommand::new("git")
        .arg("-C")
        .arg(root)
        .arg("rev-parse")
        .arg("--is-inside-work-tree")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct PublicSuiteHealth {
    schema_version: u32,
    preset: String,
    suite_name: String,
    task_count: usize,
    repo_name: String,
    head: Option<String>,
    commit_count: Option<u64>,
    min_commits: u64,
    allow_dirty: bool,
    dirty: bool,
    fsck_ok: bool,
    validation_ready: bool,
    expected_file_count: usize,
    expected_test_count: usize,
    checked_files: Vec<String>,
    missing_files: Vec<String>,
    missing_expected_files: Vec<String>,
    missing_expected_tests: Vec<String>,
    #[serde(default)]
    setup_commands_required: bool,
    #[serde(default)]
    setup_command_ready: bool,
    #[serde(default)]
    tasks_missing_setup_command: Vec<String>,
    #[serde(default)]
    tasks_failed_setup_command: Vec<String>,
    tasks_missing_success_command: Vec<String>,
    #[serde(default)]
    success_command_check_requested: bool,
    #[serde(default)]
    success_command_check_fail_fast: bool,
    #[serde(default)]
    success_command_check_ready: bool,
    #[serde(default)]
    validation_baseline_ready: bool,
    #[serde(default)]
    baseline_success_command_pass_count: usize,
    #[serde(default)]
    baseline_success_command_fail_count: usize,
    #[serde(default)]
    baseline_success_command_timeout_count: usize,
    #[serde(default)]
    baseline_success_command_skipped_count: usize,
    #[serde(default)]
    baseline_success_commands: Vec<SuccessCommandBaseline>,
    #[serde(default = "default_suite_evidence_use")]
    evidence_use: SuiteEvidenceUse,
    ok: bool,
    privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
enum SuiteEvidenceUse {
    OutcomeReady,
    NavigationOnly,
    Unhealthy,
}

fn default_suite_evidence_use() -> SuiteEvidenceUse {
    SuiteEvidenceUse::NavigationOnly
}

impl std::fmt::Display for SuiteEvidenceUse {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let value = match self {
            SuiteEvidenceUse::OutcomeReady => "outcome_ready",
            SuiteEvidenceUse::NavigationOnly => "navigation_only",
            SuiteEvidenceUse::Unhealthy => "unhealthy",
        };
        formatter.write_str(value)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct SuccessCommandBaseline {
    task_id: String,
    command_class: CommandClass,
    command_hash: String,
    exit_status: Option<i32>,
    elapsed_millis: u64,
    timed_out: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceBundleManifest {
    schema_version: u32,
    suite_name: String,
    baseline_agent: String,
    baseline_variant: AgentVariant,
    #[serde(default = "default_suite_evidence_use")]
    evidence_use: SuiteEvidenceUse,
    artifacts: Vec<EvidenceBundleArtifact>,
    privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceBundleArtifact {
    kind: String,
    path: String,
    source_name: String,
    byte_count: u64,
    content_hash: String,
    source_free_checked: bool,
}

fn verify_evidence_bundle(bundle: &Path) -> Result<()> {
    let manifest_path = bundle.join("manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest = serde_json::from_str::<EvidenceBundleManifest>(&raw)
        .with_context(|| format!("parse {}", manifest_path.display()))?;

    if manifest.schema_version != EVIDENCE_BUNDLE_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported evidence bundle schemaVersion {}; expected {}",
            manifest.schema_version,
            EVIDENCE_BUNDLE_SCHEMA_VERSION
        );
    }
    if manifest.suite_name.trim().is_empty() {
        anyhow::bail!("evidence bundle suiteName must not be empty");
    }
    if manifest.baseline_agent.trim().is_empty() {
        anyhow::bail!("evidence bundle baselineAgent must not be empty");
    }
    if manifest.artifacts.is_empty() {
        anyhow::bail!("evidence bundle must contain at least one artifact");
    }
    if !manifest.privacy.source_free
        || manifest.privacy.raw_source_logged
        || manifest.privacy.raw_prompt_logged
        || manifest.privacy.raw_transcript_logged
        || manifest.privacy.raw_terminal_logged
    {
        anyhow::bail!("evidence bundle manifest is not source-free");
    }

    let mut seen_paths = BTreeSet::new();
    let mut health_artifact_path = None;
    for artifact in &manifest.artifacts {
        if artifact.kind.trim().is_empty() {
            anyhow::bail!("evidence bundle artifact kind must not be empty");
        }
        helmbench::validate_safe_relative_path_for_cli(&artifact.path)
            .with_context(|| format!("validate artifact path `{}`", artifact.path))?;
        if !seen_paths.insert(artifact.path.clone()) {
            anyhow::bail!(
                "duplicate evidence bundle artifact path `{}`",
                artifact.path
            );
        }
        if artifact.source_name.contains('/') || artifact.source_name.contains('\\') {
            anyhow::bail!(
                "evidence bundle artifact `{}` has unsafe sourceName `{}`",
                artifact.path,
                artifact.source_name
            );
        }
        if !artifact.source_free_checked {
            anyhow::bail!(
                "evidence bundle artifact `{}` was not source-free checked",
                artifact.path
            );
        }
        if !artifact.content_hash.starts_with("fnv64:") {
            anyhow::bail!(
                "evidence bundle artifact `{}` has unsupported contentHash `{}`",
                artifact.path,
                artifact.content_hash
            );
        }
        if artifact.kind == "health" {
            health_artifact_path = Some(artifact.path.clone());
        }

        let artifact_path = bundle.join(&artifact.path);
        let bytes = std::fs::read(&artifact_path)
            .with_context(|| format!("read artifact {}", artifact_path.display()))?;
        let byte_count = bytes.len() as u64;
        if byte_count != artifact.byte_count {
            anyhow::bail!(
                "evidence bundle artifact `{}` byte count mismatch: manifest {}, actual {}",
                artifact.path,
                artifact.byte_count,
                byte_count
            );
        }
        let actual_hash = content_hash(&bytes);
        if actual_hash != artifact.content_hash {
            anyhow::bail!(
                "evidence bundle artifact `{}` hash mismatch: manifest {}, actual {}",
                artifact.path,
                artifact.content_hash,
                actual_hash
            );
        }
    }
    if let Some(health_artifact_path) = health_artifact_path {
        let health = read_public_suite_health(&bundle.join(&health_artifact_path))?;
        validate_public_suite_health_report(&health)?;
        if health.evidence_use != manifest.evidence_use {
            anyhow::bail!(
                "evidence bundle evidenceUse `{}` does not match health evidenceUse `{}`",
                manifest.evidence_use,
                health.evidence_use
            );
        }
    }

    Ok(())
}

fn write_evidence_bundle(
    suite_path: &Path,
    health_path: Option<&Path>,
    base_report_path: &Path,
    head_report_paths: &[PathBuf],
    out_dir: &Path,
    force: bool,
) -> Result<()> {
    if out_dir.exists() {
        if !force {
            anyhow::bail!(
                "{} already exists; pass --force to replace it",
                out_dir.display()
            );
        }
        std::fs::remove_dir_all(out_dir)
            .with_context(|| format!("remove {}", out_dir.display()))?;
    }
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    std::fs::create_dir_all(out_dir.join("reports"))
        .with_context(|| format!("create {}", out_dir.join("reports").display()))?;

    let suite = load_suite(suite_path)?;
    let base_report = read_report(base_report_path)?;
    if base_report.suite_name != suite.name {
        anyhow::bail!(
            "base report suite `{}` does not match suite `{}`",
            base_report.suite_name,
            suite.name
        );
    }
    let head_reports = head_report_paths
        .iter()
        .map(|path| read_report(path))
        .collect::<Result<Vec<_>>>()?;
    for report in &head_reports {
        if report.suite_name != suite.name {
            anyhow::bail!(
                "head report suite `{}` does not match suite `{}`",
                report.suite_name,
                suite.name
            );
        }
    }

    let mut evidence_use = SuiteEvidenceUse::NavigationOnly;
    let mut artifacts = Vec::new();
    artifacts.push(copy_bundle_artifact(
        "suite",
        suite_path,
        out_dir,
        "suite.json",
        true,
    )?);
    if let Some(health_path) = health_path {
        let health = read_public_suite_health(health_path)?;
        validate_public_suite_health_report(&health)?;
        evidence_use = health.evidence_use;
        artifacts.push(copy_bundle_artifact(
            "health",
            health_path,
            out_dir,
            "health.json",
            true,
        )?);
    }
    artifacts.push(copy_bundle_artifact(
        "base_report",
        base_report_path,
        out_dir,
        "reports/base.json",
        true,
    )?);
    for (index, path) in head_report_paths.iter().enumerate() {
        artifacts.push(copy_bundle_artifact(
            "head_report",
            path,
            out_dir,
            &format!("reports/head-{}.json", index + 1),
            true,
        )?);
    }

    let summary = build_benchmark_summary(&base_report, &head_reports)?;
    let summary_json = serde_json::to_string_pretty(&summary)?;
    artifacts.push(write_bundle_artifact(
        "benchmark_summary_json",
        "generated",
        out_dir,
        "benchmark-summary.json",
        summary_json.as_bytes(),
        true,
    )?);
    let summary_markdown = render_markdown_benchmark_summary(&summary);
    artifacts.push(write_bundle_artifact(
        "benchmark_summary_markdown",
        "generated",
        out_dir,
        "benchmark-summary.md",
        summary_markdown.as_bytes(),
        true,
    )?);

    let manifest = EvidenceBundleManifest {
        schema_version: EVIDENCE_BUNDLE_SCHEMA_VERSION,
        suite_name: suite.name,
        baseline_agent: base_report.agent,
        baseline_variant: base_report.variant,
        evidence_use,
        artifacts,
        privacy: PrivacyStatus::source_free(),
    };
    write_json(&manifest, &out_dir.join("manifest.json"))?;
    Ok(())
}

fn read_public_suite_health(path: &Path) -> Result<PublicSuiteHealth> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str::<PublicSuiteHealth>(&raw)
        .with_context(|| format!("parse health {}", path.display()))
}

fn validate_public_suite_health_report(health: &PublicSuiteHealth) -> Result<()> {
    if health.schema_version != SUITE_HEALTH_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported health schema version {}; expected {}",
            health.schema_version,
            SUITE_HEALTH_SCHEMA_VERSION
        );
    }
    if !health.privacy.source_free
        || health.privacy.raw_source_logged
        || health.privacy.raw_prompt_logged
        || health.privacy.raw_transcript_logged
        || health.privacy.raw_terminal_logged
    {
        anyhow::bail!("health report is not source-free");
    }
    if health.repo_name.contains('/') || health.repo_name.contains('\\') {
        anyhow::bail!("health repoName must not contain path separators");
    }
    for checked in health
        .checked_files
        .iter()
        .chain(health.missing_files.iter())
        .chain(health.missing_expected_files.iter())
        .chain(health.missing_expected_tests.iter())
    {
        helmbench::validate_safe_relative_path_for_cli(checked)?;
    }
    for result in &health.baseline_success_commands {
        if result.task_id.trim().is_empty() {
            anyhow::bail!("health baseline command taskId must not be empty");
        }
        if !result.command_hash.starts_with("cmd:") {
            anyhow::bail!(
                "health baseline command for `{}` has unsupported commandHash `{}`",
                result.task_id,
                result.command_hash
            );
        }
    }
    Ok(())
}

fn copy_bundle_artifact(
    kind: &str,
    source: &Path,
    out_dir: &Path,
    relative_out: &str,
    source_free_checked: bool,
) -> Result<EvidenceBundleArtifact> {
    let bytes = std::fs::read(source).with_context(|| format!("read {}", source.display()))?;
    write_bundle_artifact(
        kind,
        source
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("artifact"),
        out_dir,
        relative_out,
        &bytes,
        source_free_checked,
    )
}

fn write_bundle_artifact(
    kind: &str,
    source_name: &str,
    out_dir: &Path,
    relative_out: &str,
    bytes: &[u8],
    source_free_checked: bool,
) -> Result<EvidenceBundleArtifact> {
    helmbench::validate_safe_relative_path_for_cli(relative_out)?;
    let path = out_dir.join(relative_out);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&path, bytes).with_context(|| format!("write {}", path.display()))?;
    Ok(EvidenceBundleArtifact {
        kind: kind.to_string(),
        path: relative_out.to_string(),
        source_name: source_name.to_string(),
        byte_count: bytes.len() as u64,
        content_hash: content_hash(bytes),
        source_free_checked,
    })
}

fn content_hash(bytes: &[u8]) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in bytes {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("fnv64:{hash:016x}")
}

fn init_public_suite(
    preset: PublicSuitePreset,
    repo: &Path,
    suite_out: &Path,
    health_out: &Path,
    min_commits: u64,
    force: bool,
) -> Result<()> {
    ensure_output_path_available(suite_out, force)?;
    ensure_output_path_available(health_out, force)?;

    let suite = public_suite_for_preset(preset);
    validate_suite(&suite)?;

    let health = public_suite_health(preset, repo, min_commits, &suite)?;
    write_json(&health, health_out)?;
    if !health.ok {
        anyhow::bail!(
            "public suite fixture is not healthy; wrote source-free health report to {}",
            health_out.display()
        );
    }

    write_json(&suite, suite_out)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct InitPublicMatrixOptions {
    preset: PublicSuitePreset,
    repo: PathBuf,
    suite_path: Option<PathBuf>,
    out: PathBuf,
    out_dir: Option<PathBuf>,
    health_out: Option<PathBuf>,
    agent_preset: AdapterPreset,
    agent_bin: Option<PathBuf>,
    model: Option<String>,
    agent_args: Vec<String>,
    dangerously_skip_permissions: bool,
    dangerously_bypass_approvals_and_sandbox: bool,
    ctxhelm_bin: PathBuf,
    mode: String,
    target_agent: Option<String>,
    pack: bool,
    fail_on_regression: bool,
    health_min_commits: u64,
    allow_dirty_health: bool,
    health_check_success_commands: bool,
    health_fail_fast_success_commands: bool,
    health_require_setup_commands: bool,
    force: bool,
}

fn init_public_matrix_config(options: InitPublicMatrixOptions) -> Result<()> {
    ensure_output_path_available(&options.out, options.force)?;
    if let Some(health_out) = &options.health_out {
        ensure_output_path_available(health_out, options.force)?;
    }
    let suite_path = options
        .suite_path
        .unwrap_or_else(|| default_public_suite_out(options.preset));
    let suite = load_suite(&suite_path)
        .with_context(|| format!("load public suite {}", suite_path.display()))?;
    validate_suite(&suite)?;

    let expected_suite = public_suite_for_preset(options.preset);
    if suite.name != expected_suite.name {
        anyhow::bail!(
            "suite `{}` does not match preset `{}` expected suite `{}`",
            suite.name,
            public_suite_preset_name(options.preset),
            expected_suite.name
        );
    }

    let health = suite_health_report(
        Some(public_suite_preset_name(options.preset)),
        &options.repo,
        options.health_min_commits,
        options.allow_dirty_health,
        options.health_check_success_commands,
        options.health_fail_fast_success_commands,
        options.health_require_setup_commands,
        &suite,
        public_suite_anchor_files(options.preset),
    )?;
    if let Some(health_out) = &options.health_out {
        write_json(&health, health_out)?;
        println!("wrote {}", health_out.display());
    }
    if !health.ok {
        anyhow::bail!(
            "public matrix fixture is not healthy; run suite-health for details before writing {}",
            options.out.display()
        );
    }

    let agent = adapter_preset_label(Some(options.agent_preset))
        .expect("adapter preset label")
        .to_string();
    let out_dir = options
        .out_dir
        .unwrap_or_else(|| default_public_matrix_out_dir(options.preset));
    let target_agent = options.target_agent.unwrap_or_else(|| agent.clone());
    let base = public_matrix_agent_spec(
        "native",
        &agent,
        AgentVariant::Native,
        options.agent_preset,
        options.agent_bin.clone(),
        options.model.clone(),
        options.agent_args.clone(),
        options.dangerously_skip_permissions,
        options.dangerously_bypass_approvals_and_sandbox,
    );
    let mut head = public_matrix_agent_spec(
        "ctxhelm",
        &agent,
        AgentVariant::CtxhelmMcp,
        options.agent_preset,
        options.agent_bin,
        options.model,
        options.agent_args,
        options.dangerously_skip_permissions,
        options.dangerously_bypass_approvals_and_sandbox,
    );
    head.ctxhelm = true;
    head.ctxhelm_bin = Some(options.ctxhelm_bin);
    head.mode = Some(options.mode);
    head.target_agent = Some(target_agent);
    head.pack = options.pack;
    head.pack_budget = options.pack.then(|| "brief".to_string());

    let config = RunMatrixConfig {
        suite: Some(suite_path),
        repo: Some(options.repo),
        out_dir: Some(out_dir),
        setup_commands: Vec::new(),
        baseline: base,
        heads: vec![head],
        keep_workdirs: None,
        fail_on_regression: Some(options.fail_on_regression),
        quality_gate: Some(RunMatrixQualityGateConfig {
            min_task_count: Some(suite.tasks.len()),
            ..RunMatrixQualityGateConfig::default()
        }),
        health_min_commits: Some(options.health_min_commits),
        allow_dirty_health: Some(options.allow_dirty_health),
        health_check_success_commands: options.health_check_success_commands.then_some(true),
        health_fail_fast_success_commands: options
            .health_fail_fast_success_commands
            .then_some(true),
        health_require_setup_commands: options.health_require_setup_commands.then_some(true),
    };

    write_json(&config, &options.out)?;
    println!("wrote {}", options.out.display());
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn public_matrix_agent_spec(
    name: &str,
    agent: &str,
    variant: AgentVariant,
    preset: AdapterPreset,
    bin: Option<PathBuf>,
    model: Option<String>,
    args: Vec<String>,
    dangerously_skip_permissions: bool,
    dangerously_bypass_approvals_and_sandbox: bool,
) -> RunMatrixConfigSpec {
    RunMatrixConfigSpec {
        name: name.to_string(),
        agent: agent.to_string(),
        variant,
        ctxhelm: false,
        ctxhelm_bin: None,
        mode: None,
        target_agent: None,
        semantic: false,
        semantic_provider: None,
        semantic_model: None,
        semantic_dimensions: None,
        pack: false,
        pack_budget: None,
        command: None,
        preset: Some(preset),
        bin,
        model,
        args,
        dangerously_skip_permissions,
        dangerously_bypass_approvals_and_sandbox,
        capture_stream: false,
    }
}

fn ensure_output_path_available(path: &Path, force: bool) -> Result<()> {
    if path.exists() && !force {
        anyhow::bail!(
            "{} already exists; pass --force to replace it",
            path.display()
        );
    }
    Ok(())
}

fn public_suite_health(
    preset: PublicSuitePreset,
    repo: &Path,
    min_commits: u64,
    suite: &helmbench::TaskSuite,
) -> Result<PublicSuiteHealth> {
    suite_health_report(
        Some(public_suite_preset_name(preset)),
        repo,
        min_commits,
        false,
        false,
        false,
        false,
        suite,
        public_suite_anchor_files(preset),
    )
}

#[allow(clippy::too_many_arguments)]
fn suite_health_report(
    preset: Option<&str>,
    repo: &Path,
    min_commits: u64,
    allow_dirty: bool,
    check_success_commands: bool,
    fail_fast_success_commands: bool,
    require_setup_commands: bool,
    suite: &helmbench::TaskSuite,
    anchor_files: &[&str],
) -> Result<PublicSuiteHealth> {
    let repo_name = repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();
    let checked_files = checked_files_for_suite_with_anchors(anchor_files, suite);
    let missing_files = checked_files
        .iter()
        .filter(|path| !repo.join(path).exists())
        .cloned()
        .collect::<Vec<_>>();
    let missing_expected_files = suite
        .tasks
        .iter()
        .flat_map(|task| task.expected_files.iter())
        .filter(|path| !repo.join(path).exists())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let missing_expected_tests = suite
        .tasks
        .iter()
        .flat_map(|task| task.expected_tests.iter())
        .filter(|path| !repo.join(path).exists())
        .cloned()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let tasks_missing_success_command = suite
        .tasks
        .iter()
        .filter(|task| {
            task.success_command
                .as_deref()
                .is_none_or(|command| command.trim().is_empty())
        })
        .map(|task| task.id.clone())
        .collect::<Vec<_>>();
    let tasks_missing_setup_command = if require_setup_commands {
        suite
            .tasks
            .iter()
            .filter(|task| task.setup_commands.is_empty())
            .map(|task| task.id.clone())
            .collect::<Vec<_>>()
    } else {
        Vec::new()
    };
    let expected_file_count = suite
        .tasks
        .iter()
        .map(|task| task.expected_files.len())
        .sum::<usize>();
    let expected_test_count = suite
        .tasks
        .iter()
        .map(|task| task.expected_tests.len())
        .sum::<usize>();
    let validation_ready = tasks_missing_success_command.is_empty();
    let setup_command_ready = !require_setup_commands || tasks_missing_setup_command.is_empty();
    let (baseline_success_commands, tasks_failed_setup_command) = if check_success_commands {
        success_command_baseline(repo, suite, fail_fast_success_commands)?
    } else {
        (Vec::new(), Vec::new())
    };
    let baseline_success_command_pass_count = baseline_success_commands
        .iter()
        .filter(|result| !result.timed_out && result.exit_status == Some(0))
        .count();
    let baseline_success_command_timeout_count = baseline_success_commands
        .iter()
        .filter(|result| result.timed_out)
        .count();
    let baseline_success_command_skipped_count = if check_success_commands {
        suite
            .tasks
            .len()
            .saturating_sub(baseline_success_commands.len())
    } else {
        0
    };
    let baseline_success_command_fail_count = baseline_success_commands
        .len()
        .saturating_sub(baseline_success_command_pass_count)
        .saturating_sub(baseline_success_command_timeout_count);
    let success_command_check_ready = check_success_commands
        && validation_ready
        && baseline_success_command_skipped_count == 0
        && tasks_failed_setup_command.is_empty();
    let validation_baseline_ready = success_command_check_ready
        && baseline_success_command_fail_count == suite.tasks.len()
        && baseline_success_command_pass_count == 0
        && baseline_success_command_timeout_count == 0;

    let head = git_output(repo, &["rev-parse", "HEAD"]).ok();
    let commit_count = git_output(repo, &["rev-list", "--count", "HEAD"])
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let dirty = git_output(repo, &["status", "--short"])
        .map(|status| !status.trim().is_empty())
        .unwrap_or(true);
    let fsck_ok = git_status_ok(repo, &["fsck", "--no-progress"]);
    let fixture_health_ok = repo.join(".git").exists()
        && head.is_some()
        && commit_count.is_some_and(|count| count >= min_commits)
        && (!dirty || allow_dirty)
        && fsck_ok
        && validation_ready
        && setup_command_ready
        && tasks_failed_setup_command.is_empty()
        && missing_files.is_empty()
        && missing_expected_files.is_empty()
        && missing_expected_tests.is_empty();
    let evidence_use = if !fixture_health_ok {
        SuiteEvidenceUse::Unhealthy
    } else if check_success_commands && validation_baseline_ready {
        SuiteEvidenceUse::OutcomeReady
    } else {
        SuiteEvidenceUse::NavigationOnly
    };
    let ok = fixture_health_ok && (!check_success_commands || validation_baseline_ready);

    Ok(PublicSuiteHealth {
        schema_version: SUITE_HEALTH_SCHEMA_VERSION,
        preset: preset.unwrap_or("custom").to_string(),
        suite_name: suite.name.clone(),
        task_count: suite.tasks.len(),
        repo_name,
        head,
        commit_count,
        min_commits,
        allow_dirty,
        dirty,
        fsck_ok,
        validation_ready,
        expected_file_count,
        expected_test_count,
        checked_files,
        missing_files,
        missing_expected_files,
        missing_expected_tests,
        setup_commands_required: require_setup_commands,
        setup_command_ready,
        tasks_missing_setup_command,
        tasks_failed_setup_command,
        tasks_missing_success_command,
        success_command_check_requested: check_success_commands,
        success_command_check_fail_fast: fail_fast_success_commands,
        success_command_check_ready,
        validation_baseline_ready,
        baseline_success_command_pass_count,
        baseline_success_command_fail_count,
        baseline_success_command_timeout_count,
        baseline_success_command_skipped_count,
        baseline_success_commands,
        evidence_use,
        ok,
        privacy: PrivacyStatus::source_free(),
    })
}

fn success_command_baseline(
    repo: &Path,
    suite: &helmbench::TaskSuite,
    fail_fast_on_pass: bool,
) -> Result<(Vec<SuccessCommandBaseline>, Vec<String>)> {
    let temp = TempDirGuard::create("helmbench-success-baseline")?;
    let mut results = Vec::new();
    let mut tasks_failed_setup_command = Vec::new();
    for task in &suite.tasks {
        let Some(command) = task
            .success_command
            .as_deref()
            .filter(|command| !command.trim().is_empty())
        else {
            continue;
        };
        let task_dir = temp.path().join(safe_task_dir_name(&task.id));
        clone_repo(repo, &task_dir)?;
        if !run_task_setup_commands(task, &task_dir, true)? {
            tasks_failed_setup_command.push(task.id.clone());
            continue;
        }
        let result = run_shell_suppressed(command, &task_dir, &[], task.timeout_seconds)?;
        let command_passed = !result.timed_out && result.exit_status == Some(0);
        results.push(SuccessCommandBaseline {
            task_id: task.id.clone(),
            command_class: infer_command_class(command),
            command_hash: command_hash(command),
            exit_status: result.exit_status,
            elapsed_millis: result.elapsed_millis,
            timed_out: result.timed_out,
        });
        if fail_fast_on_pass && command_passed {
            break;
        }
    }
    Ok((results, tasks_failed_setup_command))
}

fn run_task_setup_commands(
    task: &helmbench::BenchTask,
    task_dir: &Path,
    suppress_output: bool,
) -> Result<bool> {
    for setup in &task.setup_commands {
        let result = if suppress_output {
            run_shell_suppressed(setup, task_dir, &[], task.timeout_seconds)?
        } else {
            run_shell(setup, task_dir, &[], task.timeout_seconds)?
        };
        if !result.success {
            return Ok(false);
        }
    }
    Ok(true)
}

struct TempDirGuard {
    path: PathBuf,
}

impl TempDirGuard {
    fn create(prefix: &str) -> Result<Self> {
        for attempt in 0..100 {
            let nanos = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map(|duration| duration.as_nanos())
                .unwrap_or_default();
            let path = std::env::temp_dir()
                .join(format!("{prefix}-{}-{nanos}-{attempt}", std::process::id()));
            match std::fs::create_dir(&path) {
                Ok(()) => return Ok(Self { path }),
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => {
                    return Err(error)
                        .with_context(|| format!("create temp dir {}", path.display()));
                }
            }
        }
        anyhow::bail!("could not create unique temp dir for {prefix}")
    }

    fn path(&self) -> &Path {
        &self.path
    }
}

impl Drop for TempDirGuard {
    fn drop(&mut self) {
        let _ = std::fs::remove_dir_all(&self.path);
    }
}

fn public_suite_for_preset(preset: PublicSuitePreset) -> helmbench::TaskSuite {
    match preset {
        PublicSuitePreset::RefactoringMiner => refactoring_miner_suite(),
        PublicSuitePreset::Flask => flask_suite(),
        PublicSuitePreset::Ripgrep => ripgrep_suite(),
        PublicSuitePreset::Express => express_suite(),
    }
}

fn public_suite_preset_name(preset: PublicSuitePreset) -> &'static str {
    match preset {
        PublicSuitePreset::RefactoringMiner => "refactoring-miner",
        PublicSuitePreset::Flask => "flask",
        PublicSuitePreset::Ripgrep => "ripgrep",
        PublicSuitePreset::Express => "express",
    }
}

fn default_public_suite_out(preset: PublicSuitePreset) -> PathBuf {
    PathBuf::from(format!(
        "suites/{}-public.json",
        public_suite_preset_name(preset)
    ))
}

fn default_public_health_out(preset: PublicSuitePreset) -> PathBuf {
    PathBuf::from(format!(
        ".helmbench/{}-public-suite-health.json",
        public_suite_preset_name(preset)
    ))
}

fn default_public_matrix_config_out(preset: PublicSuitePreset) -> PathBuf {
    PathBuf::from(format!(
        ".helmbench/{}-matrix.json",
        public_suite_preset_name(preset)
    ))
}

fn default_public_matrix_out_dir(preset: PublicSuitePreset) -> PathBuf {
    PathBuf::from(format!(
        ".helmbench/{}-matrix",
        public_suite_preset_name(preset)
    ))
}

fn public_suite_anchor_files(preset: PublicSuitePreset) -> &'static [&'static str] {
    match preset {
        PublicSuitePreset::RefactoringMiner => &["README.md", "build.gradle", "gradlew"],
        PublicSuitePreset::Flask => &["README.md", "pyproject.toml", "src/flask/__init__.py"],
        PublicSuitePreset::Ripgrep => &["README.md", "Cargo.toml", "crates/cli/Cargo.toml"],
        PublicSuitePreset::Express => &["README.md", "package.json", "index.js"],
    }
}

#[cfg(test)]
fn checked_files_for_suite(preset: PublicSuitePreset, suite: &helmbench::TaskSuite) -> Vec<String> {
    checked_files_for_suite_with_anchors(public_suite_anchor_files(preset), suite)
}

fn checked_files_for_suite_with_anchors(
    anchor_files: &[&str],
    suite: &helmbench::TaskSuite,
) -> Vec<String> {
    let mut paths = anchor_files
        .iter()
        .map(|path| (*path).to_string())
        .collect::<Vec<_>>();
    for task in &suite.tasks {
        paths.extend(task.expected_files.iter().cloned());
        paths.extend(task.expected_tests.iter().cloned());
    }
    paths.sort();
    paths.dedup();
    paths
}

fn render_markdown_suite_health(health: &PublicSuiteHealth) -> String {
    let mut out = String::new();
    out.push_str("# Suite Health\n\n");
    out.push_str(&format!("Suite: `{}`\n\n", health.suite_name));
    out.push_str(&format!(
        "Status: **{}**\n\n",
        if health.ok { "healthy" } else { "unhealthy" }
    ));
    out.push_str("| Field | Value |\n| --- | --- |\n");
    out.push_str(&format!("| Preset | `{}` |\n", health.preset));
    out.push_str(&format!("| Repo | `{}` |\n", health.repo_name));
    out.push_str(&format!("| Tasks | {} |\n", health.task_count));
    out.push_str(&format!(
        "| Expected files / tests | {} / {} |\n",
        health.expected_file_count, health.expected_test_count
    ));
    out.push_str(&format!(
        "| Commit count | {} |\n",
        health
            .commit_count
            .map(|count| count.to_string())
            .unwrap_or_else(|| "unknown".to_string())
    ));
    out.push_str(&format!("| Minimum commits | {} |\n", health.min_commits));
    out.push_str(&format!("| Dirty checkout | {} |\n", yes_no(health.dirty)));
    out.push_str(&format!(
        "| Dirty allowed | {} |\n",
        yes_no(health.allow_dirty)
    ));
    out.push_str(&format!("| Git fsck ok | {} |\n", yes_no(health.fsck_ok)));
    out.push_str(&format!(
        "| Validation commands ready | {} |\n",
        yes_no(health.validation_ready)
    ));
    out.push_str(&format!(
        "| Setup commands required | {} |\n",
        yes_no(health.setup_commands_required)
    ));
    out.push_str(&format!(
        "| Setup commands ready | {} |\n",
        yes_no(health.setup_command_ready)
    ));
    out.push_str(&format!(
        "| Success commands checked | {} |\n",
        yes_no(health.success_command_check_requested)
    ));
    out.push_str(&format!(
        "| Success command fail-fast | {} |\n",
        yes_no(health.success_command_check_fail_fast)
    ));
    out.push_str(&format!(
        "| Validation baseline ready | {} |\n",
        yes_no(health.validation_baseline_ready)
    ));
    out.push_str(&format!("| Evidence use | `{}` |\n", health.evidence_use));
    if health.success_command_check_requested {
        out.push_str(&format!(
            "| Baseline success commands pass / fail / timeout / skipped | {} / {} / {} / {} |\n",
            health.baseline_success_command_pass_count,
            health.baseline_success_command_fail_count,
            health.baseline_success_command_timeout_count,
            health.baseline_success_command_skipped_count
        ));
    }
    out.push_str("\n## Missing Evidence\n\n");
    append_markdown_list(&mut out, "Missing files", &health.missing_files);
    append_markdown_list(
        &mut out,
        "Missing expected source files",
        &health.missing_expected_files,
    );
    append_markdown_list(
        &mut out,
        "Missing expected tests",
        &health.missing_expected_tests,
    );
    append_markdown_list(
        &mut out,
        "Tasks missing setup commands",
        &health.tasks_missing_setup_command,
    );
    append_markdown_list(
        &mut out,
        "Tasks failed setup commands",
        &health.tasks_failed_setup_command,
    );
    append_markdown_list(
        &mut out,
        "Tasks missing success commands",
        &health.tasks_missing_success_command,
    );
    if health.success_command_check_requested {
        out.push_str("\n## Validation Baseline\n\n");
        out.push_str(
            "A task-success benchmark is outcome-ready when validation commands fail before the agent runs and pass only after the agent's changes.\n\n",
        );
        out.push_str("| Task | Class | Status | Timed out | Elapsed ms |\n");
        out.push_str("| --- | --- | ---: | --- | ---: |\n");
        for result in &health.baseline_success_commands {
            out.push_str(&format!(
                "| `{}` | `{}` | {} | {} | {} |\n",
                result.task_id,
                command_class_label(&result.command_class),
                result
                    .exit_status
                    .map(|status| status.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                yes_no(result.timed_out),
                result.elapsed_millis
            ));
        }
        if health.baseline_success_commands.is_empty() {
            out.push_str("| none | `other` |  | no | 0 |\n");
        }
    }
    out.push_str("\n## Privacy\n\n");
    out.push_str("- Source-free: `true`\n");
    out.push_str("- Raw source logged: `false`\n");
    out.push_str("- Raw prompts logged: `false`\n");
    out.push_str("- Raw transcripts logged: `false`\n");
    out.push_str("- Raw terminal logs logged: `false`\n");
    out
}

fn command_class_label(class: &CommandClass) -> &'static str {
    match class {
        CommandClass::Test => "test",
        CommandClass::Build => "build",
        CommandClass::Lint => "lint",
        CommandClass::Typecheck => "typecheck",
        CommandClass::Other => "other",
    }
}

fn append_markdown_list(out: &mut String, title: &str, values: &[String]) {
    out.push_str(&format!("### {title}\n\n"));
    if values.is_empty() {
        out.push_str("- None\n\n");
    } else {
        for value in values {
            out.push_str(&format!("- `{value}`\n"));
        }
        out.push('\n');
    }
}

fn git_output(repo: &Path, args: &[&str]) -> Result<String> {
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("git {} {}", repo.display(), args.join(" ")))?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed with status {:?}",
            args.join(" "),
            output.status.code()
        );
    }
    String::from_utf8(output.stdout)
        .context("git stdout utf8")
        .map(|value| value.trim().to_string())
}

fn git_status_ok(repo: &Path, args: &[&str]) -> bool {
    ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .status()
        .map(|status| status.success())
        .unwrap_or(false)
}

fn refactoring_miner_suite() -> helmbench::TaskSuite {
    helmbench::TaskSuite {
        schema_version: helmbench::SUITE_SCHEMA_VERSION,
        name: "refactoringminer-public".to_string(),
        description: "Source-free public-repo suite for RefactoringMiner agent navigation, validation, and ctxhelm comparison.".to_string(),
        tasks: vec![
            helmbench::BenchTask {
                id: "rm-mcp-intent-validation-001".to_string(),
                prompt: "Tighten MCP refactoring-intent validation without changing unrelated server behavior.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/mcp/McpIntentValidator.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/McpValidationResult.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/mcp/McpIntentValidatorTest.java".to_string(),
                    "src/test/java/org/refactoringminer/mcp/McpValidationContractTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.mcp.McpIntentValidatorTest --tests org.refactoringminer.mcp.McpValidationContractTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "mcp".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rm-mcp-tools-contract-001".to_string(),
                prompt: "Update the MCP tools layer while preserving service contracts and source-free validation behavior.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpTools.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpService.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/WorktreeChangeCollector.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/mcp/RefactoringMinerMcpToolsTest.java".to_string(),
                    "src/test/java/org/refactoringminer/mcp/WorktreeChangeCollectorTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.mcp.RefactoringMinerMcpToolsTest --tests org.refactoringminer.mcp.WorktreeChangeCollectorTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "mcp".to_string(), "feature".to_string()],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rm-webdiff-viewed-files-001".to_string(),
                prompt: "Fix viewed-file tracking in the web diff UI without altering unrelated diff rendering.".to_string(),
                expected_files: vec![
                    "src/main/java/gui/MarkAsViewed.java".to_string(),
                    "src/main/java/gui/webdiff/viewers/spv/SinglePageView.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/gui/MarkAsViewedTest.java".to_string(),
                    "src/test/java/gui/webdiff/viewers/spv/SinglePageViewViewedFilesTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests gui.MarkAsViewedTest --tests gui.webdiff.viewers.spv.SinglePageViewViewedFilesTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "webdiff".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rm-git-history-merge-001".to_string(),
                prompt: "Improve merge-commit handling in git-history refactoring detection and keep existing merge tests targeted.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/rm1/GitHistoryRefactoringMinerImpl.java".to_string(),
                    "src/main/java/org/refactoringminer/util/GitServiceImpl.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/rm1/GitHistoryRefactoringMinerImplMergeCommitTest.java".to_string(),
                    "src/test/java/org/refactoringminer/util/GitServiceImplTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.rm1.GitHistoryRefactoringMinerImplMergeCommitTest --tests org.refactoringminer.util.GitServiceImplTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "git_history".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(1200),
            },
            helmbench::BenchTask {
                id: "rm-mcp-service-repository-001".to_string(),
                prompt: "Tighten MCP service repository handling while preserving analysis and refactoring result contracts.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpService.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/McpAnalysisResult.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/McpRefactoringResult.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/mcp/RefactoringMinerMcpServiceTest.java".to_string(),
                    "src/test/java/org/refactoringminer/mcp/RefactoringMinerMcpServiceRepositoryTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.mcp.RefactoringMinerMcpServiceTest --tests org.refactoringminer.mcp.RefactoringMinerMcpServiceRepositoryTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "mcp".to_string(), "repository".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rm-mcp-server-startup-001".to_string(),
                prompt: "Improve MCP server startup and web-diff launcher behavior without changing unrelated tool contracts.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/mcp/RefactoringMinerMcpServer.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/WebDiffBrowserLauncher.java".to_string(),
                    "src/main/java/org/refactoringminer/mcp/DiffBrowserLauncher.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/mcp/RefactoringMinerMcpServerStartupTest.java".to_string(),
                    "src/test/java/org/refactoringminer/mcp/WebDiffBrowserLauncherTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.mcp.RefactoringMinerMcpServerStartupTest --tests org.refactoringminer.mcp.WebDiffBrowserLauncherTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "mcp".to_string(), "startup".to_string(), "feature".to_string()],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rm-astdiff-comments-001".to_string(),
                prompt: "Adjust AST diff comment handling while keeping comment-aware and comment-ignoring visitor behavior targeted.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/astDiff/visitors/JdtVisitor.java".to_string(),
                    "src/main/java/org/refactoringminer/astDiff/visitors/JdtWithCommentsVisitor.java".to_string(),
                    "src/main/java/org/refactoringminer/astDiff/matchers/ProjectASTDiffer.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/astDiff/tests/ConsideringCommentsVisitorTest.java".to_string(),
                    "src/test/java/org/refactoringminer/astDiff/tests/IgnoringCommentsVisitorTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.astDiff.tests.ConsideringCommentsVisitorTest --tests org.refactoringminer.astDiff.tests.IgnoringCommentsVisitorTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "ast_diff".to_string(), "comments".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(1200),
            },
            helmbench::BenchTask {
                id: "rm-astdiff-python-001".to_string(),
                prompt: "Improve Python AST diff parsing or stringification while preserving parser tree regression coverage.".to_string(),
                expected_files: vec![
                    "src/main/java/extension/ast/builder/python/PyASTBuilder.java".to_string(),
                    "src/main/java/extension/ast/stringifier/PyASTFlattener.java".to_string(),
                    "src/main/java/extension/base/LangSupportedEnum.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/astDiff/tests/PythonDiffTest.java".to_string(),
                    "src/test/java/org/refactoringminer/astDiff/tests/TreeFromParserTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.astDiff.tests.PythonDiffTest --tests org.refactoringminer.astDiff.tests.TreeFromParserTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "ast_diff".to_string(), "python".to_string(), "feature".to_string()],
                timeout_seconds: Some(1200),
            },
            helmbench::BenchTask {
                id: "rm-astdiff-matchers-001".to_string(),
                prompt: "Tune AST tree matching behavior while preserving specific-case and matcher regression coverage.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/astDiff/matchers/TreeMatcher.java".to_string(),
                    "src/main/java/org/refactoringminer/astDiff/matchers/statement/CompositeMatcher.java".to_string(),
                    "src/main/java/org/refactoringminer/astDiff/matchers/statement/LeafMatcher.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/astDiff/tests/TreeMatcherTest.java".to_string(),
                    "src/test/java/org/refactoringminer/astDiff/tests/SpecificCasesTest.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.astDiff.tests.TreeMatcherTest --tests org.refactoringminer.astDiff.tests.SpecificCasesTest".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "ast_diff".to_string(), "matcher".to_string(), "refactor".to_string()],
                timeout_seconds: Some(1200),
            },
            helmbench::BenchTask {
                id: "rm-cli-command-line-001".to_string(),
                prompt: "Improve command-line refactoring detection behavior without changing unrelated API contracts.".to_string(),
                expected_files: vec![
                    "src/main/java/org/refactoringminer/RefactoringMiner.java".to_string(),
                    "src/main/java/org/refactoringminer/api/GitHistoryRefactoringMiner.java".to_string(),
                ],
                expected_tests: vec![
                    "src/test/java/org/refactoringminer/test/TestCommandLine.java".to_string(),
                ],
                success_command: Some("./gradlew test --tests org.refactoringminer.test.TestCommandLine".to_string()),
                setup_commands: Vec::new(),
                tags: vec!["public_repo".to_string(), "cli".to_string(), "git_history".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(1200),
            },
        ],
    }
}

fn flask_suite() -> helmbench::TaskSuite {
    helmbench::TaskSuite {
        schema_version: helmbench::SUITE_SCHEMA_VERSION,
        name: "flask-public".to_string(),
        description:
            "Source-free public-repo suite for Flask agent navigation, validation, and ctxhelm comparison."
                .to_string(),
        tasks: vec![
            helmbench::BenchTask {
                id: "flask-config-loading-001".to_string(),
                prompt: "Improve configuration loading behavior while preserving file, object, envvar, and prefixed-environment contracts.".to_string(),
                expected_files: vec![
                    "src/flask/config.py".to_string(),
                    "src/flask/app.py".to_string(),
                ],
                expected_tests: vec![
                    "tests/test_config.py".to_string(),
                    "tests/test_instance_config.py".to_string(),
                ],
                success_command: Some(
                    "python -m pytest tests/test_config.py tests/test_instance_config.py"
                        .to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "python".to_string(),
                    "config".to_string(),
                    "bug_fix".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "flask-blueprint-routing-001".to_string(),
                prompt: "Update blueprint registration or routing behavior without breaking nested blueprint and endpoint validation tests.".to_string(),
                expected_files: vec![
                    "src/flask/blueprints.py".to_string(),
                    "src/flask/sansio/blueprints.py".to_string(),
                    "src/flask/app.py".to_string(),
                ],
                expected_tests: vec![
                    "tests/test_blueprints.py".to_string(),
                    "tests/test_basic.py".to_string(),
                ],
                success_command: Some(
                    "python -m pytest tests/test_blueprints.py tests/test_basic.py".to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "python".to_string(),
                    "routing".to_string(),
                    "refactor".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "flask-template-context-001".to_string(),
                prompt: "Fix template context or rendering behavior while keeping escaping, context processor, and loader behavior targeted.".to_string(),
                expected_files: vec![
                    "src/flask/templating.py".to_string(),
                    "src/flask/helpers.py".to_string(),
                ],
                expected_tests: vec!["tests/test_templating.py".to_string()],
                success_command: Some("python -m pytest tests/test_templating.py".to_string()),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "python".to_string(),
                    "templating".to_string(),
                    "bug_fix".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "flask-cli-discovery-001".to_string(),
                prompt: "Improve CLI app discovery or command behavior without changing unrelated application runtime behavior.".to_string(),
                expected_files: vec![
                    "src/flask/cli.py".to_string(),
                    "src/flask/app.py".to_string(),
                ],
                expected_tests: vec!["tests/test_cli.py".to_string()],
                success_command: Some("python -m pytest tests/test_cli.py".to_string()),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "python".to_string(),
                    "cli".to_string(),
                    "feature".to_string(),
                ],
                timeout_seconds: Some(900),
            },
        ],
    }
}

fn ripgrep_suite() -> helmbench::TaskSuite {
    helmbench::TaskSuite {
        schema_version: helmbench::SUITE_SCHEMA_VERSION,
        name: "ripgrep-public".to_string(),
        description:
            "Source-free public-repo suite for ripgrep Rust CLI navigation, validation, and ctxhelm comparison."
                .to_string(),
        tasks: vec![
            helmbench::BenchTask {
                id: "rg-ignore-walk-001".to_string(),
                prompt: "Fix ignore-file walking behavior without changing unrelated directory traversal semantics.".to_string(),
                expected_files: vec![
                    "crates/ignore/src/gitignore.rs".to_string(),
                    "crates/ignore/src/walk.rs".to_string(),
                ],
                expected_tests: vec![
                    "crates/ignore/tests/gitignore_matched_path_or_any_parents_tests.rs"
                        .to_string(),
                    "crates/ignore/tests/gitignore_skip_bom.rs".to_string(),
                ],
                success_command: Some("cargo test -p ignore gitignore".to_string()),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "rust".to_string(),
                    "ignore".to_string(),
                    "bug_fix".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rg-cli-pattern-001".to_string(),
                prompt: "Improve CLI pattern parsing or escaping behavior while preserving existing feature and regression coverage.".to_string(),
                expected_files: vec![
                    "crates/cli/src/pattern.rs".to_string(),
                    "crates/cli/src/escape.rs".to_string(),
                ],
                expected_tests: vec![
                    "tests/feature.rs".to_string(),
                    "tests/regression.rs".to_string(),
                ],
                success_command: Some(
                    "cargo test -p ripgrep --test feature --test regression".to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "rust".to_string(),
                    "cli".to_string(),
                    "regression".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rg-json-printer-001".to_string(),
                prompt: "Adjust JSON printer behavior without leaking formatting changes into unrelated standard output modes.".to_string(),
                expected_files: vec![
                    "crates/printer/src/json.rs".to_string(),
                    "crates/printer/src/jsont.rs".to_string(),
                    "crates/printer/src/standard.rs".to_string(),
                ],
                expected_tests: vec!["tests/json.rs".to_string()],
                success_command: Some("cargo test -p ripgrep --test json".to_string()),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "rust".to_string(),
                    "json".to_string(),
                    "output".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "rg-searcher-multiline-001".to_string(),
                prompt: "Improve multiline search behavior while keeping searcher buffering and sink behavior targeted.".to_string(),
                expected_files: vec![
                    "crates/searcher/src/searcher/core.rs".to_string(),
                    "crates/searcher/src/line_buffer.rs".to_string(),
                    "crates/searcher/src/sink.rs".to_string(),
                ],
                expected_tests: vec![
                    "tests/multiline.rs".to_string(),
                    "tests/misc.rs".to_string(),
                ],
                success_command: Some(
                    "cargo test -p ripgrep --test multiline --test misc".to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "rust".to_string(),
                    "searcher".to_string(),
                    "bug_fix".to_string(),
                ],
                timeout_seconds: Some(900),
            },
        ],
    }
}

fn express_suite() -> helmbench::TaskSuite {
    helmbench::TaskSuite {
        schema_version: helmbench::SUITE_SCHEMA_VERSION,
        name: "express-public".to_string(),
        description:
            "Source-free public-repo suite for Express Node.js routing, request, response, and middleware behavior."
                .to_string(),
        tasks: vec![
            helmbench::BenchTask {
                id: "express-router-stack-001".to_string(),
                prompt: "Fix routing or middleware dispatch behavior while preserving app router and Router coverage.".to_string(),
                expected_files: vec![
                    "lib/application.js".to_string(),
                    "lib/express.js".to_string(),
                ],
                expected_tests: vec![
                    "test/app.router.js".to_string(),
                    "test/Router.js".to_string(),
                ],
                success_command: Some(
                    "npx mocha --require test/support/env --check-leaks test/app.router.js test/Router.js"
                        .to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "javascript".to_string(),
                    "node".to_string(),
                    "routing".to_string(),
                    "bug_fix".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "express-response-json-001".to_string(),
                prompt: "Improve JSON or JSONP response behavior without changing unrelated response helpers.".to_string(),
                expected_files: vec![
                    "lib/response.js".to_string(),
                    "lib/utils.js".to_string(),
                ],
                expected_tests: vec![
                    "test/res.json.js".to_string(),
                    "test/res.jsonp.js".to_string(),
                ],
                success_command: Some(
                    "npx mocha --require test/support/env --check-leaks test/res.json.js test/res.jsonp.js"
                        .to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "javascript".to_string(),
                    "node".to_string(),
                    "response".to_string(),
                    "regression".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "express-request-negotiation-001".to_string(),
                prompt: "Fix request negotiation, type, or range handling while keeping request helper behavior targeted.".to_string(),
                expected_files: vec![
                    "lib/request.js".to_string(),
                    "lib/utils.js".to_string(),
                ],
                expected_tests: vec![
                    "test/req.accepts.js".to_string(),
                    "test/req.is.js".to_string(),
                    "test/req.range.js".to_string(),
                ],
                success_command: Some(
                    "npx mocha --require test/support/env --check-leaks test/req.accepts.js test/req.is.js test/req.range.js"
                        .to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "javascript".to_string(),
                    "node".to_string(),
                    "request".to_string(),
                    "bug_fix".to_string(),
                ],
                timeout_seconds: Some(900),
            },
            helmbench::BenchTask {
                id: "express-static-and-body-001".to_string(),
                prompt: "Adjust static middleware or body parser integration while preserving Express factory behavior.".to_string(),
                expected_files: vec![
                    "lib/express.js".to_string(),
                    "lib/application.js".to_string(),
                ],
                expected_tests: vec![
                    "test/express.static.js".to_string(),
                    "test/express.json.js".to_string(),
                    "test/express.urlencoded.js".to_string(),
                    "test/middleware.basic.js".to_string(),
                ],
                success_command: Some(
                    "npx mocha --require test/support/env --check-leaks test/express.static.js test/express.json.js test/express.urlencoded.js test/middleware.basic.js"
                        .to_string(),
                ),
                setup_commands: Vec::new(),
                tags: vec![
                    "public_repo".to_string(),
                    "javascript".to_string(),
                    "node".to_string(),
                    "middleware".to_string(),
                    "feature".to_string(),
                ],
                timeout_seconds: Some(900),
            },
        ],
    }
}

fn run_demo_pipeline(out_dir: &Path, force: bool) -> Result<()> {
    run_demo_pipeline_with_adapter(out_dir, force, None)
}

#[derive(Debug, Clone)]
struct RunMatrixSpec {
    name: String,
    safe_name: String,
    agent: String,
    variant: AgentVariant,
    ctxhelm: Option<CtxhelmRunConfig>,
    adapter_command: Option<String>,
    adapter_preset: Option<AdapterPresetRunConfig>,
    capture_stream: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq, ValueEnum)]
#[serde(rename_all = "kebab-case")]
enum AdapterPreset {
    ClaudeCode,
    Codex,
}

#[derive(Debug, Clone)]
struct AdapterPresetRunConfig {
    preset: AdapterPreset,
    bin: PathBuf,
    model: Option<String>,
    args: Vec<String>,
    dangerously_skip_permissions: bool,
    dangerously_bypass_approvals_and_sandbox: bool,
}

struct RunMatrixResult {
    spec: RunMatrixSpec,
    report: helmbench::RunReport,
    report_path: PathBuf,
    trace_dir: PathBuf,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixManifest {
    schema_version: u32,
    suite_path: String,
    repo_path: String,
    out_dir: String,
    provenance: RunMatrixProvenance,
    #[serde(default = "default_suite_evidence_use")]
    suite_evidence_use: SuiteEvidenceUse,
    baseline: RunMatrixManifestRun,
    heads: Vec<RunMatrixManifestRun>,
    artifacts: RunMatrixManifestArtifacts,
    artifact_digests: Vec<MatrixArtifactDigest>,
    quality_gate_passed: bool,
    evidence_bundle_verified: bool,
    privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixProvenance {
    helmbench_version: String,
    suite_hash: String,
    repo_head: Option<String>,
    repo_dirty: bool,
    setup_command_count: usize,
    setup_command_hashes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixManifestRun {
    name: String,
    agent: String,
    variant: AgentVariant,
    report_path: String,
    trace_dir: String,
    autopsy_markdown: String,
    comparison_json: Option<String>,
    comparison_markdown: Option<String>,
    ctxhelm_enabled: bool,
    pack_enabled: bool,
    stream_capture_enabled: bool,
    adapter_preset: Option<AdapterPreset>,
    adapter_command_hash: Option<String>,
    ctxhelm_config_hash: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixManifestArtifacts {
    suite_health_json: String,
    benchmark_summary_json: String,
    benchmark_summary_markdown: String,
    quality_gate_json: String,
    quality_gate_markdown: String,
    privacy_report_json: String,
    privacy_report_markdown: String,
    dashboard_html: String,
    baseline_autopsy_markdown: String,
    reproduction_markdown: String,
    evidence_manifest: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixPrivacyReport {
    schema_version: u32,
    suite_name: String,
    task_count: usize,
    run_count: usize,
    trace_count: usize,
    recorded_metadata: Vec<String>,
    forbidden_raw_data: Vec<String>,
    safeguards: Vec<String>,
    runs: Vec<RunMatrixPrivacyRun>,
    privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixPrivacyRun {
    name: String,
    agent: String,
    variant: AgentVariant,
    report_path: String,
    trace_dir: String,
    trace_count: usize,
    source_free_trace_count: usize,
    report_source_free: bool,
    raw_source_logged: bool,
    raw_prompt_logged: bool,
    raw_transcript_logged: bool,
    raw_terminal_logged: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
struct MatrixArtifactDigest {
    path: String,
    byte_count: u64,
    content_hash: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixHistoryReport {
    schema_version: u32,
    suite_name: String,
    matrices: Vec<MatrixHistoryEntry>,
    trends: Vec<MatrixRunTrend>,
    privacy: PrivacyStatus,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixHistoryEntry {
    matrix_index: usize,
    label: String,
    quality_gate_passed: bool,
    evidence_bundle_verified: bool,
    runs: Vec<MatrixHistoryRun>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixHistoryRun {
    name: String,
    agent: String,
    variant: AgentVariant,
    task_count: usize,
    success_rate: f32,
    validation_coverage_rate: f32,
    irrelevant_read_rate: f32,
    recommendation_recall: f32,
    context_precision: f32,
    edited_file_recall: f32,
    average_time_to_first_relevant_file_millis: Option<f32>,
    total_tool_calls: u32,
    total_token_estimate: u64,
    tool_calls_per_success: Option<f32>,
    token_estimate_per_success: Option<f32>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct MatrixRunTrend {
    name: String,
    agent: String,
    variant: AgentVariant,
    first_success_rate: f32,
    last_success_rate: f32,
    success_rate_delta: f32,
    first_validation_coverage_rate: f32,
    last_validation_coverage_rate: f32,
    validation_coverage_rate_delta: f32,
    first_irrelevant_read_rate: f32,
    last_irrelevant_read_rate: f32,
    irrelevant_read_rate_delta: f32,
    first_recommendation_recall: f32,
    last_recommendation_recall: f32,
    recommendation_recall_delta: f32,
    first_context_precision: f32,
    last_context_precision: f32,
    context_precision_delta: f32,
    first_edited_file_recall: f32,
    last_edited_file_recall: f32,
    edited_file_recall_delta: f32,
    first_average_time_to_first_relevant_file_millis: Option<f32>,
    last_average_time_to_first_relevant_file_millis: Option<f32>,
    average_time_to_first_relevant_file_millis_delta: Option<f32>,
    total_tool_calls_delta: i64,
    total_token_estimate_delta: i64,
    first_tool_calls_per_success: Option<f32>,
    last_tool_calls_per_success: Option<f32>,
    tool_calls_per_success_delta: Option<f32>,
    first_token_estimate_per_success: Option<f32>,
    last_token_estimate_per_success: Option<f32>,
    token_estimate_per_success_delta: Option<f32>,
}

#[derive(Debug, Clone)]
struct RunMatrixRequest {
    suite_path: PathBuf,
    repo: PathBuf,
    out_dir: PathBuf,
    baseline: RunMatrixSpec,
    heads: Vec<RunMatrixSpec>,
    setup_commands: Vec<String>,
    force: bool,
    keep_workdirs: bool,
    fail_on_regression: bool,
    quality_gate_config: QualityGateConfig,
    health_min_commits: u64,
    allow_dirty_health: bool,
    health_check_success_commands: bool,
    health_fail_fast_success_commands: bool,
    health_require_setup_commands: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixConfig {
    #[serde(skip_serializing_if = "Option::is_none")]
    suite: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    repo: Option<PathBuf>,
    #[serde(skip_serializing_if = "Option::is_none")]
    out_dir: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    setup_commands: Vec<String>,
    baseline: RunMatrixConfigSpec,
    #[serde(default)]
    heads: Vec<RunMatrixConfigSpec>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    keep_workdirs: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    fail_on_regression: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    quality_gate: Option<RunMatrixQualityGateConfig>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    health_min_commits: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    allow_dirty_health: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    health_check_success_commands: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    health_fail_fast_success_commands: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    health_require_setup_commands: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixConfigSpec {
    name: String,
    agent: String,
    variant: AgentVariant,
    #[serde(default, skip_serializing_if = "is_false")]
    ctxhelm: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    ctxhelm_bin: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    mode: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    target_agent: Option<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    semantic: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    semantic_provider: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    semantic_model: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    semantic_dimensions: Option<u16>,
    #[serde(default, skip_serializing_if = "is_false")]
    pack: bool,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pack_budget: Option<String>,
    #[serde(
        default,
        alias = "adapterCommand",
        skip_serializing_if = "Option::is_none"
    )]
    command: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    preset: Option<AdapterPreset>,
    #[serde(default, alias = "adapterBin", skip_serializing_if = "Option::is_none")]
    bin: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    model: Option<String>,
    #[serde(default, alias = "adapterArgs", skip_serializing_if = "Vec::is_empty")]
    args: Vec<String>,
    #[serde(default, skip_serializing_if = "is_false")]
    dangerously_skip_permissions: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    dangerously_bypass_approvals_and_sandbox: bool,
    #[serde(default, skip_serializing_if = "is_false")]
    capture_stream: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct RunMatrixQualityGateConfig {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_task_count: Option<usize>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_success_rate_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_validation_coverage_rate_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_irrelevant_read_rate_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_recommendation_recall_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_context_precision_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    min_edited_file_recall_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_average_time_to_first_relevant_file_millis_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_total_tool_calls_delta: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_total_token_estimate_delta: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_tool_calls_per_success_delta: Option<f32>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    max_token_estimate_per_success_delta: Option<f32>,
}

fn is_false(value: &bool) -> bool {
    !*value
}

#[allow(clippy::too_many_arguments)]
fn build_run_matrix_request(
    config_path: Option<&Path>,
    suite: Option<PathBuf>,
    repo: Option<PathBuf>,
    out_dir: Option<PathBuf>,
    baseline: Option<String>,
    heads: Vec<String>,
    setup_commands: Vec<String>,
    force: bool,
    keep_workdirs: bool,
    fail_on_regression: bool,
    health_min_commits: u64,
    allow_dirty_health: bool,
    health_check_success_commands: bool,
    health_fail_fast_success_commands: bool,
    health_require_setup_commands: bool,
    min_task_count: Option<usize>,
    max_average_time_to_first_relevant_file_millis_delta: Option<f32>,
    max_total_tool_calls_delta: Option<i64>,
    max_total_token_estimate_delta: Option<i64>,
    max_tool_calls_per_success_delta: Option<f32>,
    max_token_estimate_per_success_delta: Option<f32>,
) -> Result<RunMatrixRequest> {
    let config = config_path
        .map(load_run_matrix_config)
        .transpose()
        .context("load run-matrix config")?;

    let suite_path = suite
        .or_else(|| config.as_ref().and_then(|config| config.suite.clone()))
        .context("run-matrix requires --suite or config.suite")?;
    let repo = repo
        .or_else(|| config.as_ref().and_then(|config| config.repo.clone()))
        .context("run-matrix requires --repo or config.repo")?;
    let out_dir = out_dir
        .or_else(|| config.as_ref().and_then(|config| config.out_dir.clone()))
        .unwrap_or_else(|| PathBuf::from(".helmbench/matrix"));

    let baseline = match (baseline, config.as_ref()) {
        (Some(raw), _) => parse_run_matrix_spec(&raw).context("parse --baseline")?,
        (None, Some(config)) => {
            run_matrix_spec_from_config(&config.baseline).context("parse config baseline")?
        }
        (None, None) => anyhow::bail!("run-matrix requires --baseline or config.baseline"),
    };
    let heads = if heads.is_empty() {
        let Some(config) = config.as_ref() else {
            anyhow::bail!("run-matrix requires --head or config.heads");
        };
        config
            .heads
            .iter()
            .map(run_matrix_spec_from_config)
            .collect::<Result<Vec<_>>>()
            .context("parse config heads")?
    } else {
        heads
            .iter()
            .map(|spec| {
                parse_run_matrix_spec(spec).with_context(|| format!("parse --head `{spec}`"))
            })
            .collect::<Result<Vec<_>>>()?
    };

    let mut merged_setup_commands = config
        .as_ref()
        .map(|config| config.setup_commands.clone())
        .unwrap_or_default();
    merged_setup_commands.extend(setup_commands);

    let keep_workdirs = keep_workdirs
        || config
            .as_ref()
            .and_then(|config| config.keep_workdirs)
            .unwrap_or(false);
    let fail_on_regression = fail_on_regression
        || config
            .as_ref()
            .and_then(|config| config.fail_on_regression)
            .unwrap_or(false);
    let health_min_commits = if health_min_commits != 1 {
        health_min_commits
    } else {
        config
            .as_ref()
            .and_then(|config| config.health_min_commits)
            .unwrap_or(1)
    };
    let allow_dirty_health = allow_dirty_health
        || config
            .as_ref()
            .and_then(|config| config.allow_dirty_health)
            .unwrap_or(false);
    let health_check_success_commands = health_check_success_commands
        || config
            .as_ref()
            .and_then(|config| config.health_check_success_commands)
            .unwrap_or(false);
    let health_fail_fast_success_commands = health_fail_fast_success_commands
        || config
            .as_ref()
            .and_then(|config| config.health_fail_fast_success_commands)
            .unwrap_or(false);
    let health_require_setup_commands = health_require_setup_commands
        || config
            .as_ref()
            .and_then(|config| config.health_require_setup_commands)
            .unwrap_or(false);
    let quality_gate_config = run_matrix_quality_gate_config(
        config
            .as_ref()
            .and_then(|config| config.quality_gate.as_ref()),
        min_task_count,
        max_average_time_to_first_relevant_file_millis_delta,
        max_total_tool_calls_delta,
        max_total_token_estimate_delta,
        max_tool_calls_per_success_delta,
        max_token_estimate_per_success_delta,
    );

    validate_run_matrix_specs(&baseline, &heads)?;
    Ok(RunMatrixRequest {
        suite_path,
        repo,
        out_dir,
        baseline,
        heads,
        setup_commands: merged_setup_commands,
        force,
        keep_workdirs,
        fail_on_regression,
        quality_gate_config,
        health_min_commits,
        allow_dirty_health,
        health_check_success_commands,
        health_fail_fast_success_commands,
        health_require_setup_commands,
    })
}

fn run_matrix_quality_gate_config(
    config: Option<&RunMatrixQualityGateConfig>,
    min_task_count: Option<usize>,
    max_average_time_to_first_relevant_file_millis_delta: Option<f32>,
    max_total_tool_calls_delta: Option<i64>,
    max_total_token_estimate_delta: Option<i64>,
    max_tool_calls_per_success_delta: Option<f32>,
    max_token_estimate_per_success_delta: Option<f32>,
) -> QualityGateConfig {
    let mut gate = QualityGateConfig::default();
    if let Some(config) = config {
        if let Some(value) = config.min_task_count {
            gate.min_task_count = Some(value);
        }
        if let Some(value) = config.min_success_rate_delta {
            gate.min_success_rate_delta = value;
        }
        if let Some(value) = config.min_validation_coverage_rate_delta {
            gate.min_validation_coverage_rate_delta = value;
        }
        if let Some(value) = config.max_irrelevant_read_rate_delta {
            gate.max_irrelevant_read_rate_delta = value;
        }
        if let Some(value) = config.min_recommendation_recall_delta {
            gate.min_recommendation_recall_delta = value;
        }
        if let Some(value) = config.min_context_precision_delta {
            gate.min_context_precision_delta = value;
        }
        if let Some(value) = config.min_edited_file_recall_delta {
            gate.min_edited_file_recall_delta = value;
        }
        if let Some(value) = config.max_average_time_to_first_relevant_file_millis_delta {
            gate.max_average_time_to_first_relevant_file_millis_delta = Some(value);
        }
        if let Some(value) = config.max_total_tool_calls_delta {
            gate.max_total_tool_calls_delta = Some(value);
        }
        if let Some(value) = config.max_total_token_estimate_delta {
            gate.max_total_token_estimate_delta = Some(value);
        }
        if let Some(value) = config.max_tool_calls_per_success_delta {
            gate.max_tool_calls_per_success_delta = Some(value);
        }
        if let Some(value) = config.max_token_estimate_per_success_delta {
            gate.max_token_estimate_per_success_delta = Some(value);
        }
    }
    if min_task_count.is_some() {
        gate.min_task_count = min_task_count;
    }
    if max_average_time_to_first_relevant_file_millis_delta.is_some() {
        gate.max_average_time_to_first_relevant_file_millis_delta =
            max_average_time_to_first_relevant_file_millis_delta;
    }
    if max_total_tool_calls_delta.is_some() {
        gate.max_total_tool_calls_delta = max_total_tool_calls_delta;
    }
    if max_total_token_estimate_delta.is_some() {
        gate.max_total_token_estimate_delta = max_total_token_estimate_delta;
    }
    if max_tool_calls_per_success_delta.is_some() {
        gate.max_tool_calls_per_success_delta = max_tool_calls_per_success_delta;
    }
    if max_token_estimate_per_success_delta.is_some() {
        gate.max_token_estimate_per_success_delta = max_token_estimate_per_success_delta;
    }
    gate
}

fn load_run_matrix_config(path: &Path) -> Result<RunMatrixConfig> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    serde_json::from_str::<RunMatrixConfig>(&raw)
        .with_context(|| format!("parse {}", path.display()))
}

fn run_matrix_spec_from_config(config: &RunMatrixConfigSpec) -> Result<RunMatrixSpec> {
    if config.name.trim().is_empty() {
        anyhow::bail!("run spec name must not be empty");
    }
    if config.agent.trim().is_empty() {
        anyhow::bail!("run spec agent must not be empty");
    }
    let safe_name = safe_task_dir_name(&config.name);
    let adapter_preset = config.preset.map(|preset| AdapterPresetRunConfig {
        preset,
        bin: config
            .bin
            .clone()
            .unwrap_or_else(|| default_preset_bin(preset)),
        model: config.model.clone(),
        args: config.args.clone(),
        dangerously_skip_permissions: config.dangerously_skip_permissions,
        dangerously_bypass_approvals_and_sandbox: config.dangerously_bypass_approvals_and_sandbox,
    });
    let adapter_command = adapter_command_from_preset(
        config.command.clone(),
        adapter_preset.as_ref(),
        config.capture_stream,
    )?;
    Ok(RunMatrixSpec {
        name: config.name.clone(),
        safe_name,
        agent: config.agent.clone(),
        variant: config.variant.clone(),
        ctxhelm: (config.ctxhelm
            || config.ctxhelm_bin.is_some()
            || config.mode.is_some()
            || config.target_agent.is_some()
            || config.semantic
            || config.semantic_provider.is_some()
            || config.semantic_model.is_some()
            || config.semantic_dimensions.is_some()
            || config.pack
            || config.pack_budget.is_some())
        .then_some(CtxhelmRunConfig {
            ctxhelm_bin: config
                .ctxhelm_bin
                .clone()
                .unwrap_or_else(|| PathBuf::from("ctxhelm")),
            mode: config.mode.clone().unwrap_or_else(|| "explain".to_string()),
            target_agent: config
                .target_agent
                .clone()
                .unwrap_or_else(|| "generic".to_string()),
            semantic: config.semantic,
            semantic_provider: config.semantic_provider.clone(),
            semantic_model: config.semantic_model.clone(),
            semantic_dimensions: config.semantic_dimensions,
            include_pack: config.pack,
            pack_budget: config
                .pack_budget
                .clone()
                .unwrap_or_else(|| "brief".to_string()),
        }),
        adapter_command,
        adapter_preset,
        capture_stream: config.capture_stream,
    })
}

fn validate_run_matrix_request(request: &RunMatrixRequest) -> Result<helmbench::TaskSuite> {
    let suite = load_suite(&request.suite_path)
        .with_context(|| format!("validate suite {}", request.suite_path.display()))?;
    let repo = std::fs::canonicalize(&request.repo)
        .with_context(|| format!("resolve repo {}", request.repo.display()))?;
    if !repo.join(".git").exists() {
        anyhow::bail!("run-matrix requires a git repository: {}", repo.display());
    }
    validate_run_matrix_specs(&request.baseline, &request.heads)?;
    Ok(suite)
}

fn run_matrix(request: &RunMatrixRequest) -> Result<()> {
    let out_dir = &request.out_dir;
    if out_dir.exists() {
        if !request.force {
            anyhow::bail!(
                "{} already exists; pass --force to replace it",
                out_dir.display()
            );
        }
        std::fs::remove_dir_all(out_dir)
            .with_context(|| format!("remove {}", out_dir.display()))?;
    }
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;

    let suite = validate_run_matrix_request(request)?;

    let traces_dir = out_dir.join("traces");
    let reports_dir = out_dir.join("reports");
    let docs_dir = out_dir.join("docs");
    let work_dir = out_dir.join("workdirs");
    std::fs::create_dir_all(&reports_dir)
        .with_context(|| format!("create {}", reports_dir.display()))?;
    std::fs::create_dir_all(&docs_dir).with_context(|| format!("create {}", docs_dir.display()))?;

    let suite_health = suite_health_report(
        None,
        &request.repo,
        request.health_min_commits,
        request.allow_dirty_health,
        request.health_check_success_commands,
        request.health_fail_fast_success_commands,
        request.health_require_setup_commands,
        &suite,
        &[],
    )?;
    let suite_health_json_path = reports_dir.join("suite-health.json");
    write_json(&suite_health, &suite_health_json_path)?;
    if !suite_health.ok {
        anyhow::bail!(
            "run-matrix suite health check failed; wrote source-free health report to {}",
            suite_health_json_path.display()
        );
    }

    let baseline_result = run_matrix_spec(
        &suite,
        &request.repo,
        &work_dir,
        &traces_dir,
        &reports_dir,
        &request.baseline,
        &request.setup_commands,
        request.keep_workdirs,
    )?;
    let head_results = request
        .heads
        .iter()
        .map(|spec| {
            run_matrix_spec(
                &suite,
                &request.repo,
                &work_dir,
                &traces_dir,
                &reports_dir,
                spec,
                &request.setup_commands,
                request.keep_workdirs,
            )
        })
        .collect::<Result<Vec<_>>>()?;

    let mut comparison_paths = BTreeMap::new();
    for head in &head_results {
        validate_comparable_reports(&baseline_result.report, &head.report)?;
        let compare = compare_reports(&baseline_result.report, &head.report);
        let compare_json_path = reports_dir.join(format!("compare-{}.json", head.spec.safe_name));
        let compare_markdown_path = docs_dir.join(format!("compare-{}.md", head.spec.safe_name));
        write_json(&compare, &compare_json_path)?;
        write_text(&render_markdown_compare(&compare), &compare_markdown_path)?;
        comparison_paths.insert(
            head.spec.safe_name.clone(),
            (compare_json_path, compare_markdown_path),
        );
    }

    let head_reports = head_results
        .iter()
        .map(|result| result.report.clone())
        .collect::<Vec<_>>();
    let summary = build_benchmark_summary(&baseline_result.report, &head_reports)?;
    let summary_json_path = reports_dir.join("benchmark-summary.json");
    write_json(&summary, &summary_json_path)?;
    write_text(
        &render_markdown_benchmark_summary(&summary),
        &docs_dir.join("benchmark-summary.md"),
    )?;

    let gate = evaluate_quality_gate(&summary, &request.quality_gate_config)?;
    let quality_gate_json_path = reports_dir.join("quality-gate.json");
    write_json(&gate, &quality_gate_json_path)?;
    let quality_gate_markdown_path = docs_dir.join("quality-gate.md");
    write_text(
        &render_markdown_quality_gate(&gate),
        &quality_gate_markdown_path,
    )?;

    let mut autopsy_paths = BTreeMap::new();
    let baseline_autopsy_path = write_matrix_autopsy(&suite, &docs_dir, &baseline_result)?;
    autopsy_paths.insert(
        baseline_result.spec.safe_name.clone(),
        baseline_autopsy_path.clone(),
    );
    for head in &head_results {
        autopsy_paths.insert(
            head.spec.safe_name.clone(),
            write_matrix_autopsy(&suite, &docs_dir, head)?,
        );
    }

    let all_reports = std::iter::once(baseline_result.report.clone())
        .chain(head_results.iter().map(|result| result.report.clone()))
        .collect::<Vec<_>>();
    let dashboard_path = docs_dir.join("dashboard.html");
    write_text(&render_html_dashboard(&all_reports)?, &dashboard_path)?;

    let all_results = std::iter::once(&baseline_result)
        .chain(head_results.iter())
        .collect::<Vec<_>>();
    let privacy_report = build_run_matrix_privacy_report(&suite, &request.out_dir, &all_results)?;
    let privacy_report_json_path = reports_dir.join("privacy-report.json");
    write_json(&privacy_report, &privacy_report_json_path)?;
    let privacy_report_markdown_path = docs_dir.join("privacy-report.md");
    write_text(
        &render_markdown_run_matrix_privacy_report(&privacy_report),
        &privacy_report_markdown_path,
    )?;

    let head_report_paths = head_results
        .iter()
        .map(|result| result.report_path.clone())
        .collect::<Vec<_>>();
    let evidence_dir = out_dir.join("evidence");
    write_evidence_bundle(
        &request.suite_path,
        Some(&suite_health_json_path),
        &baseline_result.report_path,
        &head_report_paths,
        &evidence_dir,
        false,
    )?;
    verify_evidence_bundle(&evidence_dir)?;

    let reproduction_markdown_path = docs_dir.join("reproduction.md");
    let manifest = build_run_matrix_manifest(
        request,
        &baseline_result,
        &head_results,
        &suite_health_json_path,
        &summary_json_path,
        &docs_dir.join("benchmark-summary.md"),
        &quality_gate_json_path,
        &quality_gate_markdown_path,
        &privacy_report_json_path,
        &privacy_report_markdown_path,
        &dashboard_path,
        &autopsy_paths,
        &comparison_paths,
        &baseline_autopsy_path,
        &reproduction_markdown_path,
        &evidence_dir.join("manifest.json"),
        gate.passed,
        true,
    )?;
    write_text(
        &render_markdown_matrix_reproduction(&manifest),
        &reproduction_markdown_path,
    )?;
    let mut manifest = manifest;
    manifest.artifact_digests = collect_matrix_artifact_digests(&request.out_dir, &manifest)?;
    write_json(&manifest, &out_dir.join("matrix-manifest.json"))?;

    if request.fail_on_regression && !gate.passed {
        anyhow::bail!("run-matrix quality gate failed");
    }

    Ok(())
}

fn write_matrix_autopsy(
    suite: &helmbench::TaskSuite,
    docs_dir: &Path,
    result: &RunMatrixResult,
) -> Result<PathBuf> {
    let traces = load_traces(&result.trace_dir)?;
    let autopsy = build_autopsy(suite, &traces)?;
    let path = docs_dir.join(format!("{}-autopsy.md", result.spec.safe_name));
    write_text(&render_markdown_autopsy(&autopsy), &path)?;
    Ok(path)
}

fn build_run_matrix_privacy_report(
    suite: &helmbench::TaskSuite,
    out_dir: &Path,
    results: &[&RunMatrixResult],
) -> Result<RunMatrixPrivacyReport> {
    let mut runs = Vec::with_capacity(results.len());
    let mut trace_count = 0usize;

    for result in results {
        let traces = load_traces(&result.trace_dir)?;
        let source_free_trace_count = traces
            .iter()
            .filter(|trace| privacy_status_is_source_free(&trace.privacy))
            .count();
        trace_count += traces.len();
        runs.push(RunMatrixPrivacyRun {
            name: result.spec.name.clone(),
            agent: result.spec.agent.clone(),
            variant: result.spec.variant.clone(),
            report_path: manifest_path(out_dir, &result.report_path),
            trace_dir: manifest_path(out_dir, &result.trace_dir),
            trace_count: traces.len(),
            source_free_trace_count,
            report_source_free: privacy_status_is_source_free(&result.report.privacy),
            raw_source_logged: result.report.privacy.raw_source_logged
                || traces.iter().any(|trace| trace.privacy.raw_source_logged),
            raw_prompt_logged: result.report.privacy.raw_prompt_logged
                || traces.iter().any(|trace| trace.privacy.raw_prompt_logged),
            raw_transcript_logged: result.report.privacy.raw_transcript_logged
                || traces
                    .iter()
                    .any(|trace| trace.privacy.raw_transcript_logged),
            raw_terminal_logged: result.report.privacy.raw_terminal_logged
                || traces.iter().any(|trace| trace.privacy.raw_terminal_logged),
        });
    }

    Ok(RunMatrixPrivacyReport {
        schema_version: RUN_MATRIX_PRIVACY_REPORT_SCHEMA_VERSION,
        suite_name: suite.name.clone(),
        task_count: suite.tasks.len(),
        run_count: runs.len(),
        trace_count,
        recorded_metadata: vec![
            "task ids".to_string(),
            "agent labels".to_string(),
            "variant labels".to_string(),
            "relative file paths and path hashes".to_string(),
            "command classes and command hashes".to_string(),
            "touched expected-test paths".to_string(),
            "exit statuses".to_string(),
            "task status".to_string(),
            "tool-call counts".to_string(),
            "token estimates".to_string(),
            "elapsed timing metadata".to_string(),
            "ctxhelm recommendation paths".to_string(),
            "ctxhelm pack token estimates".to_string(),
            "artifact byte counts and hashes".to_string(),
        ],
        forbidden_raw_data: vec![
            "raw source".to_string(),
            "raw prompts".to_string(),
            "raw model transcripts".to_string(),
            "raw terminal logs".to_string(),
            "raw adapter commands".to_string(),
            "raw setup commands".to_string(),
            "raw MCP payloads".to_string(),
            "raw ctxhelm pack sections or snippets".to_string(),
            "secrets".to_string(),
        ],
        safeguards: vec![
            "trace and report readers reject privacy flags that indicate raw data logging"
                .to_string(),
            "structured streams are parsed in memory and not persisted by capture-stream"
                .to_string(),
            "adapter and setup commands are stored as source-free hashes".to_string(),
            "matrix artifact paths are safe relative paths".to_string(),
            "published matrix artifacts are byte-counted and content-hashed".to_string(),
            "evidence bundles are verified before the matrix manifest is written".to_string(),
        ],
        runs,
        privacy: PrivacyStatus::source_free(),
    })
}

fn privacy_status_is_source_free(privacy: &PrivacyStatus) -> bool {
    privacy.source_free
        && !privacy.raw_source_logged
        && !privacy.raw_prompt_logged
        && !privacy.raw_transcript_logged
        && !privacy.raw_terminal_logged
}

fn render_markdown_run_matrix_privacy_report(report: &RunMatrixPrivacyReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# HelmBench Privacy Report: `{}`\n\n",
        report.suite_name
    ));
    out.push_str("This source-free report describes what HelmBench recorded for a run matrix and which raw data classes were intentionally excluded.\n\n");

    out.push_str("## Summary\n\n");
    out.push_str("| Field | Value |\n| --- | ---: |\n");
    out.push_str(&format!("| Tasks | {} |\n", report.task_count));
    out.push_str(&format!("| Runs | {} |\n", report.run_count));
    out.push_str(&format!("| Traces | {} |\n", report.trace_count));
    out.push_str(&format!(
        "| Source-free | {} |\n\n",
        yes_no(privacy_status_is_source_free(&report.privacy))
    ));

    out.push_str("## Run Checks\n\n");
    out.push_str("| Run | Variant | Report source-free | Source-free traces | Raw source | Raw prompts | Raw transcripts | Raw terminal logs |\n");
    out.push_str("| --- | --- | --- | ---: | --- | --- | --- | --- |\n");
    for run in &report.runs {
        out.push_str(&format!(
            "| `{}` | `{:?}` | {} | {}/{} | {} | {} | {} | {} |\n",
            run.name,
            run.variant,
            yes_no(run.report_source_free),
            run.source_free_trace_count,
            run.trace_count,
            yes_no(run.raw_source_logged),
            yes_no(run.raw_prompt_logged),
            yes_no(run.raw_transcript_logged),
            yes_no(run.raw_terminal_logged)
        ));
    }

    out.push_str("\n## Recorded Metadata\n\n");
    for item in &report.recorded_metadata {
        out.push_str(&format!("- {item}\n"));
    }

    out.push_str("\n## Forbidden Raw Data\n\n");
    for item in &report.forbidden_raw_data {
        out.push_str(&format!("- {item}\n"));
    }

    out.push_str("\n## Safeguards\n\n");
    for item in &report.safeguards {
        out.push_str(&format!("- {item}\n"));
    }

    out.push_str("\n## Privacy Flags\n\n");
    out.push_str("- Source-free: `true`\n");
    out.push_str("- Raw source logged: `false`\n");
    out.push_str("- Raw prompts logged: `false`\n");
    out.push_str("- Raw transcripts logged: `false`\n");
    out.push_str("- Raw terminal logs logged: `false`\n");
    out
}

#[allow(clippy::too_many_arguments)]
fn build_run_matrix_manifest(
    request: &RunMatrixRequest,
    baseline: &RunMatrixResult,
    heads: &[RunMatrixResult],
    suite_health_json: &Path,
    benchmark_summary_json: &Path,
    benchmark_summary_markdown: &Path,
    quality_gate_json: &Path,
    quality_gate_markdown: &Path,
    privacy_report_json: &Path,
    privacy_report_markdown: &Path,
    dashboard_html: &Path,
    autopsy_paths: &BTreeMap<String, PathBuf>,
    comparison_paths: &BTreeMap<String, (PathBuf, PathBuf)>,
    baseline_autopsy_markdown: &Path,
    reproduction_markdown: &Path,
    evidence_manifest: &Path,
    quality_gate_passed: bool,
    evidence_bundle_verified: bool,
) -> Result<RunMatrixManifest> {
    let provenance = run_matrix_provenance(request)?;
    let suite_health = read_public_suite_health(suite_health_json)
        .with_context(|| format!("read matrix suite health {}", suite_health_json.display()))?;
    validate_public_suite_health_report(&suite_health).with_context(|| {
        format!(
            "validate matrix suite health {}",
            suite_health_json.display()
        )
    })?;
    Ok(RunMatrixManifest {
        schema_version: RUN_MATRIX_MANIFEST_SCHEMA_VERSION,
        suite_path: request.suite_path.display().to_string(),
        repo_path: request.repo.display().to_string(),
        out_dir: request.out_dir.display().to_string(),
        provenance,
        suite_evidence_use: suite_health.evidence_use,
        baseline: run_matrix_manifest_run(
            &request.out_dir,
            baseline,
            autopsy_paths,
            comparison_paths,
        )?,
        heads: heads
            .iter()
            .map(|head| {
                run_matrix_manifest_run(&request.out_dir, head, autopsy_paths, comparison_paths)
            })
            .collect::<Result<Vec<_>>>()?,
        artifacts: RunMatrixManifestArtifacts {
            suite_health_json: manifest_path(&request.out_dir, suite_health_json),
            benchmark_summary_json: manifest_path(&request.out_dir, benchmark_summary_json),
            benchmark_summary_markdown: manifest_path(&request.out_dir, benchmark_summary_markdown),
            quality_gate_json: manifest_path(&request.out_dir, quality_gate_json),
            quality_gate_markdown: manifest_path(&request.out_dir, quality_gate_markdown),
            privacy_report_json: manifest_path(&request.out_dir, privacy_report_json),
            privacy_report_markdown: manifest_path(&request.out_dir, privacy_report_markdown),
            dashboard_html: manifest_path(&request.out_dir, dashboard_html),
            baseline_autopsy_markdown: manifest_path(&request.out_dir, baseline_autopsy_markdown),
            reproduction_markdown: manifest_path(&request.out_dir, reproduction_markdown),
            evidence_manifest: manifest_path(&request.out_dir, evidence_manifest),
        },
        artifact_digests: Vec::new(),
        quality_gate_passed,
        evidence_bundle_verified,
        privacy: PrivacyStatus::source_free(),
    })
}

fn run_matrix_provenance(request: &RunMatrixRequest) -> Result<RunMatrixProvenance> {
    let suite_raw = std::fs::read_to_string(&request.suite_path)
        .with_context(|| format!("read {}", request.suite_path.display()))?;
    let repo_head = git_output(&request.repo, &["rev-parse", "HEAD"]).ok();
    let repo_dirty = git_output(&request.repo, &["status", "--short"])
        .map(|status| !status.trim().is_empty())
        .unwrap_or(true);

    Ok(RunMatrixProvenance {
        helmbench_version: env!("CARGO_PKG_VERSION").to_string(),
        suite_hash: source_free_hash("suite", &suite_raw),
        repo_head,
        repo_dirty,
        setup_command_count: request.setup_commands.len(),
        setup_command_hashes: request
            .setup_commands
            .iter()
            .map(|command| command_hash(command))
            .collect(),
    })
}

fn run_matrix_manifest_run(
    out_dir: &Path,
    result: &RunMatrixResult,
    autopsy_paths: &BTreeMap<String, PathBuf>,
    comparison_paths: &BTreeMap<String, (PathBuf, PathBuf)>,
) -> Result<RunMatrixManifestRun> {
    let autopsy_path = autopsy_paths
        .get(&result.spec.safe_name)
        .with_context(|| format!("missing autopsy path for matrix run `{}`", result.spec.name))?;
    let comparison_paths = comparison_paths.get(&result.spec.safe_name);
    Ok(RunMatrixManifestRun {
        name: result.spec.name.clone(),
        agent: result.spec.agent.clone(),
        variant: result.spec.variant.clone(),
        report_path: manifest_path(out_dir, &result.report_path),
        trace_dir: manifest_path(out_dir, &result.trace_dir),
        autopsy_markdown: manifest_path(out_dir, autopsy_path),
        comparison_json: comparison_paths.map(|(json, _)| manifest_path(out_dir, json)),
        comparison_markdown: comparison_paths.map(|(_, markdown)| manifest_path(out_dir, markdown)),
        ctxhelm_enabled: result.spec.ctxhelm.is_some(),
        pack_enabled: result
            .spec
            .ctxhelm
            .as_ref()
            .is_some_and(|ctxhelm| ctxhelm.include_pack),
        stream_capture_enabled: result.spec.capture_stream,
        adapter_preset: result
            .spec
            .adapter_preset
            .as_ref()
            .map(|config| config.preset),
        adapter_command_hash: result
            .spec
            .adapter_command
            .as_ref()
            .map(|command| command_hash(command)),
        ctxhelm_config_hash: result.spec.ctxhelm.as_ref().map(ctxhelm_config_hash),
    })
}

fn manifest_path(out_dir: &Path, path: &Path) -> String {
    path.strip_prefix(out_dir)
        .unwrap_or(path)
        .display()
        .to_string()
}

fn collect_matrix_artifact_digests(
    matrix_dir: &Path,
    manifest: &RunMatrixManifest,
) -> Result<Vec<MatrixArtifactDigest>> {
    let mut paths = BTreeSet::new();
    insert_matrix_artifact_paths(manifest, &mut paths);
    for run in std::iter::once(&manifest.baseline).chain(manifest.heads.iter()) {
        collect_matrix_trace_file_paths(matrix_dir, &run.trace_dir, &mut paths)?;
    }

    paths
        .into_iter()
        .map(|path| matrix_artifact_digest(matrix_dir, &path))
        .collect()
}

fn insert_matrix_artifact_paths(manifest: &RunMatrixManifest, paths: &mut BTreeSet<String>) {
    for run in std::iter::once(&manifest.baseline).chain(manifest.heads.iter()) {
        paths.insert(run.report_path.clone());
        paths.insert(run.autopsy_markdown.clone());
        if let Some(path) = &run.comparison_json {
            paths.insert(path.clone());
        }
        if let Some(path) = &run.comparison_markdown {
            paths.insert(path.clone());
        }
    }
    paths.insert(manifest.artifacts.suite_health_json.clone());
    paths.insert(manifest.artifacts.benchmark_summary_json.clone());
    paths.insert(manifest.artifacts.benchmark_summary_markdown.clone());
    paths.insert(manifest.artifacts.quality_gate_json.clone());
    paths.insert(manifest.artifacts.quality_gate_markdown.clone());
    paths.insert(manifest.artifacts.privacy_report_json.clone());
    paths.insert(manifest.artifacts.privacy_report_markdown.clone());
    paths.insert(manifest.artifacts.dashboard_html.clone());
    paths.insert(manifest.artifacts.baseline_autopsy_markdown.clone());
    paths.insert(manifest.artifacts.reproduction_markdown.clone());
    paths.insert(manifest.artifacts.evidence_manifest.clone());
}

fn collect_matrix_trace_file_paths(
    matrix_dir: &Path,
    trace_dir: &str,
    paths: &mut BTreeSet<String>,
) -> Result<()> {
    let trace_dir = require_matrix_dir(matrix_dir, trace_dir)?;
    let mut stack = vec![trace_dir];
    while let Some(dir) = stack.pop() {
        for entry in std::fs::read_dir(&dir).with_context(|| format!("read {}", dir.display()))? {
            let entry = entry.with_context(|| format!("read entry in {}", dir.display()))?;
            let path = entry.path();
            let file_type = entry
                .file_type()
                .with_context(|| format!("inspect {}", path.display()))?;
            if file_type.is_dir() {
                stack.push(path);
            } else if file_type.is_file() {
                let relative = path
                    .strip_prefix(matrix_dir)
                    .with_context(|| format!("resolve matrix artifact {}", path.display()))?
                    .display()
                    .to_string();
                helmbench::validate_safe_relative_path_for_cli(&relative)
                    .with_context(|| format!("validate matrix trace path `{relative}`"))?;
                paths.insert(relative);
            }
        }
    }
    Ok(())
}

fn matrix_artifact_digest(matrix_dir: &Path, relative_path: &str) -> Result<MatrixArtifactDigest> {
    let path = require_matrix_file(matrix_dir, relative_path)?;
    let bytes = std::fs::read(&path).with_context(|| format!("read {}", path.display()))?;
    Ok(MatrixArtifactDigest {
        path: relative_path.to_string(),
        byte_count: bytes.len() as u64,
        content_hash: content_hash(&bytes),
    })
}

fn render_markdown_matrix_reproduction(manifest: &RunMatrixManifest) -> String {
    let mut out = String::new();
    out.push_str("# HelmBench Reproduction\n\n");
    out.push_str("This source-free guide describes how to verify and rerun the matrix evidence without storing raw source, prompts, transcripts, terminal logs, adapter commands, setup commands, or ctxhelm pack sections.\n\n");

    out.push_str("## Verify Published Artifacts\n\n");
    out.push_str("```bash\n");
    out.push_str("helmbench verify-matrix --matrix <matrix-dir>\n");
    out.push_str("helmbench verify-bundle --bundle <matrix-dir>/evidence\n");
    out.push_str("```\n\n");

    out.push_str("## Run Identity\n\n");
    out.push_str("| Field | Value |\n| --- | --- |\n");
    out.push_str(&format!(
        "| HelmBench version | `{}` |\n",
        manifest.provenance.helmbench_version
    ));
    out.push_str(&format!(
        "| Suite hash | `{}` |\n",
        manifest.provenance.suite_hash
    ));
    out.push_str(&format!(
        "| Repo HEAD | `{}` |\n",
        manifest
            .provenance
            .repo_head
            .as_deref()
            .unwrap_or("unknown")
    ));
    out.push_str(&format!(
        "| Dirty checkout | {} |\n",
        yes_no(manifest.provenance.repo_dirty)
    ));
    out.push_str(&format!(
        "| Setup commands | {} hashed command(s) |\n\n",
        manifest.provenance.setup_command_count
    ));

    if !manifest.provenance.setup_command_hashes.is_empty() {
        out.push_str("## Setup Command Hashes\n\n");
        for hash in &manifest.provenance.setup_command_hashes {
            out.push_str(&format!("- `{hash}`\n"));
        }
        out.push('\n');
    }

    out.push_str("## Runs\n\n");
    out.push_str("| Run | Agent | Variant | Preset | ctxhelm | Pack | Stream | Report | Trace Dir | Autopsy | Comparison JSON | Comparison Markdown | Adapter Hash | ctxhelm Hash |\n");
    out.push_str(
        "| --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- | --- |\n",
    );
    for run in std::iter::once(&manifest.baseline).chain(manifest.heads.iter()) {
        out.push_str(&format!(
            "| `{}` | `{}` | `{:?}` | `{}` | {} | {} | {} | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` | `{}` |\n",
            run.name,
            run.agent,
            run.variant,
            adapter_preset_label(run.adapter_preset).unwrap_or("none"),
            yes_no(run.ctxhelm_enabled),
            yes_no(run.pack_enabled),
            yes_no(run.stream_capture_enabled),
            run.report_path,
            run.trace_dir,
            run.autopsy_markdown,
            run.comparison_json.as_deref().unwrap_or("none"),
            run.comparison_markdown.as_deref().unwrap_or("none"),
            run.adapter_command_hash.as_deref().unwrap_or("none"),
            run.ctxhelm_config_hash.as_deref().unwrap_or("none")
        ));
    }

    out.push_str("\n## Key Artifacts\n\n");
    out.push_str("| Artifact | Path |\n| --- | --- |\n");
    out.push_str(&format!(
        "| Suite health | `{}` |\n",
        manifest.artifacts.suite_health_json
    ));
    out.push_str(&format!(
        "| Benchmark summary JSON | `{}` |\n",
        manifest.artifacts.benchmark_summary_json
    ));
    out.push_str(&format!(
        "| Benchmark summary Markdown | `{}` |\n",
        manifest.artifacts.benchmark_summary_markdown
    ));
    out.push_str(&format!(
        "| Quality gate JSON | `{}` |\n",
        manifest.artifacts.quality_gate_json
    ));
    out.push_str(&format!(
        "| Quality gate Markdown | `{}` |\n",
        manifest.artifacts.quality_gate_markdown
    ));
    out.push_str(&format!(
        "| Privacy report JSON | `{}` |\n",
        manifest.artifacts.privacy_report_json
    ));
    out.push_str(&format!(
        "| Privacy report Markdown | `{}` |\n",
        manifest.artifacts.privacy_report_markdown
    ));
    out.push_str(&format!(
        "| Dashboard | `{}` |\n",
        manifest.artifacts.dashboard_html
    ));
    out.push_str(&format!(
        "| Baseline autopsy | `{}` |\n",
        manifest.artifacts.baseline_autopsy_markdown
    ));
    out.push_str(&format!(
        "| Evidence manifest | `{}` |\n\n",
        manifest.artifacts.evidence_manifest
    ));

    out.push_str("## Rerun Notes\n\n");
    out.push_str("- Check out the target repository at the recorded repo HEAD before rerunning.\n");
    out.push_str("- Use a suite with the recorded suite hash.\n");
    out.push_str("- Recreate adapter/setup commands from local configuration; HelmBench stores only hashes for privacy.\n");
    out.push_str("- Compare a new run with the published matrix using `helmbench matrix-history --matrix <old-matrix-dir> --matrix <new-matrix-dir> --out <history.md>`.\n\n");

    out.push_str("## Privacy\n\n");
    out.push_str("- Source-free: `true`\n");
    out.push_str("- Raw source logged: `false`\n");
    out.push_str("- Raw prompts logged: `false`\n");
    out.push_str("- Raw transcripts logged: `false`\n");
    out.push_str("- Raw terminal logs logged: `false`\n");
    out
}

fn verify_run_matrix(matrix_dir: &Path) -> Result<RunMatrixManifest> {
    let manifest_path = matrix_dir.join("matrix-manifest.json");
    let raw = std::fs::read_to_string(&manifest_path)
        .with_context(|| format!("read {}", manifest_path.display()))?;
    let manifest = serde_json::from_str::<RunMatrixManifest>(&raw)
        .with_context(|| format!("parse {}", manifest_path.display()))?;

    if manifest.schema_version != RUN_MATRIX_MANIFEST_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported matrix manifest schemaVersion {}; expected {}",
            manifest.schema_version,
            RUN_MATRIX_MANIFEST_SCHEMA_VERSION
        );
    }
    if manifest.suite_path.trim().is_empty() {
        anyhow::bail!("matrix manifest suitePath must not be empty");
    }
    if manifest.repo_path.trim().is_empty() {
        anyhow::bail!("matrix manifest repoPath must not be empty");
    }
    if manifest.baseline.name.trim().is_empty() {
        anyhow::bail!("matrix manifest baseline name must not be empty");
    }
    if manifest.heads.is_empty() {
        anyhow::bail!("matrix manifest must contain at least one head run");
    }
    if !manifest.evidence_bundle_verified {
        anyhow::bail!("matrix manifest evidenceBundleVerified must be true");
    }
    verify_matrix_provenance(&manifest.provenance)?;
    if !manifest.privacy.source_free
        || manifest.privacy.raw_source_logged
        || manifest.privacy.raw_prompt_logged
        || manifest.privacy.raw_transcript_logged
        || manifest.privacy.raw_terminal_logged
    {
        anyhow::bail!("matrix manifest is not source-free");
    }

    verify_matrix_run(matrix_dir, &manifest.baseline)?;
    let mut names = BTreeSet::new();
    names.insert(manifest.baseline.name.clone());
    for head in &manifest.heads {
        if !names.insert(head.name.clone()) {
            anyhow::bail!("duplicate matrix run name `{}`", head.name);
        }
        verify_matrix_run(matrix_dir, head)?;
    }

    let artifact_paths = [
        &manifest.artifacts.suite_health_json,
        &manifest.artifacts.benchmark_summary_json,
        &manifest.artifacts.benchmark_summary_markdown,
        &manifest.artifacts.quality_gate_json,
        &manifest.artifacts.quality_gate_markdown,
        &manifest.artifacts.privacy_report_json,
        &manifest.artifacts.privacy_report_markdown,
        &manifest.artifacts.dashboard_html,
        &manifest.artifacts.baseline_autopsy_markdown,
        &manifest.artifacts.reproduction_markdown,
        &manifest.artifacts.evidence_manifest,
    ];
    for path in artifact_paths {
        require_matrix_file(matrix_dir, path)?;
    }
    let suite_health_path = matrix_path(matrix_dir, &manifest.artifacts.suite_health_json)?;
    let suite_health = read_public_suite_health(&suite_health_path)
        .with_context(|| format!("read suite health {}", suite_health_path.display()))?;
    validate_public_suite_health_report(&suite_health)
        .with_context(|| format!("validate suite health {}", suite_health_path.display()))?;
    if suite_health.evidence_use != manifest.suite_evidence_use {
        anyhow::bail!(
            "matrix manifest suiteEvidenceUse `{}` does not match suite-health evidenceUse `{}`",
            manifest.suite_evidence_use,
            suite_health.evidence_use
        );
    }
    let privacy_report_path = matrix_path(matrix_dir, &manifest.artifacts.privacy_report_json)?;
    validate_run_matrix_privacy_report(&privacy_report_path)
        .with_context(|| format!("validate privacy report {}", privacy_report_path.display()))?;
    verify_matrix_benchmark_summary(matrix_dir, &manifest)?;

    let evidence_manifest = matrix_path(matrix_dir, &manifest.artifacts.evidence_manifest)?;
    let evidence_dir = evidence_manifest
        .parent()
        .with_context(|| format!("resolve evidence dir {}", evidence_manifest.display()))?;
    verify_evidence_bundle(evidence_dir)?;
    verify_matrix_artifact_digests(matrix_dir, &manifest)?;

    Ok(manifest)
}

fn verify_matrix_benchmark_summary(matrix_dir: &Path, manifest: &RunMatrixManifest) -> Result<()> {
    let summary_path = matrix_path(matrix_dir, &manifest.artifacts.benchmark_summary_json)?;
    let summary = read_benchmark_summary(&summary_path)
        .with_context(|| format!("read matrix summary {}", summary_path.display()))?;
    let expected_run_count = manifest.heads.len() + 1;
    if summary.runs.len() != expected_run_count {
        anyhow::bail!(
            "matrix benchmark summary has {} run(s) but manifest expects {}",
            summary.runs.len(),
            expected_run_count
        );
    }
    if summary.comparisons.len() != manifest.heads.len() {
        anyhow::bail!(
            "matrix benchmark summary has {} comparison(s) but manifest expects {}",
            summary.comparisons.len(),
            manifest.heads.len()
        );
    }
    if summary.baseline != summary.runs[0] {
        anyhow::bail!("matrix benchmark summary baseline does not match first run");
    }

    verify_matrix_run_summary_identity("baseline", &manifest.baseline, &summary.baseline)?;
    verify_matrix_run_summary_identity("run `0`", &manifest.baseline, &summary.runs[0])?;
    for (index, (manifest_run, summary_run)) in manifest
        .heads
        .iter()
        .zip(summary.runs.iter().skip(1))
        .enumerate()
    {
        verify_matrix_run_summary_identity(
            &format!("run `{}`", index + 1),
            manifest_run,
            summary_run,
        )?;
    }
    for (index, (manifest_run, comparison)) in manifest
        .heads
        .iter()
        .zip(summary.comparisons.iter())
        .enumerate()
    {
        if manifest_run.agent != comparison.head_agent
            || manifest_run.variant != comparison.head_variant
        {
            anyhow::bail!(
                "matrix comparison `{}` is {} / {:?}, manifest head `{}` is {} / {:?}",
                index,
                comparison.head_agent,
                comparison.head_variant,
                manifest_run.name,
                manifest_run.agent,
                manifest_run.variant
            );
        }
    }
    Ok(())
}

fn validate_run_matrix_privacy_report(path: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let report = serde_json::from_str::<RunMatrixPrivacyReport>(&raw)
        .with_context(|| format!("parse privacy report {}", path.display()))?;
    if report.schema_version != RUN_MATRIX_PRIVACY_REPORT_SCHEMA_VERSION {
        anyhow::bail!(
            "unsupported privacy report schemaVersion {}; expected {}",
            report.schema_version,
            RUN_MATRIX_PRIVACY_REPORT_SCHEMA_VERSION
        );
    }
    if report.suite_name.trim().is_empty() {
        anyhow::bail!("privacy report suiteName must not be empty");
    }
    if report.run_count != report.runs.len() {
        anyhow::bail!("privacy report runCount does not match runs");
    }
    if !privacy_status_is_source_free(&report.privacy) {
        anyhow::bail!("privacy report is not source-free");
    }
    if report.recorded_metadata.is_empty()
        || report.forbidden_raw_data.is_empty()
        || report.safeguards.is_empty()
    {
        anyhow::bail!("privacy report must describe metadata, forbidden data, and safeguards");
    }
    let mut trace_count = 0usize;
    for run in &report.runs {
        if run.name.trim().is_empty() || run.agent.trim().is_empty() {
            anyhow::bail!("privacy report run identity must not be empty");
        }
        helmbench::validate_safe_relative_path_for_cli(&run.report_path)
            .with_context(|| format!("validate privacy report path `{}`", run.report_path))?;
        helmbench::validate_safe_relative_path_for_cli(&run.trace_dir)
            .with_context(|| format!("validate privacy trace dir `{}`", run.trace_dir))?;
        if !run.report_source_free || run.source_free_trace_count != run.trace_count {
            anyhow::bail!("privacy report run `{}` is not fully source-free", run.name);
        }
        if run.raw_source_logged
            || run.raw_prompt_logged
            || run.raw_transcript_logged
            || run.raw_terminal_logged
        {
            anyhow::bail!("privacy report run `{}` logged raw data", run.name);
        }
        trace_count += run.trace_count;
    }
    if report.trace_count != trace_count {
        anyhow::bail!("privacy report traceCount does not match runs");
    }
    Ok(())
}

fn verify_matrix_artifact_digests(matrix_dir: &Path, manifest: &RunMatrixManifest) -> Result<()> {
    if manifest.artifact_digests.is_empty() {
        anyhow::bail!("matrix manifest must contain artifactDigests");
    }
    let mut seen_paths = BTreeSet::new();
    for digest in &manifest.artifact_digests {
        helmbench::validate_safe_relative_path_for_cli(&digest.path)
            .with_context(|| format!("validate matrix artifact digest path `{}`", digest.path))?;
        if !seen_paths.insert(digest.path.clone()) {
            anyhow::bail!("duplicate matrix artifact digest path `{}`", digest.path);
        }
        if !digest.content_hash.starts_with("fnv64:") {
            anyhow::bail!(
                "matrix artifact `{}` has unsupported contentHash `{}`",
                digest.path,
                digest.content_hash
            );
        }
    }

    let actual = collect_matrix_artifact_digests(matrix_dir, manifest)?;
    if actual != manifest.artifact_digests {
        anyhow::bail!("matrix artifact digest mismatch");
    }
    Ok(())
}

fn verify_matrix_provenance(provenance: &RunMatrixProvenance) -> Result<()> {
    if provenance.helmbench_version.trim().is_empty() {
        anyhow::bail!("matrix manifest helmbenchVersion must not be empty");
    }
    if !provenance.suite_hash.starts_with("suite:") {
        anyhow::bail!("matrix manifest suiteHash must be a source-free suite hash");
    }
    if provenance.setup_command_count != provenance.setup_command_hashes.len() {
        anyhow::bail!("matrix manifest setup command count does not match hashes");
    }
    for hash in &provenance.setup_command_hashes {
        if !hash.starts_with("cmd:") {
            anyhow::bail!("matrix manifest setup command hash must be source-free");
        }
    }
    Ok(())
}

fn build_matrix_history_report(matrix_dirs: &[PathBuf]) -> Result<MatrixHistoryReport> {
    if matrix_dirs.len() < 2 {
        anyhow::bail!("matrix-history requires at least two --matrix directories");
    }

    let mut entries = Vec::with_capacity(matrix_dirs.len());
    let mut suite_name = None::<String>;
    let mut expected_run_names = None::<BTreeSet<String>>;

    for (index, matrix_dir) in matrix_dirs.iter().enumerate() {
        let manifest = verify_run_matrix(matrix_dir)
            .with_context(|| format!("verify matrix {}", matrix_dir.display()))?;
        let summary_path = matrix_path(matrix_dir, &manifest.artifacts.benchmark_summary_json)?;
        let summary = read_benchmark_summary(&summary_path)
            .with_context(|| format!("read matrix summary {}", matrix_dir.display()))?;

        match &suite_name {
            Some(expected) if expected != &summary.suite_name => anyhow::bail!(
                "matrix `{}` suite `{}` does not match first suite `{}`",
                index + 1,
                summary.suite_name,
                expected
            ),
            None => suite_name = Some(summary.suite_name.clone()),
            _ => {}
        }

        let entry = matrix_history_entry(index + 1, matrix_dir, &manifest, &summary)?;
        let run_names = entry
            .runs
            .iter()
            .map(|run| run.name.clone())
            .collect::<BTreeSet<_>>();
        match &expected_run_names {
            Some(expected) if expected != &run_names => {
                anyhow::bail!("matrix `{}` run names do not match first matrix", index + 1)
            }
            None => expected_run_names = Some(run_names),
            _ => {}
        }
        entries.push(entry);
    }

    let trends = matrix_history_trends(&entries)?;
    Ok(MatrixHistoryReport {
        schema_version: MATRIX_HISTORY_SCHEMA_VERSION,
        suite_name: suite_name.unwrap_or_default(),
        matrices: entries,
        trends,
        privacy: PrivacyStatus::source_free(),
    })
}

fn matrix_history_entry(
    matrix_index: usize,
    matrix_dir: &Path,
    manifest: &RunMatrixManifest,
    summary: &BenchmarkSummaryReport,
) -> Result<MatrixHistoryEntry> {
    let manifest_runs = std::iter::once(&manifest.baseline)
        .chain(manifest.heads.iter())
        .collect::<Vec<_>>();
    if manifest_runs.len() != summary.runs.len() {
        anyhow::bail!(
            "matrix `{}` has {} manifest run(s) but {} summary run(s)",
            matrix_index,
            manifest_runs.len(),
            summary.runs.len()
        );
    }

    let runs = manifest_runs
        .iter()
        .zip(summary.runs.iter())
        .map(|(manifest_run, summary_run)| {
            matrix_history_run(manifest_run, summary_run).with_context(|| {
                format!(
                    "match matrix `{}` run `{}`",
                    matrix_index, manifest_run.name
                )
            })
        })
        .collect::<Result<Vec<_>>>()?;

    Ok(MatrixHistoryEntry {
        matrix_index,
        label: source_free_matrix_label(matrix_index, matrix_dir),
        quality_gate_passed: manifest.quality_gate_passed,
        evidence_bundle_verified: manifest.evidence_bundle_verified,
        runs,
    })
}

fn matrix_history_run(
    manifest_run: &RunMatrixManifestRun,
    summary_run: &BenchmarkRunSummary,
) -> Result<MatrixHistoryRun> {
    verify_matrix_run_summary_identity("history run", manifest_run, summary_run)?;
    Ok(MatrixHistoryRun {
        name: manifest_run.name.clone(),
        agent: summary_run.agent.clone(),
        variant: summary_run.variant.clone(),
        task_count: summary_run.task_count,
        success_rate: summary_run.success_rate,
        validation_coverage_rate: summary_run.validation_coverage_rate,
        irrelevant_read_rate: summary_run.irrelevant_read_rate,
        recommendation_recall: summary_run.recommendation_recall,
        context_precision: summary_run.context_precision,
        edited_file_recall: summary_run.edited_file_recall,
        average_time_to_first_relevant_file_millis: summary_run
            .average_time_to_first_relevant_file_millis,
        total_tool_calls: summary_run.total_tool_calls,
        total_token_estimate: summary_run.total_token_estimate,
        tool_calls_per_success: summary_run.tool_calls_per_success,
        token_estimate_per_success: summary_run.token_estimate_per_success,
    })
}

fn verify_matrix_run_summary_identity(
    label: &str,
    manifest_run: &RunMatrixManifestRun,
    summary_run: &BenchmarkRunSummary,
) -> Result<()> {
    if manifest_run.agent != summary_run.agent || manifest_run.variant != summary_run.variant {
        anyhow::bail!(
            "matrix {} `{}` is {} / {:?}, summary is {} / {:?}",
            label,
            manifest_run.name,
            manifest_run.agent,
            manifest_run.variant,
            summary_run.agent,
            summary_run.variant
        );
    }
    Ok(())
}

fn matrix_history_trends(entries: &[MatrixHistoryEntry]) -> Result<Vec<MatrixRunTrend>> {
    let first = entries
        .first()
        .context("matrix history requires a first entry")?;
    let last = entries
        .last()
        .context("matrix history requires a last entry")?;
    let last_by_name = last
        .runs
        .iter()
        .map(|run| (run.name.as_str(), run))
        .collect::<BTreeMap<_, _>>();

    first
        .runs
        .iter()
        .map(|first_run| {
            let last_run = last_by_name
                .get(first_run.name.as_str())
                .with_context(|| format!("last matrix missing run `{}`", first_run.name))?;
            Ok(MatrixRunTrend {
                name: first_run.name.clone(),
                agent: first_run.agent.clone(),
                variant: first_run.variant.clone(),
                first_success_rate: first_run.success_rate,
                last_success_rate: last_run.success_rate,
                success_rate_delta: last_run.success_rate - first_run.success_rate,
                first_validation_coverage_rate: first_run.validation_coverage_rate,
                last_validation_coverage_rate: last_run.validation_coverage_rate,
                validation_coverage_rate_delta: last_run.validation_coverage_rate
                    - first_run.validation_coverage_rate,
                first_irrelevant_read_rate: first_run.irrelevant_read_rate,
                last_irrelevant_read_rate: last_run.irrelevant_read_rate,
                irrelevant_read_rate_delta: last_run.irrelevant_read_rate
                    - first_run.irrelevant_read_rate,
                first_recommendation_recall: first_run.recommendation_recall,
                last_recommendation_recall: last_run.recommendation_recall,
                recommendation_recall_delta: last_run.recommendation_recall
                    - first_run.recommendation_recall,
                first_context_precision: first_run.context_precision,
                last_context_precision: last_run.context_precision,
                context_precision_delta: last_run.context_precision - first_run.context_precision,
                first_edited_file_recall: first_run.edited_file_recall,
                last_edited_file_recall: last_run.edited_file_recall,
                edited_file_recall_delta: last_run.edited_file_recall
                    - first_run.edited_file_recall,
                first_average_time_to_first_relevant_file_millis: first_run
                    .average_time_to_first_relevant_file_millis,
                last_average_time_to_first_relevant_file_millis: last_run
                    .average_time_to_first_relevant_file_millis,
                average_time_to_first_relevant_file_millis_delta: optional_delta(
                    first_run.average_time_to_first_relevant_file_millis,
                    last_run.average_time_to_first_relevant_file_millis,
                ),
                total_tool_calls_delta: last_run.total_tool_calls as i64
                    - first_run.total_tool_calls as i64,
                total_token_estimate_delta: last_run.total_token_estimate as i64
                    - first_run.total_token_estimate as i64,
                first_tool_calls_per_success: first_run.tool_calls_per_success,
                last_tool_calls_per_success: last_run.tool_calls_per_success,
                tool_calls_per_success_delta: optional_delta(
                    first_run.tool_calls_per_success,
                    last_run.tool_calls_per_success,
                ),
                first_token_estimate_per_success: first_run.token_estimate_per_success,
                last_token_estimate_per_success: last_run.token_estimate_per_success,
                token_estimate_per_success_delta: optional_delta(
                    first_run.token_estimate_per_success,
                    last_run.token_estimate_per_success,
                ),
            })
        })
        .collect()
}

fn optional_delta(first: Option<f32>, last: Option<f32>) -> Option<f32> {
    Some(last? - first?)
}

fn source_free_matrix_label(matrix_index: usize, matrix_dir: &Path) -> String {
    let basename = matrix_dir
        .file_name()
        .and_then(|name| name.to_str())
        .map(safe_task_dir_name)
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| "matrix".to_string());
    format!("{}-{}", matrix_index, basename)
}

fn render_markdown_matrix_history(report: &MatrixHistoryReport) -> String {
    let mut out = String::new();
    out.push_str(&format!(
        "# HelmBench Matrix History: `{}`\n\n",
        report.suite_name
    ));
    out.push_str("## Matrices\n\n");
    out.push_str("| Matrix | Label | Quality gate | Evidence verified |\n");
    out.push_str("| ---: | --- | --- | --- |\n");
    for entry in &report.matrices {
        out.push_str(&format!(
            "| {} | `{}` | {} | {} |\n",
            entry.matrix_index,
            entry.label,
            yes_no(entry.quality_gate_passed),
            yes_no(entry.evidence_bundle_verified)
        ));
    }

    out.push_str("\n## First-To-Last Trends\n\n");
    out.push_str("| Run | Variant | Success | Validation | Rec recall | Context precision | Edited recall | Irrelevant reads | First relevant | Tools | Tokens | Tools/success | Tokens/success |\n");
    out.push_str("| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n");
    for trend in &report.trends {
        out.push_str(&format!(
            "| `{}` | {} / {:?} | {:+.1}% | {:+.1}% | {:+.1}% | {:+.1}% | {:+.1}% | {:+.1}% | {} | {:+} | {:+} | {} | {} |\n",
            trend.name,
            trend.agent,
            trend.variant,
            matrix_pct(trend.success_rate_delta),
            matrix_pct(trend.validation_coverage_rate_delta),
            matrix_pct(trend.recommendation_recall_delta),
            matrix_pct(trend.context_precision_delta),
            matrix_pct(trend.edited_file_recall_delta),
            matrix_pct(trend.irrelevant_read_rate_delta),
            markdown_optional_millis_delta(trend.average_time_to_first_relevant_file_millis_delta),
            trend.total_tool_calls_delta,
            trend.total_token_estimate_delta,
            markdown_optional_number_delta(trend.tool_calls_per_success_delta),
            markdown_optional_number_delta(trend.token_estimate_per_success_delta)
        ));
    }

    out.push_str("\n## Privacy\n\n");
    out.push_str("- Source-free: `true`\n");
    out.push_str("- Raw source logged: `false`\n");
    out.push_str("- Raw prompts logged: `false`\n");
    out.push_str("- Raw transcripts logged: `false`\n");
    out.push_str("- Raw terminal logs logged: `false`\n");
    out
}

fn render_html_matrix_history(report: &MatrixHistoryReport) -> String {
    let mut out = String::new();
    out.push_str("<!doctype html>\n<html lang=\"en\">\n<head>\n");
    out.push_str("<meta charset=\"utf-8\">\n");
    out.push_str("<meta name=\"viewport\" content=\"width=device-width, initial-scale=1\">\n");
    out.push_str("<title>HelmBench Matrix History</title>\n");
    out.push_str("<style>\n");
    out.push_str(MATRIX_HISTORY_CSS);
    out.push_str("\n</style>\n</head>\n<body>\n");
    out.push_str("<main class=\"shell\">\n");
    out.push_str("<header class=\"hero\">\n");
    out.push_str("<div><p class=\"eyebrow\">Source-free longitudinal evaluation</p>\n");
    out.push_str(&format!(
        "<h1>Matrix History</h1>\n<p class=\"lede\">Suite <strong>{}</strong> across {} verified matrix runs.</p></div>\n",
        matrix_html_escape(&report.suite_name),
        report.matrices.len()
    ));
    out.push_str("<div class=\"privacy-badge\">Source-free</div>\n</header>\n");

    out.push_str("<section class=\"summary-grid\" aria-label=\"Trend summary\">\n");
    for trend in &report.trends {
        out.push_str("<article class=\"run-card\">\n");
        out.push_str(&format!(
            "<div class=\"run-title\"><span>{}</span><code>{:?}</code></div>\n",
            matrix_html_escape(&trend.name),
            trend.variant
        ));
        out.push_str(&format!(
            "<p class=\"suite\">Agent: <strong>{}</strong></p>\n",
            matrix_html_escape(&trend.agent)
        ));
        out.push_str("<div class=\"metric-row\">\n");
        out.push_str(&history_metric_tile(
            "Success",
            matrix_pct(trend.last_success_rate),
            history_delta(trend.success_rate_delta, true),
        ));
        out.push_str(&history_metric_tile(
            "Validation",
            matrix_pct(trend.last_validation_coverage_rate),
            history_delta(trend.validation_coverage_rate_delta, true),
        ));
        out.push_str(&history_metric_tile(
            "Context precision",
            matrix_pct(trend.last_context_precision),
            history_delta(trend.context_precision_delta, true),
        ));
        out.push_str(&history_metric_tile(
            "Irrelevant reads",
            matrix_pct(trend.last_irrelevant_read_rate),
            history_delta(trend.irrelevant_read_rate_delta, false),
        ));
        out.push_str(&history_metric_tile_text(
            "First relevant",
            html_optional_millis(trend.last_average_time_to_first_relevant_file_millis),
            html_optional_millis_delta(trend.average_time_to_first_relevant_file_millis_delta),
        ));
        out.push_str(&history_metric_tile_text(
            "Tools/success",
            html_optional_number(trend.last_tool_calls_per_success),
            html_optional_number_delta(trend.tool_calls_per_success_delta, false),
        ));
        out.push_str(&history_metric_tile_text(
            "Tokens/success",
            html_optional_number(trend.last_token_estimate_per_success),
            html_optional_number_delta(trend.token_estimate_per_success_delta, false),
        ));
        out.push_str("</div>\n</article>\n");
    }
    out.push_str("</section>\n");

    out.push_str("<section class=\"panel\">\n");
    out.push_str("<h2>First-To-Last Trends</h2>\n");
    out.push_str("<div class=\"table-wrap\"><table>\n");
    out.push_str("<thead><tr><th>Run</th><th>Variant</th><th>Success</th><th>Validation</th><th>Recommendation recall</th><th>Context precision</th><th>Edited recall</th><th>Irrelevant reads</th><th>First relevant</th><th>Tools</th><th>Tokens</th><th>Tools/success</th><th>Tokens/success</th></tr></thead>\n<tbody>\n");
    for trend in &report.trends {
        out.push_str(&format!(
            "<tr><td><strong>{}</strong><br>{}</td><td><code>{:?}</code></td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{}</td><td>{:+}</td><td>{:+}</td><td>{}</td><td>{}</td></tr>\n",
            matrix_html_escape(&trend.name),
            matrix_html_escape(&trend.agent),
            trend.variant,
            history_delta(trend.success_rate_delta, true),
            history_delta(trend.validation_coverage_rate_delta, true),
            history_delta(trend.recommendation_recall_delta, true),
            history_delta(trend.context_precision_delta, true),
            history_delta(trend.edited_file_recall_delta, true),
            history_delta(trend.irrelevant_read_rate_delta, false),
            html_optional_millis_delta(trend.average_time_to_first_relevant_file_millis_delta),
            trend.total_tool_calls_delta,
            trend.total_token_estimate_delta,
            html_optional_number_delta(trend.tool_calls_per_success_delta, false),
            html_optional_number_delta(trend.token_estimate_per_success_delta, false)
        ));
    }
    out.push_str("</tbody></table></div>\n</section>\n");

    out.push_str("<section class=\"panel\">\n");
    out.push_str("<h2>Verified Matrices</h2>\n");
    out.push_str("<div class=\"table-wrap\"><table>\n");
    out.push_str("<thead><tr><th>Matrix</th><th>Quality gate</th><th>Evidence</th><th>Runs</th></tr></thead>\n<tbody>\n");
    for entry in &report.matrices {
        let runs = entry
            .runs
            .iter()
            .map(|run| {
                format!(
                    "{} ({:?}): {:.1}% success, {:.1}% validation, {} first relevant",
                    matrix_html_escape(&run.name),
                    run.variant,
                    matrix_pct(run.success_rate),
                    matrix_pct(run.validation_coverage_rate),
                    matrix_html_escape(&markdown_optional_millis(
                        run.average_time_to_first_relevant_file_millis
                    ))
                )
            })
            .collect::<Vec<_>>()
            .join("<br>");
        out.push_str(&format!(
            "<tr><td><strong>{}</strong><br><code>{}</code></td><td>{}</td><td>{}</td><td>{}</td></tr>\n",
            entry.matrix_index,
            matrix_html_escape(&entry.label),
            yes_no(entry.quality_gate_passed),
            yes_no(entry.evidence_bundle_verified),
            runs
        ));
    }
    out.push_str("</tbody></table></div>\n</section>\n");

    out.push_str("<section class=\"privacy\">\n");
    out.push_str("<h2>Privacy Contract</h2>\n");
    out.push_str("<p>This dashboard is generated from verified source-free matrix summaries. It does not include raw source, prompts, transcripts, terminal logs, MCP payloads, or absolute local matrix paths.</p>\n");
    out.push_str("</section>\n</main>\n</body>\n</html>\n");
    out
}

fn history_metric_tile(label: &str, value: f32, delta: String) -> String {
    format!(
        "<div class=\"metric\"><span>{}</span><strong>{:.1}%</strong><em>{}</em></div>\n",
        matrix_html_escape(label),
        value,
        delta
    )
}

fn history_metric_tile_text(label: &str, value: String, delta: String) -> String {
    format!(
        "<div class=\"metric\"><span>{}</span><strong>{}</strong><em>{}</em></div>\n",
        matrix_html_escape(label),
        matrix_html_escape(&value),
        delta
    )
}

fn history_delta(value: f32, higher_is_better: bool) -> String {
    let class = if value.abs() < f32::EPSILON {
        "flat"
    } else if (value > 0.0 && higher_is_better) || (value < 0.0 && !higher_is_better) {
        "good"
    } else {
        "bad"
    };
    format!(
        "<span class=\"delta {class}\">{:+.1}%</span>",
        matrix_pct(value)
    )
}

fn markdown_optional_millis(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.0} ms"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn markdown_optional_millis_delta(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:+.0} ms"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn markdown_optional_number_delta(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:+.1}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn html_optional_millis(value: Option<f32>) -> String {
    markdown_optional_millis(value)
}

fn html_optional_millis_delta(value: Option<f32>) -> String {
    match value {
        Some(value) => {
            let class = if value.abs() < f32::EPSILON {
                "flat"
            } else if value < 0.0 {
                "good"
            } else {
                "bad"
            };
            format!("<span class=\"delta {class}\">{value:+.0} ms</span>")
        }
        None => "<span class=\"delta flat\">n/a</span>".to_string(),
    }
}

fn html_optional_number(value: Option<f32>) -> String {
    value
        .map(|value| format!("{value:.1}"))
        .unwrap_or_else(|| "n/a".to_string())
}

fn html_optional_number_delta(value: Option<f32>, higher_is_better: bool) -> String {
    match value {
        Some(value) => {
            let class = if value.abs() < f32::EPSILON {
                "flat"
            } else if (value > 0.0 && higher_is_better) || (value < 0.0 && !higher_is_better) {
                "good"
            } else {
                "bad"
            };
            format!("<span class=\"delta {class}\">{value:+.1}</span>")
        }
        None => "<span class=\"delta flat\">n/a</span>".to_string(),
    }
}

fn matrix_pct(value: f32) -> f32 {
    value * 100.0
}

fn matrix_html_escape(value: &str) -> String {
    value
        .replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}

fn yes_no(value: bool) -> &'static str {
    if value {
        "yes"
    } else {
        "no"
    }
}

const MATRIX_HISTORY_CSS: &str = r#"
:root {
  color-scheme: light;
  --bg: #f6f7f2;
  --ink: #17211b;
  --muted: #647068;
  --line: #d7ddd1;
  --panel: #ffffff;
  --accent: #245c4f;
  --accent-2: #3759a8;
  --good: #116d3f;
  --bad: #a13f2d;
  --flat: #6b6f76;
}
* { box-sizing: border-box; }
body {
  margin: 0;
  background: var(--bg);
  color: var(--ink);
  font-family: ui-sans-serif, system-ui, -apple-system, BlinkMacSystemFont, "Segoe UI", sans-serif;
}
.shell {
  width: min(1180px, calc(100vw - 32px));
  margin: 0 auto;
  padding: 32px 0 48px;
}
.hero {
  display: flex;
  justify-content: space-between;
  gap: 24px;
  align-items: flex-start;
  padding: 8px 0 28px;
}
.eyebrow {
  margin: 0 0 8px;
  color: var(--accent);
  font-size: 0.78rem;
  font-weight: 700;
  text-transform: uppercase;
  letter-spacing: 0.08em;
}
h1 {
  margin: 0;
  font-size: clamp(2rem, 5vw, 4.2rem);
  line-height: 1;
  letter-spacing: 0;
}
.lede {
  max-width: 720px;
  margin: 14px 0 0;
  color: var(--muted);
  font-size: 1.05rem;
}
.privacy-badge {
  border: 1px solid var(--line);
  border-radius: 8px;
  background: var(--panel);
  padding: 10px 12px;
  color: var(--accent);
  font-weight: 700;
  white-space: nowrap;
}
.summary-grid {
  display: grid;
  grid-template-columns: repeat(auto-fit, minmax(260px, 1fr));
  gap: 14px;
  margin-bottom: 18px;
}
.run-card,
.panel,
.privacy {
  background: var(--panel);
  border: 1px solid var(--line);
  border-radius: 8px;
}
.run-card {
  padding: 16px;
}
.run-title {
  display: flex;
  justify-content: space-between;
  gap: 12px;
  align-items: center;
  font-weight: 800;
}
code {
  color: var(--accent-2);
  font-family: ui-monospace, SFMono-Regular, Menlo, Consolas, monospace;
  font-size: 0.86em;
}
.suite {
  margin: 8px 0 14px;
  color: var(--muted);
}
.metric-row {
  display: grid;
  grid-template-columns: repeat(2, minmax(0, 1fr));
  gap: 10px;
}
.metric {
  border: 1px solid var(--line);
  border-radius: 8px;
  padding: 10px;
  min-height: 88px;
}
.metric span,
.metric em {
  display: block;
  color: var(--muted);
  font-size: 0.82rem;
  font-style: normal;
}
.metric strong {
  display: block;
  margin: 5px 0;
  font-size: 1.45rem;
}
.delta.good { color: var(--good); font-weight: 800; }
.delta.bad { color: var(--bad); font-weight: 800; }
.delta.flat { color: var(--flat); font-weight: 800; }
.panel,
.privacy {
  padding: 18px;
  margin-top: 18px;
}
h2 {
  margin: 0 0 14px;
  font-size: 1.12rem;
}
.table-wrap {
  overflow-x: auto;
}
table {
  width: 100%;
  border-collapse: collapse;
  min-width: 900px;
}
th,
td {
  border-top: 1px solid var(--line);
  padding: 10px 8px;
  text-align: left;
  vertical-align: top;
}
th {
  color: var(--muted);
  font-size: 0.78rem;
  text-transform: uppercase;
  letter-spacing: 0.04em;
}
.privacy p {
  margin: 0;
  color: var(--muted);
}
@media (max-width: 760px) {
  .hero { display: block; }
  .privacy-badge { display: inline-block; margin-top: 16px; }
  .metric-row { grid-template-columns: 1fr; }
}
"#;

fn verify_matrix_run(matrix_dir: &Path, run: &RunMatrixManifestRun) -> Result<()> {
    if run.name.trim().is_empty() {
        anyhow::bail!("matrix run name must not be empty");
    }
    if run.agent.trim().is_empty() {
        anyhow::bail!("matrix run `{}` agent must not be empty", run.name);
    }
    if run
        .adapter_command_hash
        .as_deref()
        .is_some_and(|hash| !hash.starts_with("cmd:"))
    {
        anyhow::bail!(
            "matrix run `{}` adapter command hash must be source-free",
            run.name
        );
    }
    if run
        .ctxhelm_config_hash
        .as_deref()
        .is_some_and(|hash| !hash.starts_with("ctxhelm:"))
    {
        anyhow::bail!(
            "matrix run `{}` ctxhelm config hash must be source-free",
            run.name
        );
    }
    require_matrix_file(matrix_dir, &run.report_path)
        .with_context(|| format!("verify report for run `{}`", run.name))?;
    require_matrix_dir(matrix_dir, &run.trace_dir)
        .with_context(|| format!("verify trace dir for run `{}`", run.name))?;
    require_matrix_file(matrix_dir, &run.autopsy_markdown)
        .with_context(|| format!("verify autopsy for run `{}`", run.name))?;
    if let Some(path) = &run.comparison_json {
        require_matrix_file(matrix_dir, path)
            .with_context(|| format!("verify comparison JSON for run `{}`", run.name))?;
    }
    if let Some(path) = &run.comparison_markdown {
        require_matrix_file(matrix_dir, path)
            .with_context(|| format!("verify comparison Markdown for run `{}`", run.name))?;
    }
    Ok(())
}

fn require_matrix_file(matrix_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    let path = matrix_path(matrix_dir, relative_path)?;
    if !path.is_file() {
        anyhow::bail!(
            "matrix artifact `{}` is missing or not a file",
            relative_path
        );
    }
    Ok(path)
}

fn require_matrix_dir(matrix_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    let path = matrix_path(matrix_dir, relative_path)?;
    if !path.is_dir() {
        anyhow::bail!(
            "matrix artifact `{}` is missing or not a directory",
            relative_path
        );
    }
    Ok(path)
}

fn matrix_path(matrix_dir: &Path, relative_path: &str) -> Result<PathBuf> {
    helmbench::validate_safe_relative_path_for_cli(relative_path)
        .with_context(|| format!("validate matrix path `{relative_path}`"))?;
    Ok(matrix_dir.join(relative_path))
}

#[allow(clippy::too_many_arguments)]
fn run_matrix_spec(
    suite: &helmbench::TaskSuite,
    repo: &Path,
    work_dir: &Path,
    traces_dir: &Path,
    reports_dir: &Path,
    spec: &RunMatrixSpec,
    setup_commands: &[String],
    keep_workdirs: bool,
) -> Result<RunMatrixResult> {
    let trace_dir = traces_dir.join(&spec.safe_name);
    run_local_suite(
        suite,
        repo,
        &work_dir.join(&spec.safe_name),
        &trace_dir,
        &spec.agent,
        spec.variant.clone(),
        setup_commands,
        spec.ctxhelm.as_ref(),
        spec.adapter_command.as_deref(),
        spec.capture_stream,
        keep_workdirs,
    )
    .with_context(|| format!("run matrix spec `{}`", spec.name))?;
    let report = build_report(suite, &load_traces(&trace_dir)?)?;
    let report_path = reports_dir.join(format!("{}.json", spec.safe_name));
    write_json(&report, &report_path)?;
    write_text(
        &render_markdown_report(&report),
        &reports_dir.join(format!("{}.md", spec.safe_name)),
    )?;
    Ok(RunMatrixResult {
        spec: spec.clone(),
        report,
        report_path,
        trace_dir,
    })
}

fn parse_run_matrix_spec(raw: &str) -> Result<RunMatrixSpec> {
    let mut name = None;
    let mut agent = None;
    let mut variant = None;
    let mut command = None;
    let mut preset = None;
    let mut preset_bin = None;
    let mut preset_model = None;
    let mut preset_args = Vec::new();
    let mut dangerously_skip_permissions = false;
    let mut dangerously_bypass_approvals_and_sandbox = false;
    let mut use_ctxhelm = false;
    let mut ctxhelm_bin = PathBuf::from("ctxhelm");
    let mut mode = "explain".to_string();
    let mut target_agent = "generic".to_string();
    let mut semantic = false;
    let mut semantic_provider = None;
    let mut semantic_model = None;
    let mut semantic_dimensions = None;
    let mut pack = false;
    let mut pack_budget = "brief".to_string();
    let mut capture_stream = false;

    for part in raw.split(',') {
        let part = part.trim();
        if part.is_empty() {
            continue;
        }
        let Some((key, value)) = part.split_once('=') else {
            anyhow::bail!("run spec part `{part}` must use key=value");
        };
        let key = key.trim();
        let value = value.trim();
        match key {
            "name" => name = Some(value.to_string()),
            "agent" => agent = Some(value.to_string()),
            "variant" => variant = Some(parse_agent_variant(value)?),
            "ctxhelm" => use_ctxhelm = parse_bool_field("ctxhelm", value)?,
            "ctxhelm_bin" => {
                use_ctxhelm = true;
                ctxhelm_bin = PathBuf::from(value);
            }
            "mode" => {
                use_ctxhelm = true;
                mode = value.to_string();
            }
            "target_agent" => {
                use_ctxhelm = true;
                target_agent = value.to_string();
            }
            "semantic" => {
                use_ctxhelm = true;
                semantic = parse_bool_field("semantic", value)?;
            }
            "semantic_provider" => {
                use_ctxhelm = true;
                semantic_provider = non_empty_string(value);
            }
            "semantic_model" => {
                use_ctxhelm = true;
                semantic_model = non_empty_string(value);
            }
            "semantic_dimensions" => {
                use_ctxhelm = true;
                semantic_dimensions = Some(
                    value
                        .parse::<u16>()
                        .with_context(|| format!("parse semantic_dimensions `{value}`"))?,
                );
            }
            "pack" => {
                use_ctxhelm = true;
                pack = parse_bool_field("pack", value)?;
            }
            "pack_budget" => {
                use_ctxhelm = true;
                pack_budget = value.to_string();
            }
            "command" => {
                if !value.is_empty() {
                    command = Some(value.to_string());
                }
            }
            "preset" => preset = Some(parse_adapter_preset(value)?),
            "bin" | "adapter_bin" => preset_bin = Some(PathBuf::from(value)),
            "model" => preset_model = non_empty_string(value),
            "arg" | "adapter_arg" => {
                if !value.is_empty() {
                    preset_args.push(value.to_string());
                }
            }
            "dangerously_skip_permissions" => {
                dangerously_skip_permissions = parse_bool_field(key, value)?;
            }
            "dangerously_bypass_approvals_and_sandbox" => {
                dangerously_bypass_approvals_and_sandbox = parse_bool_field(key, value)?;
            }
            "capture_stream" => {
                capture_stream = parse_bool_field(key, value)?;
            }
            other => anyhow::bail!("unsupported run spec field `{other}`"),
        }
    }

    let name = name.context("run spec requires name=<id>")?;
    if name.trim().is_empty() {
        anyhow::bail!("run spec name must not be empty");
    }
    let agent = agent.context("run spec requires agent=<agent>")?;
    if agent.trim().is_empty() {
        anyhow::bail!("run spec agent must not be empty");
    }
    let safe_name = safe_task_dir_name(&name);
    let adapter_preset = preset.map(|preset| AdapterPresetRunConfig {
        preset,
        bin: preset_bin.unwrap_or_else(|| default_preset_bin(preset)),
        model: preset_model,
        args: preset_args,
        dangerously_skip_permissions,
        dangerously_bypass_approvals_and_sandbox,
    });
    let adapter_command =
        adapter_command_from_preset(command, adapter_preset.as_ref(), capture_stream)?;
    Ok(RunMatrixSpec {
        name,
        safe_name,
        agent,
        variant: variant.context("run spec requires variant=<variant>")?,
        ctxhelm: use_ctxhelm.then_some(CtxhelmRunConfig {
            ctxhelm_bin,
            mode,
            target_agent,
            semantic,
            semantic_provider,
            semantic_model,
            semantic_dimensions,
            include_pack: pack,
            pack_budget,
        }),
        adapter_command,
        adapter_preset,
        capture_stream,
    })
}

fn parse_adapter_preset(value: &str) -> Result<AdapterPreset> {
    match value {
        "claude-code" | "claude" => Ok(AdapterPreset::ClaudeCode),
        "codex" => Ok(AdapterPreset::Codex),
        _ => anyhow::bail!("preset must be claude-code or codex, got `{value}`"),
    }
}

fn adapter_preset_label(preset: Option<AdapterPreset>) -> Option<&'static str> {
    match preset {
        Some(AdapterPreset::ClaudeCode) => Some("claude-code"),
        Some(AdapterPreset::Codex) => Some("codex"),
        None => None,
    }
}

fn default_preset_bin(preset: AdapterPreset) -> PathBuf {
    match preset {
        AdapterPreset::ClaudeCode => PathBuf::from("claude"),
        AdapterPreset::Codex => PathBuf::from("codex"),
    }
}

fn adapter_command_from_preset(
    explicit_command: Option<String>,
    preset: Option<&AdapterPresetRunConfig>,
    capture_stream: bool,
) -> Result<Option<String>> {
    match (explicit_command, preset) {
        (Some(_), Some(_)) => anyhow::bail!("run spec cannot combine command and preset"),
        (Some(command), None) => Ok(Some(command)),
        (None, None) => Ok(None),
        (None, Some(config)) => {
            let helmbench_bin = current_helmbench_bin()?;
            let suppress_output = !capture_stream;
            let command = match config.preset {
                AdapterPreset::ClaudeCode => claude_adapter_command(
                    &helmbench_bin,
                    &config.bin,
                    config.model.as_deref(),
                    &config.args,
                    config.dangerously_skip_permissions,
                    suppress_output,
                ),
                AdapterPreset::Codex => codex_adapter_command(
                    &helmbench_bin,
                    &config.bin,
                    config.model.as_deref(),
                    &config.args,
                    config.dangerously_bypass_approvals_and_sandbox,
                    suppress_output,
                ),
            };
            Ok(Some(command))
        }
    }
}

fn parse_bool_field(name: &str, value: &str) -> Result<bool> {
    match value {
        "true" | "1" | "yes" => Ok(true),
        "false" | "0" | "no" => Ok(false),
        _ => anyhow::bail!("{name} must be true or false, got `{value}`"),
    }
}

fn non_empty_string(value: &str) -> Option<String> {
    (!value.trim().is_empty()).then(|| value.to_string())
}

fn validate_run_matrix_specs(baseline: &RunMatrixSpec, heads: &[RunMatrixSpec]) -> Result<()> {
    if heads.is_empty() {
        anyhow::bail!("run-matrix requires at least one --head");
    }
    let mut names = BTreeSet::new();
    for spec in std::iter::once(baseline).chain(heads.iter()) {
        if !names.insert(spec.safe_name.clone()) {
            anyhow::bail!("duplicate run-matrix name `{}`", spec.safe_name);
        }
    }
    Ok(())
}

fn parse_agent_variant(value: &str) -> Result<AgentVariant> {
    match value {
        "native" => Ok(AgentVariant::Native),
        "native_search" | "native-search" => Ok(AgentVariant::NativeSearch),
        "ctxhelm_plan" => Ok(AgentVariant::CtxhelmPlan),
        "ctxhelm_mcp" => Ok(AgentVariant::CtxhelmMcp),
        "ctxhelm_pack" => Ok(AgentVariant::CtxhelmPack),
        "other" => Ok(AgentVariant::Other),
        _ => anyhow::bail!("unsupported variant `{value}`"),
    }
}

fn run_demo_pipeline_with_adapter(
    out_dir: &Path,
    force: bool,
    adapter_command_override: Option<String>,
) -> Result<()> {
    if out_dir.exists() {
        if !force {
            anyhow::bail!(
                "{} already exists; pass --force to replace it",
                out_dir.display()
            );
        }
        std::fs::remove_dir_all(out_dir)
            .with_context(|| format!("remove {}", out_dir.display()))?;
    }
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let repo = out_dir.join("repo");
    let suite_path = out_dir.join("suite.json");
    let reports_dir = out_dir.join("reports");
    let traces_dir = out_dir.join("traces");
    let work_dir = out_dir.join("workdirs");
    let docs_dir = out_dir.join("docs");
    std::fs::create_dir_all(&reports_dir)
        .with_context(|| format!("create {}", reports_dir.display()))?;
    std::fs::create_dir_all(&docs_dir).with_context(|| format!("create {}", docs_dir.display()))?;

    init_demo_repo(&repo, &suite_path, false)?;
    let suite = load_suite(&suite_path)?;

    let native_traces = traces_dir.join("native");
    run_local_suite(
        &suite,
        &repo,
        &work_dir.join("native"),
        &native_traces,
        "demo-baseline",
        AgentVariant::Native,
        &[],
        None,
        None,
        false,
        false,
    )?;
    let native_report = build_report(&suite, &load_traces(&native_traces)?)?;
    let native_report_path = reports_dir.join("native.json");
    write_json(&native_report, &native_report_path)?;
    write_text(
        &render_markdown_report(&native_report),
        &reports_dir.join("native.md"),
    )?;

    let guided_traces = traces_dir.join("guided");
    let adapter_command = match adapter_command_override {
        Some(command) => command,
        None => format!(
            "HELMBENCH_BIN={} sh scripts/demo-agent.sh",
            shell_escape(&current_helmbench_bin()?.to_string_lossy())
        ),
    };
    run_local_suite(
        &suite,
        &repo,
        &work_dir.join("guided"),
        &guided_traces,
        "demo-guided",
        AgentVariant::CtxhelmMcp,
        &[],
        None,
        Some(&adapter_command),
        false,
        false,
    )?;
    let guided_report = build_report(&suite, &load_traces(&guided_traces)?)?;
    let guided_report_path = reports_dir.join("guided.json");
    write_json(&guided_report, &guided_report_path)?;
    write_text(
        &render_markdown_report(&guided_report),
        &reports_dir.join("guided.md"),
    )?;

    let compare = compare_reports(&native_report, &guided_report);
    write_text(
        &render_markdown_compare(&compare),
        &docs_dir.join("compare.md"),
    )?;

    let summary = build_benchmark_summary(&native_report, std::slice::from_ref(&guided_report))?;
    let summary_json_path = reports_dir.join("benchmark-summary.json");
    write_json(&summary, &summary_json_path)?;
    write_text(
        &render_markdown_benchmark_summary(&summary),
        &docs_dir.join("benchmark-summary.md"),
    )?;

    let gate = evaluate_quality_gate(&summary, &QualityGateConfig::default())?;
    write_json(&gate, &reports_dir.join("quality-gate.json"))?;
    write_text(
        &render_markdown_quality_gate(&gate),
        &docs_dir.join("quality-gate.md"),
    )?;
    if !gate.passed {
        anyhow::bail!("demo quality gate failed");
    }

    let native_result = RunMatrixResult {
        spec: RunMatrixSpec {
            name: "native".to_string(),
            safe_name: "native".to_string(),
            agent: "demo-baseline".to_string(),
            variant: AgentVariant::Native,
            ctxhelm: None,
            adapter_command: None,
            adapter_preset: None,
            capture_stream: false,
        },
        report: native_report.clone(),
        report_path: native_report_path.clone(),
        trace_dir: native_traces.clone(),
    };
    let guided_result = RunMatrixResult {
        spec: RunMatrixSpec {
            name: "guided".to_string(),
            safe_name: "guided".to_string(),
            agent: "demo-guided".to_string(),
            variant: AgentVariant::CtxhelmMcp,
            ctxhelm: None,
            adapter_command: Some(adapter_command.clone()),
            adapter_preset: None,
            capture_stream: false,
        },
        report: guided_report.clone(),
        report_path: guided_report_path.clone(),
        trace_dir: guided_traces.clone(),
    };
    let privacy_report =
        build_run_matrix_privacy_report(&suite, out_dir, &[&native_result, &guided_result])?;
    write_json(&privacy_report, &reports_dir.join("privacy-report.json"))?;
    write_text(
        &render_markdown_run_matrix_privacy_report(&privacy_report),
        &docs_dir.join("privacy-report.md"),
    )?;

    let autopsy = build_autopsy(&suite, &load_traces(&native_traces)?)?;
    write_text(
        &render_markdown_autopsy(&autopsy),
        &docs_dir.join("native-autopsy.md"),
    )?;
    let dashboard = render_html_dashboard(&[native_report, guided_report])?;
    write_text(&dashboard, &docs_dir.join("dashboard.html"))?;

    write_evidence_bundle(
        &suite_path,
        None,
        &native_report_path,
        std::slice::from_ref(&guided_report_path),
        &out_dir.join("evidence"),
        false,
    )?;
    Ok(())
}

fn init_demo_repo(repo_out: &Path, suite_out: &Path, force: bool) -> Result<()> {
    if repo_out.exists() {
        if !force {
            anyhow::bail!(
                "{} already exists; pass --force to replace it",
                repo_out.display()
            );
        }
        std::fs::remove_dir_all(repo_out)
            .with_context(|| format!("remove {}", repo_out.display()))?;
    }
    std::fs::create_dir_all(repo_out).with_context(|| format!("create {}", repo_out.display()))?;
    write_demo_file(
        repo_out,
        "README.md",
        "# HelmBench Tiny Demo Repo\n\nA deterministic fixture repo for evaluating source-free coding-agent traces.\n",
    )?;
    write_demo_file(
        repo_out,
        "AGENTS.md",
        "# AGENTS.md\n\nKeep changes minimal. Run the task-specific shell test after editing.\n",
    )?;
    write_demo_file(
        repo_out,
        "src/auth/session.txt",
        "expired sessions redirect to /login\nactive sessions redirect to /dashboard\n",
    )?;
    write_demo_file(
        repo_out,
        "src/auth/middleware.txt",
        "middleware calls session redirect rules\n",
    )?;
    write_demo_file(
        repo_out,
        "src/billing/invoice.txt",
        "invoice rounding mode: round half up\ncurrency: USD\n",
    )?;
    write_demo_file(
        repo_out,
        "src/billing/tax.txt",
        "tax service reads invoice rounding mode\n",
    )?;
    write_demo_file(
        repo_out,
        "tests/auth/session.test.sh",
        "#!/usr/bin/env sh\nset -eu\ngrep -q '/login' src/auth/session.txt\n",
    )?;
    write_demo_file(
        repo_out,
        "tests/billing/invoice.test.sh",
        "#!/usr/bin/env sh\nset -eu\ngrep -q 'round half up' src/billing/invoice.txt\n",
    )?;
    write_demo_file(repo_out, "scripts/demo-agent.sh", DEMO_AGENT_SCRIPT)?;
    set_executable(&repo_out.join("tests/auth/session.test.sh"))?;
    set_executable(&repo_out.join("tests/billing/invoice.test.sh"))?;
    set_executable(&repo_out.join("scripts/demo-agent.sh"))?;
    init_git_repo(repo_out)?;

    let suite = helmbench::TaskSuite {
        schema_version: helmbench::SUITE_SCHEMA_VERSION,
        name: "demo-tiny-repo".to_string(),
        description: "Two-task deterministic demo suite generated by helmbench init-demo-repo."
            .to_string(),
        tasks: vec![
            helmbench::BenchTask {
                id: "demo-auth-redirect-001".to_string(),
                prompt: "Fix expired sessions so they redirect to /login.".to_string(),
                expected_files: vec!["src/auth/session.txt".to_string()],
                expected_tests: vec!["tests/auth/session.test.sh".to_string()],
                success_command: Some("sh tests/auth/session.test.sh".to_string()),
                setup_commands: vec![
                    "printf 'expired sessions redirect to /expired\\nactive sessions redirect to /dashboard\\n' > src/auth/session.txt".to_string(),
                ],
                tags: vec!["bug_fix".to_string(), "auth".to_string()],
                timeout_seconds: Some(60),
            },
            helmbench::BenchTask {
                id: "demo-billing-rounding-001".to_string(),
                prompt: "Fix invoice rounding so it uses round half up.".to_string(),
                expected_files: vec!["src/billing/invoice.txt".to_string()],
                expected_tests: vec!["tests/billing/invoice.test.sh".to_string()],
                success_command: Some("sh tests/billing/invoice.test.sh".to_string()),
                setup_commands: vec![
                    "printf 'invoice rounding mode: floor\\ncurrency: USD\\n' > src/billing/invoice.txt"
                        .to_string(),
                ],
                tags: vec!["bug_fix".to_string(), "billing".to_string()],
                timeout_seconds: Some(60),
            },
        ],
    };
    validate_suite(&suite)?;
    write_json(&suite, suite_out)?;
    Ok(())
}

fn write_demo_file(repo: &Path, relative: &str, content: &str) -> Result<()> {
    let path = repo.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    std::fs::write(&path, content).with_context(|| format!("write {}", path.display()))
}

fn set_executable(path: &Path) -> Result<()> {
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut permissions = std::fs::metadata(path)
            .with_context(|| format!("metadata {}", path.display()))?
            .permissions();
        permissions.set_mode(0o755);
        std::fs::set_permissions(path, permissions)
            .with_context(|| format!("chmod {}", path.display()))?;
    }
    Ok(())
}

fn init_git_repo(repo: &Path) -> Result<()> {
    let init = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("init")
        .arg("--quiet")
        .status()
        .with_context(|| format!("git init {}", repo.display()))?;
    if !init.success() {
        anyhow::bail!("git init failed with status {:?}", init.code());
    }
    let add = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("add")
        .arg(".")
        .status()
        .with_context(|| format!("git add {}", repo.display()))?;
    if !add.success() {
        anyhow::bail!("git add failed with status {:?}", add.code());
    }
    let commit = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("-c")
        .arg("user.name=HelmBench")
        .arg("-c")
        .arg("user.email=helmbench@example.test")
        .arg("commit")
        .arg("--quiet")
        .arg("-m")
        .arg("Create tiny benchmark fixture")
        .status()
        .with_context(|| format!("git commit {}", repo.display()))?;
    if !commit.success() {
        anyhow::bail!("git commit failed with status {:?}", commit.code());
    }
    Ok(())
}

const DEMO_AGENT_SCRIPT: &str = r#"#!/usr/bin/env sh
set -eu

: "${HELMBENCH_BIN:?HELMBENCH_BIN is required}"
: "${HELMBENCH_EVENTS:?HELMBENCH_EVENTS is required}"
: "${HELMBENCH_TASK_ID:?HELMBENCH_TASK_ID is required}"

record_read() {
  "$HELMBENCH_BIN" record-event \
    --events "$HELMBENCH_EVENTS" \
    --task-id "$HELMBENCH_TASK_ID" \
    --event-kind file-read \
    --path "$1" \
    --observed-at-millis "$2"
}

case "$HELMBENCH_TASK_ID" in
  demo-auth-redirect-001)
    "$HELMBENCH_BIN" record-event \
      --events "$HELMBENCH_EVENTS" \
      --task-id "$HELMBENCH_TASK_ID" \
      --event-kind recommended-file \
      --path src/auth/session.txt \
      --observed-at-millis 5
    record_read src/auth/session.txt 15
    printf 'expired sessions redirect to /login\nactive sessions redirect to /dashboard\n' > src/auth/session.txt
    ;;
  demo-billing-rounding-001)
    "$HELMBENCH_BIN" record-event \
      --events "$HELMBENCH_EVENTS" \
      --task-id "$HELMBENCH_TASK_ID" \
      --event-kind recommended-file \
      --path src/billing/invoice.txt \
      --observed-at-millis 5
    record_read src/billing/invoice.txt 15
    printf 'invoice rounding mode: round half up\ncurrency: USD\n' > src/billing/invoice.txt
    ;;
  *)
    echo "unknown task: $HELMBENCH_TASK_ID" >&2
    exit 2
    ;;
esac
"#;

#[allow(clippy::too_many_arguments)]
fn run_local_suite(
    suite: &helmbench::TaskSuite,
    repo: &Path,
    work_dir: &Path,
    out_dir: &Path,
    agent: &str,
    variant: AgentVariant,
    setup_commands: &[String],
    ctxhelm: Option<&CtxhelmRunConfig>,
    adapter_command: Option<&str>,
    capture_stream: bool,
    keep_workdirs: bool,
) -> Result<()> {
    let repo =
        std::fs::canonicalize(repo).with_context(|| format!("resolve {}", repo.display()))?;
    if !repo.join(".git").exists() {
        anyhow::bail!("local-run requires a git repository: {}", repo.display());
    }
    std::fs::create_dir_all(work_dir).with_context(|| format!("create {}", work_dir.display()))?;
    std::fs::create_dir_all(out_dir).with_context(|| format!("create {}", out_dir.display()))?;
    let work_dir = std::fs::canonicalize(work_dir)
        .with_context(|| format!("resolve {}", work_dir.display()))?;
    let out_dir =
        std::fs::canonicalize(out_dir).with_context(|| format!("resolve {}", out_dir.display()))?;

    for task in &suite.tasks {
        let task_started = Instant::now();
        let task_dir = work_dir.join(safe_task_dir_name(&task.id));
        if task_dir.exists() {
            std::fs::remove_dir_all(&task_dir)
                .with_context(|| format!("remove {}", task_dir.display()))?;
        }
        clone_repo(&repo, &task_dir)?;

        let events = task_dir.join(".helmbench/events.jsonl");
        for setup in setup_commands {
            let setup_result = run_shell(setup, &task_dir, &[], task.timeout_seconds)?;
            if !setup_result.success {
                anyhow::bail!(
                    "setup command failed for `{}` with status {:?}{}",
                    task.id,
                    setup_result.exit_status,
                    timed_out_suffix(setup_result.timed_out)
                );
            }
        }
        if !run_task_setup_commands(task, &task_dir, false)? {
            anyhow::bail!("task setup command failed for `{}`", task.id);
        }
        commit_setup_baseline_if_changed(&task_dir)
            .with_context(|| format!("snapshot setup baseline for `{}`", task.id))?;

        if let Some(ctxhelm) = ctxhelm {
            append_ctxhelm_events(ctxhelm, task, &task_dir, &events, task_started)?;
        }

        let mut adapter_ok = true;
        if let Some(command) = adapter_command {
            let rendered = render_adapter_command(command, &task.id, &task_dir, &events);
            let prompt = task.prompt.as_str();
            let env = [
                ("HELMBENCH_TASK_ID", task.id.as_str()),
                ("HELMBENCH_TASK_PROMPT", prompt),
                ("HELMBENCH_REPO", path_as_str(&task_dir)?),
                ("HELMBENCH_EVENTS", path_as_str(&events)?),
                ("HELMBENCH_SUITE_NAME", suite.name.as_str()),
            ];
            let result = if capture_stream {
                let result =
                    run_shell_capture_stdout(&rendered, &task_dir, &env, task.timeout_seconds)?;
                if result.stdout_truncated {
                    anyhow::bail!(
                        "captured stream for `{}` exceeded the source-free parse buffer",
                        task.id
                    );
                }
                append_stream_capture_events(
                    task,
                    &task_dir,
                    &events,
                    &result.stdout,
                    task_started,
                )?;
                ShellResult {
                    success: result.success,
                    exit_status: result.exit_status,
                    elapsed_millis: result.elapsed_millis,
                    timed_out: result.timed_out,
                }
            } else {
                run_shell(&rendered, &task_dir, &env, task.timeout_seconds)?
            };
            adapter_ok = result.success;
        }

        let edited_paths = git_changed_paths(&task_dir)?;
        for path in edited_paths {
            append_event(
                &events,
                &path_event(&task.id, AgentEventKind::FileEdit, path, None)?,
            )?;
        }

        let mut validation_ok = true;
        if let Some(command) = &task.success_command {
            let result = run_shell(command, &task_dir, &[], task.timeout_seconds)?;
            validation_ok = result.success;
            append_event(
                &events,
                &AgentEvent {
                    schema_version: TRACE_SCHEMA_VERSION,
                    task_id: task.id.clone(),
                    event_kind: AgentEventKind::Command,
                    path: None,
                    command_class: Some(infer_command_class(command)),
                    command_hash: Some(command_hash(command)),
                    touched_tests: task.expected_tests.clone(),
                    exit_status: result.exit_status,
                    status: None,
                    token_estimate: None,
                    elapsed_millis: Some(result.elapsed_millis),
                    observed_at_millis: Some(task_started.elapsed().as_millis() as u64),
                    privacy: PrivacyStatus::source_free(),
                },
            )?;
        }

        let final_status = if adapter_ok && validation_ok {
            TaskStatus::Success
        } else {
            TaskStatus::Failure
        };
        append_event(
            &events,
            &AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: task.id.clone(),
                event_kind: AgentEventKind::Status,
                path: None,
                command_class: None,
                command_hash: None,
                touched_tests: Vec::new(),
                exit_status: None,
                status: Some(final_status),
                token_estimate: None,
                elapsed_millis: None,
                observed_at_millis: Some(task_started.elapsed().as_millis() as u64),
                privacy: PrivacyStatus::source_free(),
            },
        )?;

        let events = load_agent_events(&events)?;
        let traces = traces_from_agent_events(suite, &events, agent, variant.clone())?;
        let trace = traces
            .into_iter()
            .find(|trace| trace.task_id == task.id)
            .with_context(|| format!("trace for `{}`", task.id))?;
        let out = out_dir.join(format!("{}.json", task.id));
        write_json(&trace, &out)?;
        println!("wrote {}", out.display());

        if !keep_workdirs {
            std::fs::remove_dir_all(&task_dir)
                .with_context(|| format!("remove {}", task_dir.display()))?;
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct CtxhelmRunConfig {
    ctxhelm_bin: PathBuf,
    mode: String,
    target_agent: String,
    semantic: bool,
    semantic_provider: Option<String>,
    semantic_model: Option<String>,
    semantic_dimensions: Option<u16>,
    include_pack: bool,
    pack_budget: String,
}

fn append_ctxhelm_events(
    config: &CtxhelmRunConfig,
    task: &helmbench::BenchTask,
    repo: &Path,
    events: &PathBuf,
    task_started: Instant,
) -> Result<()> {
    let prepare = run_ctxhelm_json(config, repo, &task.prompt, false)
        .with_context(|| format!("ctxhelm prepare-task for `{}`", task.id))?;
    let value =
        serde_json::from_str::<serde_json::Value>(&prepare).context("parse ctxhelm JSON")?;
    let mut recommended = Vec::new();
    collect_ctxhelm_paths(&value, "targetFiles", &mut recommended)?;
    collect_ctxhelm_paths(&value, "relatedTests", &mut recommended)?;
    recommended.sort();
    recommended.dedup();
    for path in recommended {
        append_event(
            events,
            &path_event(
                &task.id,
                AgentEventKind::RecommendedFile,
                path,
                Some(task_started.elapsed().as_millis() as u64),
            )?,
        )?;
    }
    append_event(
        events,
        &AgentEvent {
            schema_version: TRACE_SCHEMA_VERSION,
            task_id: task.id.clone(),
            event_kind: AgentEventKind::Command,
            path: None,
            command_class: Some(CommandClass::Other),
            command_hash: Some(command_hash("ctxhelm prepare-task")),
            touched_tests: Vec::new(),
            exit_status: Some(0),
            status: None,
            token_estimate: None,
            elapsed_millis: None,
            observed_at_millis: Some(task_started.elapsed().as_millis() as u64),
            privacy: PrivacyStatus::source_free(),
        },
    )?;

    if config.include_pack {
        let pack = run_ctxhelm_json(config, repo, &task.prompt, true)
            .with_context(|| format!("ctxhelm get-pack for `{}`", task.id))?;
        let value = serde_json::from_str::<serde_json::Value>(&pack).context("parse pack JSON")?;
        let token_estimate = value.get("tokenEstimate").and_then(|value| value.as_u64());
        if let Some(tokens) = token_estimate {
            append_event(
                events,
                &AgentEvent {
                    schema_version: TRACE_SCHEMA_VERSION,
                    task_id: task.id.clone(),
                    event_kind: AgentEventKind::Usage,
                    path: None,
                    command_class: None,
                    command_hash: None,
                    touched_tests: Vec::new(),
                    exit_status: None,
                    status: None,
                    token_estimate: Some(tokens),
                    elapsed_millis: None,
                    observed_at_millis: Some(task_started.elapsed().as_millis() as u64),
                    privacy: PrivacyStatus::source_free(),
                },
            )?;
        }
        append_event(
            events,
            &AgentEvent {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: task.id.clone(),
                event_kind: AgentEventKind::Command,
                path: None,
                command_class: Some(CommandClass::Other),
                command_hash: Some(command_hash(&format!(
                    "ctxhelm get-pack {}",
                    config.pack_budget
                ))),
                touched_tests: Vec::new(),
                exit_status: Some(0),
                status: None,
                token_estimate: None,
                elapsed_millis: None,
                observed_at_millis: Some(task_started.elapsed().as_millis() as u64),
                privacy: PrivacyStatus::source_free(),
            },
        )?;
    }
    Ok(())
}

fn run_ctxhelm_json(
    config: &CtxhelmRunConfig,
    repo: &Path,
    task_prompt: &str,
    pack: bool,
) -> Result<String> {
    let mut command = ProcessCommand::new(&config.ctxhelm_bin);
    if pack {
        command
            .arg("get-pack")
            .arg("--budget")
            .arg(&config.pack_budget)
            .arg("--format")
            .arg("json");
    } else {
        command.arg("prepare-task");
    }
    command
        .arg("--repo")
        .arg(repo)
        .arg("--mode")
        .arg(&config.mode)
        .arg("--target-agent")
        .arg(&config.target_agent)
        .arg("--no-trace");
    if config.semantic {
        command.arg("--semantic");
    }
    if let Some(provider) = &config.semantic_provider {
        command.arg("--semantic-provider").arg(provider);
    }
    if let Some(model) = &config.semantic_model {
        command.arg("--semantic-model").arg(model);
    }
    if let Some(dimensions) = config.semantic_dimensions {
        command
            .arg("--semantic-dimensions")
            .arg(dimensions.to_string());
    }
    command.arg(task_prompt);
    let output = command
        .output()
        .with_context(|| format!("run {}", config.ctxhelm_bin.display()))?;
    if !output.status.success() {
        anyhow::bail!("ctxhelm failed with status {:?}", output.status.code());
    }
    String::from_utf8(output.stdout).context("ctxhelm stdout utf8")
}

fn collect_ctxhelm_paths(
    value: &serde_json::Value,
    key: &str,
    output: &mut Vec<String>,
) -> Result<()> {
    let Some(items) = value.get(key).and_then(|items| items.as_array()) else {
        return Ok(());
    };
    for item in items {
        let Some(path) = item.get("path").and_then(|path| path.as_str()) else {
            continue;
        };
        helmbench::validate_safe_relative_path_for_cli(path)?;
        output.push(path.to_string());
    }
    Ok(())
}

fn append_event(path: &PathBuf, event: &AgentEvent) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
    validate_agent_event(event)?;
    let mut line = serde_json::to_string(event)?;
    line.push('\n');
    let mut file = std::fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(path)
        .with_context(|| format!("open {}", path.display()))?;
    use std::io::Write;
    file.write_all(line.as_bytes())
        .with_context(|| format!("append {}", path.display()))
}

fn claude_adapter_command(
    helmbench_bin: &Path,
    claude_bin: &Path,
    model: Option<&str>,
    extra_args: &[String],
    dangerously_skip_permissions: bool,
    suppress_output: bool,
) -> String {
    let mut parts = vec![
        format!(
            "HELMBENCH_BIN={}",
            shell_escape(&helmbench_bin.to_string_lossy())
        ),
        shell_escape(&claude_bin.to_string_lossy()),
        "--print".to_string(),
        "--output-format".to_string(),
        "text".to_string(),
        "--no-session-persistence".to_string(),
        "--append-system-prompt".to_string(),
        shell_escape(AGENT_EVENT_INSTRUCTIONS),
    ];
    if dangerously_skip_permissions {
        parts.push("--dangerously-skip-permissions".to_string());
    }
    if let Some(model) = model {
        parts.push("--model".to_string());
        parts.push(shell_escape(model));
    }
    parts.extend(extra_args.iter().map(|arg| shell_escape(arg)));
    parts.push("\"$HELMBENCH_TASK_PROMPT\"".to_string());
    if suppress_output {
        parts.push(">/dev/null".to_string());
        parts.push("2>/dev/null".to_string());
    }
    parts.join(" ")
}

fn codex_adapter_command(
    helmbench_bin: &Path,
    codex_bin: &Path,
    model: Option<&str>,
    extra_args: &[String],
    dangerously_bypass_approvals_and_sandbox: bool,
    suppress_output: bool,
) -> String {
    let mut parts = vec![
        format!(
            "HELMBENCH_BIN={}",
            shell_escape(&helmbench_bin.to_string_lossy())
        ),
        shell_escape(&codex_bin.to_string_lossy()),
        "exec".to_string(),
        "--cd".to_string(),
        "\"$HELMBENCH_REPO\"".to_string(),
    ];
    if dangerously_bypass_approvals_and_sandbox {
        parts.push("--dangerously-bypass-approvals-and-sandbox".to_string());
    } else {
        parts.push("--full-auto".to_string());
    }
    if let Some(model) = model {
        parts.push("--model".to_string());
        parts.push(shell_escape(model));
    }
    parts.extend(extra_args.iter().map(|arg| shell_escape(arg)));
    parts.push("\"$(printf '%s\\n\\nTask:\\n%s'".to_string());
    parts.push(shell_escape(AGENT_EVENT_INSTRUCTIONS));
    parts.push("\"$HELMBENCH_TASK_PROMPT\")\"".to_string());
    if suppress_output {
        parts.push(">/dev/null".to_string());
        parts.push("2>/dev/null".to_string());
    }
    parts.join(" ")
}

const AGENT_EVENT_INSTRUCTIONS: &str = r#"You are running inside HelmBench, a source-free evaluation harness.
Work only inside HELMBENCH_REPO.
Before or immediately after inspecting a relevant file, emit:
$HELMBENCH_BIN record-event --events "$HELMBENCH_EVENTS" --task-id "$HELMBENCH_TASK_ID" --event-kind file-read --path <relative-path>
If ctxhelm or another context source recommends a file, emit event-kind recommended-file.
Do not put source code, model text, terminal output, secrets, or raw tool payloads into HelmBench events.
HelmBench will infer edited files from git status and run the task validation command after you exit.
Make the smallest useful change for the task."#;

fn current_helmbench_bin() -> Result<PathBuf> {
    std::env::current_exe()
        .context("resolve current helmbench executable")
        .and_then(|path| {
            std::fs::canonicalize(&path).with_context(|| format!("resolve {}", path.display()))
        })
}

fn clone_repo(repo: &Path, out: &Path) -> Result<()> {
    let status = ProcessCommand::new("git")
        .arg("-c")
        .arg("advice.detachedHead=false")
        .arg("-c")
        .arg("init.defaultBranch=main")
        .arg("clone")
        .arg("--quiet")
        .arg("--no-hardlinks")
        .arg(repo)
        .arg(out)
        .status()
        .with_context(|| format!("git clone {} {}", repo.display(), out.display()))?;
    if !status.success() {
        anyhow::bail!("git clone failed with status {:?}", status.code());
    }
    Ok(())
}

fn run_shell(
    command: &str,
    cwd: &Path,
    env: &[(&str, &str)],
    timeout_seconds: Option<u64>,
) -> Result<ShellResult> {
    run_shell_with_output(command, cwd, env, timeout_seconds, false)
}

fn run_shell_suppressed(
    command: &str,
    cwd: &Path,
    env: &[(&str, &str)],
    timeout_seconds: Option<u64>,
) -> Result<ShellResult> {
    run_shell_with_output(command, cwd, env, timeout_seconds, true)
}

fn run_shell_with_output(
    command: &str,
    cwd: &Path,
    env: &[(&str, &str)],
    timeout_seconds: Option<u64>,
    suppress_output: bool,
) -> Result<ShellResult> {
    let started = Instant::now();
    let mut process = ProcessCommand::new("sh");
    process.arg("-lc").arg(command).current_dir(cwd);
    if suppress_output {
        process.stdout(Stdio::null()).stderr(Stdio::null());
    }
    for (key, value) in env {
        process.env(key, value);
    }
    let mut child = process
        .spawn()
        .with_context(|| format!("run shell command in {}", cwd.display()))?;
    loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("wait for shell command in {}", cwd.display()))?
        {
            return Ok(ShellResult {
                success: status.success(),
                exit_status: status.code(),
                elapsed_millis: started.elapsed().as_millis() as u64,
                timed_out: false,
            });
        }
        if timeout_seconds.is_some_and(|seconds| started.elapsed() >= Duration::from_secs(seconds))
        {
            child
                .kill()
                .with_context(|| format!("kill timed-out command in {}", cwd.display()))?;
            let _ = child.wait();
            return Ok(ShellResult {
                success: false,
                exit_status: None,
                elapsed_millis: started.elapsed().as_millis() as u64,
                timed_out: true,
            });
        }
        std::thread::sleep(Duration::from_millis(25));
    }
}

fn run_shell_capture_stdout(
    command: &str,
    cwd: &Path,
    env: &[(&str, &str)],
    timeout_seconds: Option<u64>,
) -> Result<ShellCaptureResult> {
    const MAX_CAPTURE_BYTES: usize = 1024 * 1024;

    let started = Instant::now();
    let mut process = ProcessCommand::new("sh");
    process
        .arg("-lc")
        .arg(command)
        .current_dir(cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::null());
    for (key, value) in env {
        process.env(key, value);
    }
    let mut child = process
        .spawn()
        .with_context(|| format!("run shell command in {}", cwd.display()))?;
    let mut stdout = child
        .stdout
        .take()
        .context("capture stdout pipe from shell command")?;
    let reader = std::thread::spawn(move || {
        let mut captured = Vec::new();
        let mut buffer = [0u8; 8192];
        let mut truncated = false;
        loop {
            match stdout.read(&mut buffer) {
                Ok(0) => break,
                Ok(count) => {
                    let remaining = MAX_CAPTURE_BYTES.saturating_sub(captured.len());
                    if remaining > 0 {
                        captured.extend_from_slice(&buffer[..count.min(remaining)]);
                    }
                    if count > remaining {
                        truncated = true;
                    }
                }
                Err(error) => return Err(error),
            }
        }
        Ok::<_, std::io::Error>((captured, truncated))
    });

    let mut timed_out = false;
    let status = loop {
        if let Some(status) = child
            .try_wait()
            .with_context(|| format!("wait for shell command in {}", cwd.display()))?
        {
            break status;
        }
        if timeout_seconds.is_some_and(|seconds| started.elapsed() >= Duration::from_secs(seconds))
        {
            timed_out = true;
            child
                .kill()
                .with_context(|| format!("kill timed-out command in {}", cwd.display()))?;
            break child
                .wait()
                .with_context(|| format!("wait for killed shell command in {}", cwd.display()))?;
        }
        std::thread::sleep(Duration::from_millis(25));
    };

    let (stdout, truncated) = reader
        .join()
        .map_err(|_| anyhow::anyhow!("stdout capture thread panicked"))?
        .context("read captured stdout")?;
    let stdout = String::from_utf8_lossy(&stdout).into_owned();
    Ok(ShellCaptureResult {
        success: status.success() && !timed_out,
        exit_status: status.code(),
        elapsed_millis: started.elapsed().as_millis() as u64,
        timed_out,
        stdout,
        stdout_truncated: truncated,
    })
}

fn append_stream_capture_events(
    task: &helmbench::BenchTask,
    repo: &Path,
    events: &PathBuf,
    stdout: &str,
    task_started: Instant,
) -> Result<()> {
    if stdout.trim().is_empty() {
        return Ok(());
    }
    let stream_events =
        events_from_agent_stream_jsonl(&task.id, stdout, Some(repo), &task.expected_tests)
            .with_context(|| format!("parse captured stream for `{}`", task.id))?;
    for mut event in stream_events {
        if event.observed_at_millis.is_none() {
            event.observed_at_millis = Some(task_started.elapsed().as_millis() as u64);
        }
        append_event(events, &event)?;
    }
    Ok(())
}

struct ShellResult {
    success: bool,
    exit_status: Option<i32>,
    elapsed_millis: u64,
    timed_out: bool,
}

struct ShellCaptureResult {
    success: bool,
    exit_status: Option<i32>,
    elapsed_millis: u64,
    timed_out: bool,
    stdout: String,
    stdout_truncated: bool,
}

fn timed_out_suffix(timed_out: bool) -> &'static str {
    if timed_out {
        " (timed out)"
    } else {
        ""
    }
}

fn git_changed_paths(repo: &Path) -> Result<Vec<String>> {
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("status")
        .arg("--short")
        .arg("--untracked-files=all")
        .output()
        .with_context(|| format!("git status {}", repo.display()))?;
    if !output.status.success() {
        anyhow::bail!("git status failed with status {:?}", output.status.code());
    }
    let stdout = String::from_utf8(output.stdout).context("git status utf8")?;
    let mut paths = Vec::new();
    for line in stdout.lines() {
        if line.len() < 4 {
            continue;
        }
        let raw_path = line[3..].trim();
        let path = raw_path
            .rsplit_once(" -> ")
            .map_or(raw_path, |(_, new_path)| new_path)
            .trim_matches('"');
        if path.starts_with(".helmbench/") {
            continue;
        }
        helmbench::validate_safe_relative_path_for_cli(path)?;
        paths.push(path.to_string());
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn commit_setup_baseline_if_changed(repo: &Path) -> Result<()> {
    if git_changed_paths(repo)?.is_empty() {
        return Ok(());
    }
    run_git(repo, &["add", "-A", "--", "."])?;
    run_git(
        repo,
        &[
            "-c",
            "user.name=HelmBench",
            "-c",
            "user.email=helmbench@example.invalid",
            "commit",
            "--quiet",
            "--no-gpg-sign",
            "--no-verify",
            "-m",
            "helmbench setup baseline",
        ],
    )?;
    Ok(())
}

fn run_git(repo: &Path, args: &[&str]) -> Result<()> {
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .args(args)
        .output()
        .with_context(|| format!("git {} {}", args.join(" "), repo.display()))?;
    if !output.status.success() {
        anyhow::bail!(
            "git {} failed with status {:?}",
            args.join(" "),
            output.status.code()
        );
    }
    Ok(())
}

fn git_diff_paths(repo: &Path, base_ref: &str, head_ref: &str) -> Result<Vec<String>> {
    if base_ref.trim().is_empty() {
        anyhow::bail!("base ref must not be empty");
    }
    if head_ref.trim().is_empty() {
        anyhow::bail!("head ref must not be empty");
    }
    let output = ProcessCommand::new("git")
        .arg("-C")
        .arg(repo)
        .arg("diff")
        .arg("--name-only")
        .arg("--diff-filter=ACMRTUXB")
        .arg(base_ref)
        .arg(head_ref)
        .output()
        .with_context(|| {
            format!(
                "git diff --name-only {} {} in {}",
                base_ref,
                head_ref,
                repo.display()
            )
        })?;
    if !output.status.success() {
        anyhow::bail!("git diff failed with status {:?}", output.status.code());
    }
    let stdout = String::from_utf8(output.stdout).context("git diff utf8")?;
    parse_name_only_paths(&stdout)
}

fn gh_pr_diff_paths(repo: &Path, pr: &str, github_repo: Option<&str>) -> Result<Vec<String>> {
    if pr.trim().is_empty() {
        anyhow::bail!("PR identifier must not be empty");
    }
    let mut command = ProcessCommand::new("gh");
    command
        .current_dir(repo)
        .arg("pr")
        .arg("diff")
        .arg(pr)
        .arg("--name-only");
    if let Some(github_repo) = github_repo {
        if github_repo.trim().is_empty() {
            anyhow::bail!("--github-repo must not be empty");
        }
        command.arg("--repo").arg(github_repo);
    }
    let output = command
        .output()
        .with_context(|| format!("gh pr diff --name-only in {}", repo.display()))?;
    if !output.status.success() {
        anyhow::bail!("gh pr diff failed with status {:?}", output.status.code());
    }
    let stdout = String::from_utf8(output.stdout).context("gh pr diff utf8")?;
    parse_name_only_paths(&stdout)
}

fn parse_name_only_paths(stdout: &str) -> Result<Vec<String>> {
    let mut paths = Vec::new();
    for line in stdout.lines() {
        let path = line.trim();
        if path.is_empty() || path.starts_with(".helmbench/") {
            continue;
        }
        helmbench::validate_safe_relative_path_for_cli(path)?;
        paths.push(path.to_string());
    }
    paths.sort();
    paths.dedup();
    Ok(paths)
}

fn source_free_pr_label(pr: &str) -> String {
    let pr = pr.trim();
    if !pr.is_empty() && pr.chars().all(|ch| ch.is_ascii_digit()) {
        format!("pr:{pr}")
    } else {
        source_free_hash("pr-ref", pr)
    }
}

fn path_event(
    task_id: &str,
    event_kind: AgentEventKind,
    path: String,
    observed_at_millis: Option<u64>,
) -> Result<AgentEvent> {
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
        observed_at_millis,
        privacy: PrivacyStatus::source_free(),
    };
    validate_agent_event(&event)?;
    Ok(event)
}

fn render_adapter_command(template: &str, task_id: &str, repo: &Path, events: &Path) -> String {
    template
        .replace("{task_id}", task_id)
        .replace("{repo}", &shell_escape(&repo.to_string_lossy()))
        .replace("{events}", &shell_escape(&events.to_string_lossy()))
}

fn infer_command_class(command: &str) -> CommandClass {
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

fn command_hash(command: &str) -> String {
    source_free_hash("cmd", command)
}

fn source_free_hash(label: &str, value: &str) -> String {
    let mut hash = 0xcbf29ce484222325u64;
    for byte in value.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("{label}:{hash:016x}")
}

fn ctxhelm_config_hash(config: &CtxhelmRunConfig) -> String {
    source_free_hash(
        "ctxhelm",
        &format!(
            "bin={}|mode={}|target={}|semantic={}|provider={:?}|model={:?}|dimensions={:?}|pack={}|budget={}",
            config.ctxhelm_bin.display(),
            config.mode,
            config.target_agent,
            config.semantic,
            config.semantic_provider,
            config.semantic_model,
            config.semantic_dimensions,
            config.include_pack,
            config.pack_budget
        ),
    )
}

fn safe_task_dir_name(task_id: &str) -> String {
    let mut name = task_id
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.') {
                ch
            } else {
                '_'
            }
        })
        .collect::<String>();
    if name.is_empty() {
        name.push_str("task");
    }
    name
}

fn path_as_str(path: &Path) -> Result<&str> {
    path.to_str()
        .with_context(|| format!("path is not utf8: {}", path.display()))
}

fn shell_escape(value: &str) -> String {
    format!("'{}'", value.replace('\'', "'\\''"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn refresh_matrix_manifest_digests(matrix_dir: &Path) {
        let manifest_path = matrix_dir.join("matrix-manifest.json");
        let raw = std::fs::read_to_string(&manifest_path).expect("matrix manifest");
        let mut manifest =
            serde_json::from_str::<RunMatrixManifest>(&raw).expect("parse matrix manifest");
        manifest.artifact_digests =
            collect_matrix_artifact_digests(matrix_dir, &manifest).expect("matrix digests");
        write_json(&manifest, &manifest_path).expect("write matrix manifest");
    }

    #[test]
    fn claude_command_includes_source_free_event_instructions() {
        let command = claude_adapter_command(
            Path::new("/tmp/helmbench"),
            Path::new("claude"),
            Some("sonnet"),
            &["--debug".to_string()],
            true,
            true,
        );
        assert!(command.contains("claude"));
        assert!(command.contains("--print"));
        assert!(command.contains("--append-system-prompt"));
        assert!(command.contains("record-event"));
        assert!(command.contains("--dangerously-skip-permissions"));
        assert!(command.contains("--model 'sonnet'"));
        assert!(command.contains(">/dev/null 2>/dev/null"));
    }

    #[test]
    fn codex_command_uses_isolated_repo_and_workspace_sandbox_by_default() {
        let command = codex_adapter_command(
            Path::new("/tmp/helmbench"),
            Path::new("codex"),
            None,
            &[],
            false,
            true,
        );
        assert!(command.contains("'codex' exec"));
        assert!(command.contains("--cd \"$HELMBENCH_REPO\""));
        assert!(command.contains("--full-auto"));
        assert!(command.contains("record-event"));
        assert!(command.contains("HELMBENCH_TASK_PROMPT"));
        assert!(command.contains(">/dev/null 2>/dev/null"));
    }

    #[test]
    fn direct_agent_commands_can_leave_stdout_for_stream_capture() {
        let claude = claude_adapter_command(
            Path::new("/tmp/helmbench"),
            Path::new("claude"),
            None,
            &[],
            false,
            false,
        );
        assert!(!claude.contains(">/dev/null"));
        assert!(claude.contains("--output-format text"));

        let codex = codex_adapter_command(
            Path::new("/tmp/helmbench"),
            Path::new("codex"),
            None,
            &[],
            false,
            false,
        );
        assert!(!codex.contains(">/dev/null"));
        assert!(codex.contains("'codex' exec"));
    }

    #[test]
    fn init_demo_repo_creates_git_repo_and_valid_suite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let suite_path = temp.path().join("suite.json");

        init_demo_repo(&repo, &suite_path, false).expect("demo repo");

        assert!(repo.join(".git").exists());
        assert!(repo.join("scripts/demo-agent.sh").exists());
        let suite = load_suite(&suite_path).expect("suite");
        assert_eq!(suite.name, "demo-tiny-repo");
        assert_eq!(suite.tasks.len(), 2);
        assert_eq!(suite.tasks[0].expected_files, vec!["src/auth/session.txt"]);
        assert_eq!(
            suite.tasks[1].expected_tests,
            vec!["tests/billing/invoice.test.sh"]
        );
    }

    #[test]
    fn local_run_can_capture_structured_stdout_stream() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let suite_path = temp.path().join("suite.json");
        init_demo_repo(&repo, &suite_path, false).expect("demo repo");
        let suite = load_suite(&suite_path).expect("suite");
        let adapter = temp.path().join("stream-agent.sh");
        std::fs::write(&adapter, FAKE_STREAM_AGENT).expect("adapter");
        set_executable(&adapter).expect("chmod adapter");
        let out_dir = temp.path().join("traces");

        run_local_suite(
            &suite,
            &repo,
            &temp.path().join("workdirs"),
            &out_dir,
            "stream-agent",
            AgentVariant::Native,
            &[],
            None,
            Some(&format!("sh {}", shell_escape(&adapter.to_string_lossy()))),
            true,
            false,
        )
        .expect("local run");

        let traces = load_traces(&out_dir).expect("traces");
        assert_eq!(traces.len(), 2);
        assert!(traces.iter().all(|trace| !trace.files_read.is_empty()));
        assert!(traces.iter().all(|trace| trace.privacy.source_free));
        assert!(traces.iter().any(|trace| trace
            .files_read
            .iter()
            .any(|path| path.path == "src/auth/session.txt")));
    }

    #[test]
    fn local_run_infers_edits_against_post_setup_baseline() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo).expect("repo");
        write_demo_file(&repo, "app.txt", "fixed\n").expect("app");
        write_demo_file(&repo, "app.test", "expected validation target\n").expect("test");
        init_git_repo(&repo).expect("git repo");
        let suite = helmbench::TaskSuite {
            schema_version: helmbench::SUITE_SCHEMA_VERSION,
            name: "seeded-edit-suite".to_string(),
            description: "Seeded edit inference regression.".to_string(),
            tasks: vec![helmbench::BenchTask {
                id: "seeded-edit-001".to_string(),
                prompt: "Repair app.txt.".to_string(),
                expected_files: vec!["app.txt".to_string()],
                expected_tests: vec!["app.test".to_string()],
                success_command: Some("grep -q fixed app.txt".to_string()),
                setup_commands: vec!["printf 'broken\\n' > app.txt".to_string()],
                tags: Vec::new(),
                timeout_seconds: Some(60),
            }],
        };

        let baseline_out = temp.path().join("baseline-traces");
        run_local_suite(
            &suite,
            &repo,
            &temp.path().join("baseline-workdirs"),
            &baseline_out,
            "baseline",
            AgentVariant::Native,
            &[],
            None,
            None,
            false,
            false,
        )
        .expect("baseline local run");
        let baseline_traces = load_traces(&baseline_out).expect("baseline traces");
        assert_eq!(baseline_traces[0].status, TaskStatus::Failure);
        assert!(
            baseline_traces[0].files_edited.is_empty(),
            "setup changes should not be counted as agent edits"
        );

        let adapter = temp.path().join("repair-agent.sh");
        std::fs::write(
            &adapter,
            "#!/usr/bin/env sh\nset -eu\nprintf 'fixed\\n' > app.txt\n",
        )
        .expect("adapter");
        set_executable(&adapter).expect("chmod adapter");
        let repair_out = temp.path().join("repair-traces");
        run_local_suite(
            &suite,
            &repo,
            &temp.path().join("repair-workdirs"),
            &repair_out,
            "repair-agent",
            AgentVariant::Native,
            &[],
            None,
            Some(&format!("sh {}", shell_escape(&adapter.to_string_lossy()))),
            false,
            false,
        )
        .expect("repair local run");
        let repair_traces = load_traces(&repair_out).expect("repair traces");
        assert_eq!(repair_traces[0].status, TaskStatus::Success);
        assert!(repair_traces[0]
            .files_edited
            .iter()
            .any(|path| path.path == "app.txt"));
    }

    #[test]
    fn demo_pipeline_writes_full_artifact_set() {
        let temp = tempfile::tempdir().expect("tempdir");
        let out = temp.path().join("demo-run");
        let adapter = temp.path().join("fake-demo-agent.sh");
        std::fs::write(&adapter, FAKE_DEMO_AGENT).expect("adapter");
        set_executable(&adapter).expect("chmod adapter");

        run_demo_pipeline_with_adapter(
            &out,
            false,
            Some(format!("sh {}", shell_escape(&adapter.to_string_lossy()))),
        )
        .expect("demo pipeline");

        assert!(out.join("suite.json").exists());
        assert!(out.join("repo/.git").exists());
        assert!(out
            .join("traces/native/demo-auth-redirect-001.json")
            .exists());
        assert!(out
            .join("traces/guided/demo-auth-redirect-001.json")
            .exists());
        assert!(out.join("reports/native.json").exists());
        assert!(out.join("reports/guided.json").exists());
        assert!(out.join("reports/benchmark-summary.json").exists());
        assert!(out.join("reports/quality-gate.json").exists());
        assert!(out.join("reports/privacy-report.json").exists());
        assert!(out.join("docs/compare.md").exists());
        assert!(out.join("docs/benchmark-summary.md").exists());
        assert!(out.join("docs/quality-gate.md").exists());
        assert!(out.join("docs/privacy-report.md").exists());
        assert!(out.join("docs/native-autopsy.md").exists());
        assert!(out.join("docs/dashboard.html").exists());
        assert!(out.join("evidence/manifest.json").exists());
        verify_evidence_bundle(&out.join("evidence")).expect("verify evidence bundle");

        let gate = std::fs::read_to_string(out.join("reports/quality-gate.json")).expect("gate");
        let gate = serde_json::from_str::<serde_json::Value>(&gate).expect("json");
        assert_eq!(gate["passed"], true);
        let guided =
            std::fs::read_to_string(out.join("reports/guided.json")).expect("guided report");
        let guided = serde_json::from_str::<serde_json::Value>(&guided).expect("json");
        assert_eq!(guided["summary"]["successRate"], 1.0);
    }

    #[test]
    fn run_matrix_writes_publishable_artifacts() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let suite_path = temp.path().join("suite.json");
        init_demo_repo(&repo, &suite_path, false).expect("demo repo");

        let adapter = temp.path().join("fake-demo-agent.sh");
        std::fs::write(&adapter, FAKE_DEMO_AGENT).expect("adapter");
        set_executable(&adapter).expect("chmod adapter");
        let ctxhelm = temp.path().join("fake-ctxhelm.sh");
        std::fs::write(&ctxhelm, FAKE_CTXHELM).expect("ctxhelm");
        set_executable(&ctxhelm).expect("chmod ctxhelm");

        let out = temp.path().join("matrix");
        let adapter_command = format!("sh {}", shell_escape(&adapter.to_string_lossy()));
        let head = format!(
            "name=guided,agent=demo-guided,variant=ctxhelm_mcp,ctxhelm=true,ctxhelm_bin={},pack=true,pack_budget=brief,command={}",
            ctxhelm.to_string_lossy(),
            adapter_command,
        );
        let native_search_head = format!(
            "name=native-search,agent=demo-native-search,variant=native_search,command={}",
            adapter_command,
        );
        let request = build_run_matrix_request(
            None,
            Some(suite_path.clone()),
            Some(repo.clone()),
            Some(out.clone()),
            Some("name=native,agent=demo-baseline,variant=native".to_string()),
            vec![native_search_head, head],
            Vec::new(),
            false,
            false,
            true,
            1,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("matrix request");
        run_matrix(&request).expect("run matrix");

        assert!(out
            .join("traces/native/demo-auth-redirect-001.json")
            .exists());
        assert!(out
            .join("traces/guided/demo-auth-redirect-001.json")
            .exists());
        assert!(out
            .join("traces/native-search/demo-auth-redirect-001.json")
            .exists());
        assert!(out.join("reports/native.json").exists());
        assert!(out.join("reports/native-search.json").exists());
        assert!(out.join("reports/guided.json").exists());
        assert!(out.join("reports/compare-native-search.json").exists());
        assert!(out.join("reports/compare-guided.json").exists());
        assert!(out.join("reports/suite-health.json").exists());
        assert!(out.join("reports/benchmark-summary.json").exists());
        assert!(out.join("reports/quality-gate.json").exists());
        assert!(out.join("reports/privacy-report.json").exists());
        assert!(out.join("docs/compare-native-search.md").exists());
        assert!(out.join("docs/compare-guided.md").exists());
        assert!(out.join("docs/benchmark-summary.md").exists());
        assert!(out.join("docs/privacy-report.md").exists());
        assert!(out.join("docs/native-autopsy.md").exists());
        assert!(out.join("docs/native-search-autopsy.md").exists());
        assert!(out.join("docs/guided-autopsy.md").exists());
        assert!(out.join("docs/reproduction.md").exists());
        assert!(out.join("docs/dashboard.html").exists());
        assert!(out.join("evidence/health.json").exists());
        assert!(out.join("evidence/manifest.json").exists());
        assert!(out.join("matrix-manifest.json").exists());
        verify_evidence_bundle(&out.join("evidence")).expect("verify evidence");

        let summary =
            std::fs::read_to_string(out.join("reports/benchmark-summary.json")).expect("summary");
        let summary = serde_json::from_str::<serde_json::Value>(&summary).expect("json");
        assert_eq!(summary["runs"][1]["variant"], "native_search");
        assert_eq!(summary["comparisons"][0]["headVariant"], "native_search");
        assert_eq!(summary["comparisons"][1]["successRateDelta"], 1.0);
        let gate = std::fs::read_to_string(out.join("reports/quality-gate.json")).expect("gate");
        let gate = serde_json::from_str::<serde_json::Value>(&gate).expect("json");
        assert_eq!(gate["passed"], true);
        let privacy = std::fs::read_to_string(out.join("reports/privacy-report.json"))
            .expect("privacy report");
        let privacy = serde_json::from_str::<serde_json::Value>(&privacy).expect("json");
        assert_eq!(privacy["schemaVersion"], 1);
        assert_eq!(privacy["runCount"], 3);
        assert_eq!(privacy["traceCount"], 6);
        assert_eq!(privacy["privacy"]["sourceFree"], true);
        assert!(privacy["forbiddenRawData"]
            .as_array()
            .expect("forbidden raw data")
            .iter()
            .any(|item| item == "raw source"));
        assert!(privacy["runs"]
            .as_array()
            .expect("privacy runs")
            .iter()
            .all(|run| {
                run["reportSourceFree"] == true
                    && run["sourceFreeTraceCount"] == run["traceCount"]
                    && run["rawSourceLogged"] == false
                    && run["rawPromptLogged"] == false
                    && run["rawTranscriptLogged"] == false
                    && run["rawTerminalLogged"] == false
            }));
        let health =
            std::fs::read_to_string(out.join("reports/suite-health.json")).expect("health");
        let health = serde_json::from_str::<serde_json::Value>(&health).expect("json");
        assert_eq!(health["ok"], true);
        assert_eq!(health["privacy"]["sourceFree"], true);
        assert_eq!(health["evidenceUse"], "navigation_only");

        let traces = load_traces(&out.join("traces/guided")).expect("guided traces");
        assert!(traces.iter().all(|trace| trace.token_estimate == Some(321)));
        assert!(traces
            .iter()
            .all(|trace| trace.commands.iter().any(|command| {
                command.command_hash == Some(command_hash("ctxhelm prepare-task"))
            })));
        assert!(traces
            .iter()
            .all(|trace| trace.commands.iter().any(|command| {
                command.command_hash == Some(command_hash("ctxhelm get-pack brief"))
            })));

        let manifest = std::fs::read_to_string(out.join("matrix-manifest.json")).expect("manifest");
        let manifest = serde_json::from_str::<serde_json::Value>(&manifest).expect("json");
        assert_eq!(
            manifest["schemaVersion"],
            serde_json::json!(RUN_MATRIX_MANIFEST_SCHEMA_VERSION)
        );
        assert_eq!(manifest["suiteEvidenceUse"], "navigation_only");
        assert_eq!(
            manifest["provenance"]["helmbenchVersion"],
            serde_json::json!(env!("CARGO_PKG_VERSION"))
        );
        assert!(manifest["provenance"]["suiteHash"]
            .as_str()
            .expect("suite hash")
            .starts_with("suite:"));
        assert!(
            manifest["provenance"]["repoHead"]
                .as_str()
                .expect("repo head")
                .len()
                >= 40
        );
        assert_eq!(manifest["provenance"]["repoDirty"], false);
        assert_eq!(
            manifest["provenance"]["setupCommandCount"],
            serde_json::json!(0)
        );
        assert_eq!(manifest["baseline"]["name"], "native");
        assert_eq!(
            manifest["baseline"]["autopsyMarkdown"],
            "docs/native-autopsy.md"
        );
        assert_eq!(
            manifest["baseline"]["comparisonJson"],
            serde_json::Value::Null
        );
        assert_eq!(
            manifest["baseline"]["comparisonMarkdown"],
            serde_json::Value::Null
        );
        assert_eq!(
            manifest["baseline"]["adapterCommandHash"],
            serde_json::Value::Null
        );
        assert_eq!(manifest["heads"][0]["name"], "native-search");
        assert_eq!(manifest["heads"][0]["variant"], "native_search");
        assert_eq!(
            manifest["heads"][0]["autopsyMarkdown"],
            "docs/native-search-autopsy.md"
        );
        assert_eq!(
            manifest["heads"][0]["comparisonJson"],
            "reports/compare-native-search.json"
        );
        assert_eq!(
            manifest["heads"][0]["comparisonMarkdown"],
            "docs/compare-native-search.md"
        );
        assert_eq!(manifest["heads"][0]["ctxhelmEnabled"], false);
        assert_eq!(manifest["heads"][0]["packEnabled"], false);
        assert_eq!(
            manifest["heads"][0]["adapterPreset"],
            serde_json::Value::Null
        );
        assert_eq!(
            manifest["heads"][0]["adapterCommandHash"],
            serde_json::json!(command_hash(&adapter_command))
        );
        assert_eq!(manifest["heads"][1]["name"], "guided");
        assert_eq!(
            manifest["heads"][1]["autopsyMarkdown"],
            "docs/guided-autopsy.md"
        );
        assert_eq!(
            manifest["heads"][1]["comparisonJson"],
            "reports/compare-guided.json"
        );
        assert_eq!(
            manifest["heads"][1]["comparisonMarkdown"],
            "docs/compare-guided.md"
        );
        assert_eq!(manifest["heads"][1]["ctxhelmEnabled"], true);
        assert_eq!(manifest["heads"][1]["packEnabled"], true);
        assert_eq!(
            manifest["heads"][1]["adapterPreset"],
            serde_json::Value::Null
        );
        assert_eq!(
            manifest["heads"][1]["adapterCommandHash"],
            serde_json::json!(command_hash(&adapter_command))
        );
        assert!(manifest["heads"][1]["ctxhelmConfigHash"]
            .as_str()
            .expect("ctxhelm hash")
            .starts_with("ctxhelm:"));
        assert_eq!(manifest["qualityGatePassed"], true);
        assert_eq!(manifest["evidenceBundleVerified"], true);
        assert_eq!(manifest["privacy"]["sourceFree"], true);
        assert_eq!(
            manifest["artifacts"]["suiteHealthJson"],
            "reports/suite-health.json"
        );
        assert_eq!(
            manifest["artifacts"]["benchmarkSummaryJson"],
            "reports/benchmark-summary.json"
        );
        assert_eq!(
            manifest["artifacts"]["baselineAutopsyMarkdown"],
            "docs/native-autopsy.md"
        );
        assert_eq!(
            manifest["artifacts"]["privacyReportJson"],
            "reports/privacy-report.json"
        );
        assert_eq!(
            manifest["artifacts"]["privacyReportMarkdown"],
            "docs/privacy-report.md"
        );
        assert_eq!(
            manifest["artifacts"]["reproductionMarkdown"],
            "docs/reproduction.md"
        );
        assert_eq!(
            manifest["artifacts"]["evidenceManifest"],
            "evidence/manifest.json"
        );
        let artifact_digests = manifest["artifactDigests"]
            .as_array()
            .expect("artifact digests");
        assert!(artifact_digests.iter().any(|digest| {
            digest["path"] == "reports/guided.json"
                && digest["contentHash"]
                    .as_str()
                    .is_some_and(|hash| hash.starts_with("fnv64:"))
        }));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "reports/native-search.json"));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "reports/compare-native-search.json"));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "reports/compare-guided.json"));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "reports/privacy-report.json"));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "docs/privacy-report.md"));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "docs/compare-guided.md"));
        assert!(artifact_digests
            .iter()
            .any(|digest| { digest["path"] == "traces/guided/demo-auth-redirect-001.json" }));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "docs/guided-autopsy.md"));
        assert!(artifact_digests
            .iter()
            .any(|digest| digest["path"] == "docs/reproduction.md"));
        let reproduction =
            std::fs::read_to_string(out.join("docs/reproduction.md")).expect("reproduction");
        assert!(reproduction.contains("helmbench verify-matrix --matrix <matrix-dir>"));
        assert!(reproduction.contains("Suite hash"));
        assert!(reproduction.contains("docs/native-search-autopsy.md"));
        assert!(reproduction.contains("reports/compare-native-search.json"));
        assert!(reproduction.contains("docs/guided-autopsy.md"));
        assert!(reproduction.contains("reports/compare-guided.json"));
        assert!(reproduction.contains("docs/compare-guided.md"));
        assert!(reproduction.contains("reports/privacy-report.json"));
        assert!(reproduction.contains("docs/privacy-report.md"));
        assert!(reproduction.contains(&command_hash(&adapter_command)));
        assert!(!reproduction.contains(adapter.to_string_lossy().as_ref()));

        let verified = verify_run_matrix(&out).expect("verify matrix");
        assert_eq!(verified.heads.len(), 2);
        assert_eq!(
            verified.suite_evidence_use,
            SuiteEvidenceUse::NavigationOnly
        );
        assert!(verified.evidence_bundle_verified);
        let manifest_path = out.join("matrix-manifest.json");
        let manifest_raw = std::fs::read_to_string(&manifest_path).expect("manifest raw");
        let mut mismatched_manifest: RunMatrixManifest =
            serde_json::from_str(&manifest_raw).expect("manifest struct");
        mismatched_manifest.suite_evidence_use = SuiteEvidenceUse::OutcomeReady;
        write_json(&mismatched_manifest, &manifest_path).expect("write mismatched manifest");
        refresh_matrix_manifest_digests(&out);
        let error = verify_run_matrix(&out).expect_err("reject mismatched evidence use");
        assert!(
            error
                .to_string()
                .contains("suiteEvidenceUse `outcome_ready`"),
            "{error}"
        );
        std::fs::write(&manifest_path, manifest_raw).expect("restore manifest");
        refresh_matrix_manifest_digests(&out);
        let first_summary_path = out.join("reports/benchmark-summary.json");
        let original_summary =
            read_benchmark_summary(&first_summary_path).expect("original benchmark summary");
        let mut mismatched_summary = original_summary.clone();
        mismatched_summary.runs[1].agent = "wrong-agent".to_string();
        write_json(&mismatched_summary, &first_summary_path).expect("mismatched summary");
        refresh_matrix_manifest_digests(&out);
        let error = verify_run_matrix(&out).expect_err("reject mismatched benchmark summary");
        assert!(error.to_string().contains("matrix run `1` `native-search`"));
        write_json(&original_summary, &first_summary_path).expect("restore benchmark summary");
        refresh_matrix_manifest_digests(&out);

        let mut first_summary =
            read_benchmark_summary(&first_summary_path).expect("first benchmark summary");
        first_summary.runs[1].average_time_to_first_relevant_file_millis = Some(150.0);
        first_summary.runs[1].tool_calls_per_success = Some(4.0);
        first_summary.runs[1].token_estimate_per_success = Some(900.0);
        write_json(&first_summary, &first_summary_path).expect("mutated first summary");
        refresh_matrix_manifest_digests(&out);

        let out2 = temp.path().join("matrix-second");
        let mut request2 = request.clone();
        request2.out_dir = out2.clone();
        run_matrix(&request2).expect("second matrix");
        let summary_path = out2.join("reports/benchmark-summary.json");
        let mut second_summary =
            read_benchmark_summary(&summary_path).expect("second benchmark summary");
        second_summary.runs[1].success_rate = 0.5;
        second_summary.runs[1].validation_coverage_rate = 0.5;
        second_summary.runs[1].irrelevant_read_rate = 0.25;
        second_summary.runs[1].recommendation_recall = 0.5;
        second_summary.runs[1].context_precision = 0.5;
        second_summary.runs[1].edited_file_recall = 0.5;
        second_summary.runs[1].average_time_to_first_relevant_file_millis = Some(75.0);
        second_summary.runs[1].total_tool_calls += 3;
        second_summary.runs[1].total_token_estimate += 100;
        second_summary.runs[1].tool_calls_per_success = Some(6.5);
        second_summary.runs[1].token_estimate_per_success = Some(1050.0);
        write_json(&second_summary, &summary_path).expect("mutated second summary");
        refresh_matrix_manifest_digests(&out2);

        let history =
            build_matrix_history_report(&[out.clone(), out2.clone()]).expect("matrix history");
        assert_eq!(history.matrices.len(), 2);
        assert_eq!(history.trends.len(), 3);
        assert_eq!(history.trends[1].name, "native-search");
        assert!(history.trends[1].success_rate_delta < 0.0);
        assert!(history.trends[1].irrelevant_read_rate_delta > 0.0);
        assert_eq!(
            history.trends[1].average_time_to_first_relevant_file_millis_delta,
            Some(-75.0)
        );
        assert_eq!(history.trends[1].total_tool_calls_delta, 3);
        assert_eq!(history.trends[1].tool_calls_per_success_delta, Some(2.5));
        assert_eq!(
            history.trends[1].token_estimate_per_success_delta,
            Some(150.0)
        );
        assert!(history.privacy.source_free);
        let rendered = render_markdown_matrix_history(&history);
        assert!(rendered.contains("First-To-Last Trends"));
        assert!(rendered.contains("`guided`"));
        assert!(rendered.contains("-75 ms"));
        assert!(rendered.contains("Tools/success"));
        assert!(!rendered.contains(temp.path().to_string_lossy().as_ref()));
        let html = render_html_matrix_history(&history);
        assert!(html.contains("<title>HelmBench Matrix History</title>"));
        assert!(html.contains("Matrix History"));
        assert!(html.contains("Source-free"));
        assert!(html.contains("guided"));
        assert!(html.contains("-75 ms"));
        assert!(html.contains("Tools/success"));
        assert!(!html.contains(temp.path().to_string_lossy().as_ref()));

        std::fs::write(out.join("docs/guided-autopsy.md"), "tampered").expect("tamper autopsy");
        let err = verify_run_matrix(&out).expect_err("tampered matrix should fail");
        assert!(
            err.to_string().contains("matrix artifact digest mismatch"),
            "{err}"
        );

        let mut tampered = verified;
        tampered.artifacts.dashboard_html = "../dashboard.html".to_string();
        write_json(&tampered, &out.join("matrix-manifest.json")).expect("tampered manifest");
        let err = verify_run_matrix(&out).expect_err("unsafe manifest should fail");
        assert!(err.to_string().contains("validate matrix path"), "{err}");
    }

    #[test]
    fn run_matrix_config_file_builds_request() {
        let temp = tempfile::tempdir().expect("tempdir");
        let config_path = temp.path().join("matrix.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "suite": "suite.json",
                "repo": "repo",
                "outDir": "matrix-out",
                "setupCommands": ["printf setup >/dev/null"],
                "failOnRegression": true,
                "healthMinCommits": 2,
                "allowDirtyHealth": true,
                "healthCheckSuccessCommands": true,
                "healthFailFastSuccessCommands": true,
                "healthRequireSetupCommands": true,
                "qualityGate": {
                    "minTaskCount": 10,
                    "maxAverageTimeToFirstRelevantFileMillisDelta": 0.0,
                    "maxTotalToolCallsDelta": 0,
                    "maxTotalTokenEstimateDelta": 0,
                    "maxToolCallsPerSuccessDelta": 0.0,
                    "maxTokenEstimatePerSuccessDelta": 0.0
                },
                "baseline": {
                    "name": "native",
                    "agent": "demo-baseline",
                    "variant": "native"
                },
                "heads": [
                    {
                        "name": "ctxhelm",
                        "agent": "demo-guided",
                        "variant": "ctxhelm_mcp",
                        "ctxhelm": true,
                        "ctxhelmBin": "fake-ctxhelm",
                        "pack": true,
                        "packBudget": "brief",
                        "preset": "claude-code",
                        "bin": "fake-agent.sh",
                        "model": "sonnet",
                        "args": ["--debug"],
                        "dangerouslySkipPermissions": true
                    }
                ]
            })
            .to_string(),
        )
        .expect("config");

        let request = build_run_matrix_request(
            Some(&config_path),
            None,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            true,
            false,
            false,
            1,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("request");

        assert_eq!(request.suite_path, PathBuf::from("suite.json"));
        assert_eq!(request.repo, PathBuf::from("repo"));
        assert_eq!(request.out_dir, PathBuf::from("matrix-out"));
        assert!(request.force);
        assert!(request.fail_on_regression);
        assert_eq!(request.quality_gate_config.min_task_count, Some(10));
        assert_eq!(
            request
                .quality_gate_config
                .max_average_time_to_first_relevant_file_millis_delta,
            Some(0.0)
        );
        assert_eq!(
            request.quality_gate_config.max_total_tool_calls_delta,
            Some(0)
        );
        assert_eq!(
            request.quality_gate_config.max_total_token_estimate_delta,
            Some(0)
        );
        assert_eq!(
            request.quality_gate_config.max_tool_calls_per_success_delta,
            Some(0.0)
        );
        assert_eq!(
            request
                .quality_gate_config
                .max_token_estimate_per_success_delta,
            Some(0.0)
        );
        assert_eq!(request.health_min_commits, 2);
        assert!(request.allow_dirty_health);
        assert!(request.health_check_success_commands);
        assert!(request.health_fail_fast_success_commands);
        assert!(request.health_require_setup_commands);
        assert_eq!(request.setup_commands, vec!["printf setup >/dev/null"]);
        assert_eq!(request.baseline.safe_name, "native");
        assert_eq!(request.heads[0].safe_name, "ctxhelm");
        assert_eq!(request.heads[0].variant, AgentVariant::CtxhelmMcp);
        assert_eq!(
            request.heads[0]
                .ctxhelm
                .as_ref()
                .expect("ctxhelm")
                .ctxhelm_bin,
            PathBuf::from("fake-ctxhelm")
        );
        assert!(
            request.heads[0]
                .ctxhelm
                .as_ref()
                .expect("ctxhelm")
                .include_pack
        );
        assert_eq!(
            request.heads[0]
                .adapter_preset
                .as_ref()
                .expect("preset")
                .preset,
            AdapterPreset::ClaudeCode
        );
        assert_eq!(
            request.heads[0]
                .adapter_preset
                .as_ref()
                .expect("preset")
                .bin,
            PathBuf::from("fake-agent.sh")
        );
        assert!(request.heads[0]
            .adapter_command
            .as_ref()
            .expect("adapter command")
            .contains("--model 'sonnet'"));
        assert!(request.heads[0]
            .adapter_command
            .as_ref()
            .expect("adapter command")
            .contains("--dangerously-skip-permissions"));

        let override_request = build_run_matrix_request(
            Some(&config_path),
            Some(PathBuf::from("suite-override.json")),
            Some(PathBuf::from("repo-override")),
            Some(PathBuf::from("out-override")),
            Some("name=base2,agent=agent2,variant=other".to_string()),
            vec!["name=head2,agent=agent2,variant=other".to_string()],
            vec!["printf cli >/dev/null".to_string()],
            false,
            true,
            true,
            3,
            false,
            true,
            true,
            true,
            Some(20),
            Some(-10.0),
            Some(5),
            Some(100),
            Some(1.5),
            Some(250.0),
        )
        .expect("override request");
        assert_eq!(
            override_request.suite_path,
            PathBuf::from("suite-override.json")
        );
        assert_eq!(override_request.repo, PathBuf::from("repo-override"));
        assert_eq!(override_request.out_dir, PathBuf::from("out-override"));
        assert_eq!(override_request.baseline.safe_name, "base2");
        assert_eq!(override_request.heads[0].safe_name, "head2");
        assert_eq!(
            override_request.setup_commands,
            vec!["printf setup >/dev/null", "printf cli >/dev/null"]
        );
        assert!(override_request.keep_workdirs);
        assert!(override_request.fail_on_regression);
        assert_eq!(
            override_request.quality_gate_config.min_task_count,
            Some(20)
        );
        assert_eq!(
            override_request
                .quality_gate_config
                .max_average_time_to_first_relevant_file_millis_delta,
            Some(-10.0)
        );
        assert_eq!(
            override_request
                .quality_gate_config
                .max_total_tool_calls_delta,
            Some(5)
        );
        assert_eq!(
            override_request
                .quality_gate_config
                .max_total_token_estimate_delta,
            Some(100)
        );
        assert_eq!(
            override_request
                .quality_gate_config
                .max_tool_calls_per_success_delta,
            Some(1.5)
        );
        assert_eq!(
            override_request
                .quality_gate_config
                .max_token_estimate_per_success_delta,
            Some(250.0)
        );
        assert_eq!(override_request.health_min_commits, 3);
        assert!(override_request.allow_dirty_health);
        assert!(override_request.health_check_success_commands);
        assert!(override_request.health_fail_fast_success_commands);
        assert!(override_request.health_require_setup_commands);
    }

    #[test]
    fn run_matrix_spec_parser_builds_adapter_preset_command() {
        let spec = parse_run_matrix_spec(
            "name=claude-guided,agent=claude-code,variant=ctxhelm_mcp,preset=claude-code,bin=fake-claude,model=sonnet,arg=--debug,dangerously_skip_permissions=true,capture_stream=false",
        )
        .expect("parse spec");

        assert_eq!(spec.safe_name, "claude-guided");
        assert_eq!(spec.variant, AgentVariant::CtxhelmMcp);
        assert_eq!(
            spec.adapter_preset.as_ref().expect("preset").preset,
            AdapterPreset::ClaudeCode
        );
        assert_eq!(
            spec.adapter_preset.as_ref().expect("preset").bin,
            PathBuf::from("fake-claude")
        );
        let command = spec.adapter_command.as_ref().expect("adapter command");
        assert!(command.contains("'fake-claude'"));
        assert!(command.contains("--model 'sonnet'"));
        assert!(command.contains("'--debug'"));
        assert!(command.contains("--dangerously-skip-permissions"));
        assert!(command.contains("record-event"));
        assert!(command.contains(">/dev/null 2>/dev/null"));

        let err = parse_run_matrix_spec(
            "name=bad,agent=claude-code,variant=native,preset=claude-code,command=claude --print",
        )
        .expect_err("reject command plus preset");
        assert!(err
            .to_string()
            .contains("cannot combine command and preset"));
    }

    #[test]
    fn run_matrix_variant_parser_accepts_native_search_aliases() {
        assert_eq!(
            parse_agent_variant("native_search").expect("underscore"),
            AgentVariant::NativeSearch
        );
        assert_eq!(
            parse_agent_variant("native-search").expect("hyphen"),
            AgentVariant::NativeSearch
        );
    }

    #[test]
    fn schema_contracts_are_valid_json() {
        let contracts = [
            (SchemaKind::TaskSuite, "HelmBench Task Suite"),
            (SchemaKind::AgentTrace, "HelmBench Agent Trace"),
            (SchemaKind::AgentEvent, "HelmBench Agent Event"),
            (SchemaKind::RunReport, "HelmBench Run Report"),
            (SchemaKind::CompareReport, "HelmBench Compare Report"),
            (SchemaKind::BenchmarkSummary, "HelmBench Benchmark Summary"),
            (SchemaKind::LaunchReadiness, "HelmBench Launch Readiness"),
            (SchemaKind::QualityGate, "HelmBench Quality Gate"),
            (SchemaKind::RunMatrixConfig, "HelmBench Run Matrix Config"),
            (SchemaKind::MatrixHistory, "HelmBench Matrix History"),
            (SchemaKind::DoctorReport, "HelmBench Doctor Report"),
            (SchemaKind::Autopsy, "HelmBench Autopsy"),
            (SchemaKind::DiffAutopsy, "HelmBench Diff Autopsy"),
            (SchemaKind::SuiteHealth, "HelmBench Suite Health"),
            (SchemaKind::EvidenceBundle, "HelmBench Evidence Bundle"),
            (
                SchemaKind::RunMatrixManifest,
                "HelmBench Run Matrix Manifest",
            ),
            (
                SchemaKind::RunMatrixPrivacyReport,
                "HelmBench Run Matrix Privacy Report",
            ),
        ];

        for (kind, title) in contracts {
            let raw = schema_contract(kind);
            let value = serde_json::from_str::<serde_json::Value>(raw).expect("schema json");
            assert_eq!(
                value["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
            assert_eq!(value["title"], title);
            assert!(value["$id"]
                .as_str()
                .expect("schema id")
                .starts_with("https://github.com/thromel/helmbench/schemas/"));
            if matches!(kind, SchemaKind::RunReport | SchemaKind::CompareReport) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    helmbench::REPORT_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::BenchmarkSummary) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    helmbench::BENCHMARK_SUMMARY_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::LaunchReadiness) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    LAUNCH_READINESS_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::QualityGate) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    helmbench::QUALITY_GATE_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::MatrixHistory) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    MATRIX_HISTORY_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::DoctorReport) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    DOCTOR_REPORT_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::Autopsy) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    helmbench::AUTOPSY_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::DiffAutopsy) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    helmbench::DIFF_AUTOPSY_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::SuiteHealth) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    SUITE_HEALTH_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::EvidenceBundle) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    EVIDENCE_BUNDLE_SCHEMA_VERSION
                );
            }
            if matches!(kind, SchemaKind::RunMatrixManifest) {
                assert_eq!(
                    value["properties"]["schemaVersion"]["const"],
                    RUN_MATRIX_MANIFEST_SCHEMA_VERSION
                );
            }
        }
    }

    #[test]
    fn all_schema_contracts_write_checked_in_filenames() {
        let temp = tempfile::tempdir().expect("tempdir");

        write_all_schema_contracts(temp.path()).expect("write all schemas");

        let actual_count = std::fs::read_dir(temp.path())
            .expect("read schema dir")
            .count();
        assert_eq!(actual_count, all_schema_kinds().len());

        for kind in all_schema_kinds() {
            let path = temp.path().join(schema_contract_filename(*kind));
            assert!(path.exists(), "missing {}", path.display());
            let raw = std::fs::read_to_string(path).expect("schema file");
            let value = serde_json::from_str::<serde_json::Value>(&raw).expect("schema json");
            assert_eq!(
                value["$schema"],
                "https://json-schema.org/draft/2020-12/schema"
            );
        }
    }

    #[test]
    fn validate_matrix_request_accepts_demo_repo() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let suite_path = temp.path().join("suite.json");
        init_demo_repo(&repo, &suite_path, false).expect("demo repo");
        let config_path = temp.path().join("matrix.json");
        std::fs::write(
            &config_path,
            serde_json::json!({
                "suite": suite_path,
                "repo": repo,
                "baseline": {
                    "name": "native",
                    "agent": "demo-baseline",
                    "variant": "native"
                },
                "heads": [
                    {
                        "name": "guided",
                        "agent": "demo-guided",
                        "variant": "ctxhelm_mcp"
                    }
                ]
            })
            .to_string(),
        )
        .expect("config");

        let request = build_run_matrix_request(
            Some(&config_path),
            None,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            false,
            false,
            false,
            1,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("request");
        let suite = validate_run_matrix_request(&request).expect("valid matrix");
        assert_eq!(suite.name, "demo-tiny-repo");
        assert_eq!(request.heads.len(), 1);
    }

    #[test]
    fn init_public_matrix_config_writes_valid_agent_matrix_request() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::RefactoringMiner, &repo)
            .expect("fixture repo");
        let suite_path = temp.path().join("refactoring-miner-public.json");
        write_json(&refactoring_miner_suite(), &suite_path).expect("suite");
        let config_path = temp.path().join("refactoring-miner-matrix.json");
        let health_path = temp.path().join("refactoring-miner-matrix-health.json");
        let matrix_out = temp.path().join("matrix-out");

        init_public_matrix_config(InitPublicMatrixOptions {
            preset: PublicSuitePreset::RefactoringMiner,
            repo: repo.clone(),
            suite_path: Some(suite_path.clone()),
            out: config_path.clone(),
            out_dir: Some(matrix_out.clone()),
            health_out: Some(health_path.clone()),
            agent_preset: AdapterPreset::ClaudeCode,
            agent_bin: Some(PathBuf::from("fake-claude")),
            model: Some("sonnet".to_string()),
            agent_args: vec!["--debug".to_string()],
            dangerously_skip_permissions: true,
            dangerously_bypass_approvals_and_sandbox: false,
            ctxhelm_bin: PathBuf::from("fake-ctxhelm"),
            mode: "bug-fix".to_string(),
            target_agent: None,
            pack: true,
            fail_on_regression: false,
            health_min_commits: 1,
            allow_dirty_health: false,
            health_check_success_commands: false,
            health_fail_fast_success_commands: false,
            health_require_setup_commands: false,
            force: false,
        })
        .expect("matrix config");

        let raw = std::fs::read_to_string(&config_path).expect("config raw");
        let health_raw = std::fs::read_to_string(&health_path).expect("health raw");
        assert!(!raw.contains("null"));
        assert!(raw.contains("\"preset\": \"claude-code\""));
        assert!(health_raw.contains("\"suiteName\": \"refactoringminer-public\""));
        assert!(health_raw.contains("\"ok\": true"));
        let request = build_run_matrix_request(
            Some(&config_path),
            None,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            false,
            false,
            false,
            1,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("request");
        let suite = validate_run_matrix_request(&request).expect("valid matrix");
        assert_eq!(suite.name, "refactoringminer-public");
        assert_eq!(suite.tasks.len(), 10);
        assert_eq!(request.repo, repo);
        assert_eq!(request.out_dir, matrix_out);
        assert_eq!(request.quality_gate_config.min_task_count, Some(10));
        assert_eq!(request.health_min_commits, 1);
        assert_eq!(request.baseline.safe_name, "native");
        assert_eq!(request.baseline.variant, AgentVariant::Native);
        assert_eq!(
            request
                .baseline
                .adapter_preset
                .as_ref()
                .expect("preset")
                .preset,
            AdapterPreset::ClaudeCode
        );
        assert_eq!(request.heads[0].safe_name, "ctxhelm");
        assert_eq!(request.heads[0].variant, AgentVariant::CtxhelmMcp);
        assert!(request.heads[0].ctxhelm.is_some());
        assert!(
            request.heads[0]
                .ctxhelm
                .as_ref()
                .expect("ctxhelm")
                .include_pack
        );
        assert!(request.heads[0]
            .adapter_command
            .as_ref()
            .expect("command")
            .contains("'fake-claude'"));
    }

    #[test]
    fn init_public_matrix_config_can_require_outcome_health() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::RefactoringMiner, &repo)
            .expect("fixture repo");
        let mut suite = refactoring_miner_suite();
        for task in &mut suite.tasks {
            task.setup_commands = vec!["true".to_string()];
            task.success_command = Some("test -f .helmbench/agent-created-sentinel".to_string());
        }
        let suite_path = temp.path().join("refactoring-miner-public.json");
        write_json(&suite, &suite_path).expect("suite");
        let config_path = temp.path().join("refactoring-miner-matrix.json");

        init_public_matrix_config(InitPublicMatrixOptions {
            preset: PublicSuitePreset::RefactoringMiner,
            repo,
            suite_path: Some(suite_path),
            out: config_path.clone(),
            out_dir: None,
            health_out: None,
            agent_preset: AdapterPreset::ClaudeCode,
            agent_bin: Some(PathBuf::from("fake-claude")),
            model: None,
            agent_args: Vec::new(),
            dangerously_skip_permissions: true,
            dangerously_bypass_approvals_and_sandbox: false,
            ctxhelm_bin: PathBuf::from("fake-ctxhelm"),
            mode: "bug-fix".to_string(),
            target_agent: None,
            pack: false,
            fail_on_regression: true,
            health_min_commits: 1,
            allow_dirty_health: false,
            health_check_success_commands: true,
            health_fail_fast_success_commands: true,
            health_require_setup_commands: true,
            force: false,
        })
        .expect("matrix config");

        let raw = std::fs::read_to_string(&config_path).expect("config raw");
        assert!(raw.contains("\"healthCheckSuccessCommands\": true"));
        assert!(raw.contains("\"healthFailFastSuccessCommands\": true"));
        assert!(raw.contains("\"healthRequireSetupCommands\": true"));
        let request = build_run_matrix_request(
            Some(&config_path),
            None,
            None,
            None,
            None,
            Vec::new(),
            Vec::new(),
            false,
            false,
            false,
            1,
            false,
            false,
            false,
            false,
            None,
            None,
            None,
            None,
            None,
            None,
        )
        .expect("request");
        assert!(request.health_check_success_commands);
        assert!(request.health_fail_fast_success_commands);
        assert!(request.health_require_setup_commands);
    }

    #[test]
    fn refactoring_miner_public_suite_is_source_free_and_valid() {
        let suite = refactoring_miner_suite();
        validate_suite(&suite).expect("suite");

        assert_eq!(suite.name, "refactoringminer-public");
        assert_eq!(suite.tasks.len(), 10);
        assert!(suite
            .tasks
            .iter()
            .all(|task| task.tags.contains(&"public_repo".to_string())));
        assert!(suite.tasks.iter().any(|task| task.expected_files.contains(
            &"src/main/java/org/refactoringminer/mcp/McpIntentValidator.java".to_string()
        )));
        assert!(suite.tasks.iter().any(|task| task.expected_files.contains(
            &"src/main/java/org/refactoringminer/astDiff/matchers/TreeMatcher.java".to_string()
        )));
        assert!(suite.tasks.iter().any(|task| task
            .expected_tests
            .contains(&"src/test/java/gui/MarkAsViewedTest.java".to_string())));
        assert!(suite.tasks.iter().any(|task| task.expected_tests.contains(
            &"src/test/java/org/refactoringminer/test/TestCommandLine.java".to_string()
        )));
    }

    #[test]
    fn public_suite_health_accepts_clean_fixture() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::RefactoringMiner, &repo)
            .expect("fixture repo");
        let suite = refactoring_miner_suite();

        let health = public_suite_health(PublicSuitePreset::RefactoringMiner, &repo, 1, &suite)
            .expect("health");

        assert!(health.ok);
        assert_eq!(health.repo_name, "repo");
        assert_eq!(health.commit_count, Some(1));
        assert!(!health.dirty);
        assert!(health.fsck_ok);
        assert!(health.missing_files.is_empty());
        assert!(health.head.is_some());
    }

    #[test]
    fn public_suite_health_rejects_dirty_fixture_without_source_logs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::RefactoringMiner, &repo)
            .expect("fixture repo");
        std::fs::write(repo.join("UNTRACKED.md"), "dirty").expect("dirty file");
        let suite = refactoring_miner_suite();

        let health = public_suite_health(PublicSuitePreset::RefactoringMiner, &repo, 1, &suite)
            .expect("health");

        assert!(!health.ok);
        assert!(health.dirty);
        assert!(health.missing_files.is_empty());
        assert!(health
            .checked_files
            .iter()
            .all(|path| !path.starts_with('/')));
    }

    #[test]
    fn flask_public_suite_uses_python_paths_and_health_anchors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::Flask, &repo).expect("fixture repo");
        let suite = flask_suite();

        validate_suite(&suite).expect("suite");
        assert_eq!(suite.name, "flask-public");
        assert_eq!(suite.tasks.len(), 4);
        assert!(suite
            .tasks
            .iter()
            .all(|task| task.tags.contains(&"python".to_string())));
        assert!(suite.tasks.iter().any(|task| task
            .expected_files
            .contains(&"src/flask/config.py".to_string())));
        assert!(suite.tasks.iter().any(|task| task
            .expected_tests
            .contains(&"tests/test_templating.py".to_string())));

        let checked = checked_files_for_suite(PublicSuitePreset::Flask, &suite);
        assert!(checked.contains(&"pyproject.toml".to_string()));
        assert!(checked.contains(&"src/flask/__init__.py".to_string()));
        assert!(!checked.contains(&"build.gradle".to_string()));

        let health =
            public_suite_health(PublicSuitePreset::Flask, &repo, 1, &suite).expect("health");
        assert!(health.ok);
        assert_eq!(health.preset, "flask");
        assert!(health.missing_files.is_empty());
    }

    #[test]
    fn ripgrep_public_suite_uses_rust_paths_and_health_anchors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::Ripgrep, &repo).expect("fixture repo");
        let suite = ripgrep_suite();

        validate_suite(&suite).expect("suite");
        assert_eq!(suite.name, "ripgrep-public");
        assert_eq!(suite.tasks.len(), 4);
        assert!(suite
            .tasks
            .iter()
            .all(|task| task.tags.contains(&"rust".to_string())));
        assert!(suite.tasks.iter().any(|task| task
            .expected_files
            .contains(&"crates/ignore/src/gitignore.rs".to_string())));
        assert!(suite
            .tasks
            .iter()
            .any(|task| task.expected_tests.contains(&"tests/json.rs".to_string())));

        let checked = checked_files_for_suite(PublicSuitePreset::Ripgrep, &suite);
        assert!(checked.contains(&"Cargo.toml".to_string()));
        assert!(checked.contains(&"crates/cli/Cargo.toml".to_string()));
        assert!(checked.contains(&"crates/searcher/src/searcher/core.rs".to_string()));
        assert!(!checked.contains(&"pyproject.toml".to_string()));

        let health =
            public_suite_health(PublicSuitePreset::Ripgrep, &repo, 1, &suite).expect("health");
        assert!(health.ok);
        assert_eq!(health.preset, "ripgrep");
        assert!(health.missing_files.is_empty());
        assert!(health.privacy.source_free);
    }

    #[test]
    fn express_public_suite_uses_javascript_paths_and_health_anchors() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(PublicSuitePreset::Express, &repo).expect("fixture repo");
        let suite = express_suite();

        validate_suite(&suite).expect("suite");
        assert_eq!(suite.name, "express-public");
        assert_eq!(suite.tasks.len(), 4);
        assert!(suite
            .tasks
            .iter()
            .all(|task| task.tags.contains(&"javascript".to_string())));
        assert!(suite.tasks.iter().any(|task| task
            .expected_files
            .contains(&"lib/application.js".to_string())));
        assert!(suite.tasks.iter().any(|task| task
            .expected_tests
            .contains(&"test/res.jsonp.js".to_string())));

        let checked = checked_files_for_suite(PublicSuitePreset::Express, &suite);
        assert!(checked.contains(&"package.json".to_string()));
        assert!(checked.contains(&"index.js".to_string()));
        assert!(checked.contains(&"test/middleware.basic.js".to_string()));
        assert!(!checked.contains(&"Cargo.toml".to_string()));

        let health =
            public_suite_health(PublicSuitePreset::Express, &repo, 1, &suite).expect("health");
        assert!(health.ok);
        assert_eq!(health.preset, "express");
        assert!(health.missing_files.is_empty());
        assert!(health.privacy.source_free);
    }

    #[test]
    fn suite_health_accepts_generic_demo_suite() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        let suite_path = temp.path().join("suite.json");
        init_demo_repo(&repo, &suite_path, false).expect("demo repo");
        let suite = load_suite(&suite_path).expect("suite");

        let health = suite_health_report(None, &repo, 1, false, false, false, false, &suite, &[])
            .expect("suite health");

        assert!(health.ok);
        assert_eq!(health.preset, "custom");
        assert_eq!(health.suite_name, "demo-tiny-repo");
        assert_eq!(health.task_count, 2);
        assert_eq!(health.expected_file_count, 2);
        assert_eq!(health.expected_test_count, 2);
        assert!(health.validation_ready);
        assert!(!health.setup_commands_required);
        assert!(health.setup_command_ready);
        assert!(health.tasks_missing_setup_command.is_empty());
        assert!(health.missing_expected_files.is_empty());
        assert!(health.missing_expected_tests.is_empty());
        assert!(health.tasks_missing_success_command.is_empty());
        assert_eq!(health.evidence_use, SuiteEvidenceUse::NavigationOnly);
        assert!(health.privacy.source_free);

        let rendered = render_markdown_suite_health(&health);
        assert!(rendered.contains("Status: **healthy**"));
        assert!(rendered.contains("Raw source logged: `false`"));
    }

    #[test]
    fn suite_health_can_gate_validation_baseline_without_command_text() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo).expect("repo");
        write_demo_file(&repo, "src/target.txt", "current\n").expect("target");
        write_demo_file(&repo, "tests/target.test", "fixture\n").expect("test");
        init_git_repo(&repo).expect("git repo");

        let mut suite = helmbench::TaskSuite {
            schema_version: helmbench::SUITE_SCHEMA_VERSION,
            name: "baseline-suite".to_string(),
            description: String::new(),
            tasks: vec![helmbench::BenchTask {
                id: "needs-agent-change".to_string(),
                prompt: "Fix the target.".to_string(),
                expected_files: vec!["src/target.txt".to_string()],
                expected_tests: vec!["tests/target.test".to_string()],
                success_command: Some("test -f fixed.txt".to_string()),
                setup_commands: Vec::new(),
                tags: Vec::new(),
                timeout_seconds: Some(30),
            }],
        };

        let failing = suite_health_report(None, &repo, 1, false, true, false, false, &suite, &[])
            .expect("failing baseline health");
        assert!(failing.ok);
        assert!(failing.success_command_check_requested);
        assert!(failing.success_command_check_ready);
        assert!(failing.validation_baseline_ready);
        assert_eq!(failing.evidence_use, SuiteEvidenceUse::OutcomeReady);
        assert_eq!(failing.baseline_success_command_fail_count, 1);
        assert_eq!(failing.baseline_success_command_pass_count, 0);
        assert_eq!(failing.baseline_success_commands.len(), 1);
        assert!(failing.baseline_success_commands[0]
            .command_hash
            .starts_with("cmd:"));

        let missing_setup =
            suite_health_report(None, &repo, 1, false, true, false, true, &suite, &[])
                .expect("missing setup health");
        assert!(!missing_setup.ok);
        assert!(missing_setup.setup_commands_required);
        assert!(!missing_setup.setup_command_ready);
        assert_eq!(missing_setup.evidence_use, SuiteEvidenceUse::Unhealthy);
        assert_eq!(
            missing_setup.tasks_missing_setup_command,
            vec!["needs-agent-change"]
        );

        suite.tasks[0].success_command = Some("test -f src/target.txt".to_string());
        suite.tasks[0].setup_commands = vec!["rm src/target.txt".to_string()];
        let seeded = suite_health_report(None, &repo, 1, false, true, false, true, &suite, &[])
            .expect("seeded baseline health");
        assert!(seeded.ok);
        assert!(seeded.setup_commands_required);
        assert!(seeded.setup_command_ready);
        assert!(seeded.tasks_missing_setup_command.is_empty());
        assert!(seeded.validation_baseline_ready);
        assert_eq!(seeded.evidence_use, SuiteEvidenceUse::OutcomeReady);
        assert_eq!(seeded.baseline_success_command_fail_count, 1);
        assert!(seeded.tasks_failed_setup_command.is_empty());

        suite.tasks[0].setup_commands = vec!["exit 2".to_string()];
        let setup_failed =
            suite_health_report(None, &repo, 1, false, true, false, true, &suite, &[])
                .expect("setup failure health");
        assert!(!setup_failed.ok);
        assert_eq!(
            setup_failed.tasks_failed_setup_command,
            vec!["needs-agent-change"]
        );
        assert_eq!(setup_failed.baseline_success_commands.len(), 0);

        suite.tasks[0].success_command = Some("test -f src/target.txt".to_string());
        suite.tasks[0].setup_commands = Vec::new();
        suite.tasks.push(helmbench::BenchTask {
            id: "second-task".to_string(),
            prompt: "Fix the second target.".to_string(),
            expected_files: vec!["src/target.txt".to_string()],
            expected_tests: vec!["tests/target.test".to_string()],
            success_command: Some("test -f never-created.txt".to_string()),
            setup_commands: Vec::new(),
            tags: Vec::new(),
            timeout_seconds: Some(30),
        });
        let passing = suite_health_report(None, &repo, 1, false, true, false, false, &suite, &[])
            .expect("passing baseline health");
        assert!(!passing.ok);
        assert!(!passing.validation_baseline_ready);
        assert_eq!(passing.evidence_use, SuiteEvidenceUse::NavigationOnly);
        assert_eq!(passing.baseline_success_command_pass_count, 1);
        assert_eq!(passing.baseline_success_commands.len(), 2);

        let fail_fast = suite_health_report(None, &repo, 1, false, true, true, false, &suite, &[])
            .expect("fail-fast baseline health");
        assert!(!fail_fast.ok);
        assert!(fail_fast.success_command_check_fail_fast);
        assert_eq!(fail_fast.baseline_success_command_pass_count, 1);
        assert_eq!(fail_fast.baseline_success_command_skipped_count, 1);
        assert_eq!(fail_fast.baseline_success_commands.len(), 1);
        let rendered = render_markdown_suite_health(&passing);
        assert!(rendered.contains("Validation baseline"));
        assert!(rendered.contains("Evidence use"));
        assert!(rendered.contains("`navigation_only`"));
        assert!(!rendered.contains("test -f src/target.txt"));
    }

    #[test]
    fn suite_health_reports_missing_evidence_without_source_logs() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        std::fs::create_dir_all(&repo).expect("repo");
        write_demo_file(&repo, "README.md", "fixture\n").expect("readme");
        init_git_repo(&repo).expect("git repo");
        let suite = helmbench::TaskSuite {
            schema_version: helmbench::SUITE_SCHEMA_VERSION,
            name: "bad-suite".to_string(),
            description: String::new(),
            tasks: vec![helmbench::BenchTask {
                id: "missing-target".to_string(),
                prompt: "Fix the missing target.".to_string(),
                expected_files: vec!["src/missing.rs".to_string()],
                expected_tests: vec!["tests/missing.rs".to_string()],
                success_command: None,
                setup_commands: Vec::new(),
                tags: Vec::new(),
                timeout_seconds: None,
            }],
        };

        let health = suite_health_report(None, &repo, 1, false, false, false, false, &suite, &[])
            .expect("suite health");

        assert!(!health.ok);
        assert_eq!(health.missing_expected_files, vec!["src/missing.rs"]);
        assert_eq!(health.missing_expected_tests, vec!["tests/missing.rs"]);
        assert_eq!(health.tasks_missing_success_command, vec!["missing-target"]);
        assert!(health
            .missing_files
            .iter()
            .all(|path| !path.starts_with('/')));
        assert!(health.privacy.source_free);
    }

    #[test]
    fn public_suite_defaults_are_preset_specific() {
        assert_eq!(
            default_public_suite_out(PublicSuitePreset::RefactoringMiner),
            PathBuf::from("suites/refactoring-miner-public.json")
        );
        assert_eq!(
            default_public_health_out(PublicSuitePreset::RefactoringMiner),
            PathBuf::from(".helmbench/refactoring-miner-public-suite-health.json")
        );
        assert_eq!(
            default_public_matrix_config_out(PublicSuitePreset::RefactoringMiner),
            PathBuf::from(".helmbench/refactoring-miner-matrix.json")
        );
        assert_eq!(
            default_public_matrix_out_dir(PublicSuitePreset::RefactoringMiner),
            PathBuf::from(".helmbench/refactoring-miner-matrix")
        );
        assert_eq!(
            default_public_suite_out(PublicSuitePreset::Flask),
            PathBuf::from("suites/flask-public.json")
        );
        assert_eq!(
            default_public_health_out(PublicSuitePreset::Flask),
            PathBuf::from(".helmbench/flask-public-suite-health.json")
        );
        assert_eq!(
            default_public_suite_out(PublicSuitePreset::Ripgrep),
            PathBuf::from("suites/ripgrep-public.json")
        );
        assert_eq!(
            default_public_health_out(PublicSuitePreset::Ripgrep),
            PathBuf::from(".helmbench/ripgrep-public-suite-health.json")
        );
        assert_eq!(
            default_public_suite_out(PublicSuitePreset::Express),
            PathBuf::from("suites/express-public.json")
        );
        assert_eq!(
            default_public_health_out(PublicSuitePreset::Express),
            PathBuf::from(".helmbench/express-public-suite-health.json")
        );
    }

    #[test]
    fn evidence_bundle_writes_source_free_manifest_and_summaries() {
        let temp = tempfile::tempdir().expect("tempdir");
        let suite = example_suite();
        let suite_path = temp.path().join("suite.json");
        write_json(&suite, &suite_path).expect("suite");
        let health_path = temp.path().join("health.json");
        write_json(
            &PublicSuiteHealth {
                schema_version: 1,
                preset: "test".to_string(),
                suite_name: suite.name.clone(),
                task_count: suite.tasks.len(),
                repo_name: "repo".to_string(),
                head: Some("abc123".to_string()),
                commit_count: Some(1),
                min_commits: 1,
                allow_dirty: false,
                dirty: false,
                fsck_ok: true,
                validation_ready: true,
                expected_file_count: 1,
                expected_test_count: 1,
                checked_files: vec!["README.md".to_string()],
                missing_files: Vec::new(),
                missing_expected_files: Vec::new(),
                missing_expected_tests: Vec::new(),
                setup_commands_required: false,
                setup_command_ready: true,
                tasks_missing_setup_command: Vec::new(),
                tasks_failed_setup_command: Vec::new(),
                tasks_missing_success_command: Vec::new(),
                success_command_check_requested: false,
                success_command_check_fail_fast: false,
                success_command_check_ready: false,
                validation_baseline_ready: false,
                baseline_success_command_pass_count: 0,
                baseline_success_command_fail_count: 0,
                baseline_success_command_timeout_count: 0,
                baseline_success_command_skipped_count: 0,
                baseline_success_commands: Vec::new(),
                evidence_use: SuiteEvidenceUse::NavigationOnly,
                ok: true,
                privacy: PrivacyStatus::source_free(),
            },
            &health_path,
        )
        .expect("health");

        let base_report = build_report(
            &suite,
            &[helmbench::AgentTrace {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: "auth-redirect-001".to_string(),
                agent: "claude-code".to_string(),
                variant: AgentVariant::Native,
                status: TaskStatus::Failure,
                recommended_files: Vec::new(),
                files_read: vec![bundle_path("README.md")],
                files_edited: Vec::new(),
                commands: Vec::new(),
                tool_call_count: 5,
                token_estimate: Some(1000),
                elapsed_millis: Some(1000),
                time_to_first_relevant_file_millis: None,
                privacy: PrivacyStatus::source_free(),
            }],
        )
        .expect("base report");
        let head_report = build_report(
            &suite,
            &[helmbench::AgentTrace {
                schema_version: TRACE_SCHEMA_VERSION,
                task_id: "auth-redirect-001".to_string(),
                agent: "claude-code".to_string(),
                variant: AgentVariant::CtxhelmMcp,
                status: TaskStatus::Success,
                recommended_files: vec![bundle_path("src/auth/session.ts")],
                files_read: vec![bundle_path("src/auth/session.ts")],
                files_edited: vec![bundle_path("src/auth/session.ts")],
                commands: Vec::new(),
                tool_call_count: 3,
                token_estimate: Some(700),
                elapsed_millis: Some(800),
                time_to_first_relevant_file_millis: None,
                privacy: PrivacyStatus::source_free(),
            }],
        )
        .expect("head report");
        let base_path = temp.path().join("base.json");
        let head_path = temp.path().join("head.json");
        write_json(&base_report, &base_path).expect("base");
        write_json(&head_report, &head_path).expect("head");
        let out_dir = temp.path().join("bundle");

        write_evidence_bundle(
            &suite_path,
            Some(&health_path),
            &base_path,
            std::slice::from_ref(&head_path),
            &out_dir,
            false,
        )
        .expect("bundle");

        assert!(out_dir.join("suite.json").exists());
        assert!(out_dir.join("health.json").exists());
        assert!(out_dir.join("reports/base.json").exists());
        assert!(out_dir.join("reports/head-1.json").exists());
        assert!(out_dir.join("benchmark-summary.json").exists());
        assert!(out_dir.join("benchmark-summary.md").exists());
        let manifest = std::fs::read_to_string(out_dir.join("manifest.json")).expect("manifest");
        let manifest = serde_json::from_str::<serde_json::Value>(&manifest).expect("json");
        assert_eq!(manifest["suiteName"], "example-auth-bugs");
        assert_eq!(manifest["evidenceUse"], "navigation_only");
        assert_eq!(manifest["privacy"]["sourceFree"], true);
        let artifacts = manifest["artifacts"].as_array().expect("artifacts");
        assert_eq!(artifacts.len(), 6);
        assert!(artifacts.iter().all(|artifact| {
            artifact["path"]
                .as_str()
                .is_some_and(|path| !path.starts_with('/'))
        }));
        assert!(artifacts.iter().all(|artifact| artifact["contentHash"]
            .as_str()
            .is_some_and(|hash| hash.starts_with("fnv64:"))));

        verify_evidence_bundle(&out_dir).expect("verify bundle");
        let manifest_path = out_dir.join("manifest.json");
        let manifest_raw = std::fs::read_to_string(&manifest_path).expect("manifest raw");
        let mut mismatched_manifest: EvidenceBundleManifest =
            serde_json::from_str(&manifest_raw).expect("manifest struct");
        mismatched_manifest.evidence_use = SuiteEvidenceUse::OutcomeReady;
        write_json(&mismatched_manifest, &manifest_path).expect("write mismatched manifest");
        let err = verify_evidence_bundle(&out_dir).expect_err("mismatched evidence use");
        assert!(
            err.to_string()
                .contains("evidenceUse `outcome_ready` does not match"),
            "{err}"
        );
        std::fs::write(&manifest_path, manifest_raw).expect("restore manifest");

        std::fs::write(out_dir.join("reports/head-1.json"), b"tampered").expect("tamper");
        let err = verify_evidence_bundle(&out_dir).expect_err("tampered bundle should fail");
        assert!(
            err.to_string().contains("byte count mismatch")
                || err.to_string().contains("hash mismatch"),
            "{err}"
        );
    }

    #[test]
    fn doctor_accepts_current_checkout() {
        run_doctor(Path::new(env!("CARGO_MANIFEST_DIR"))).expect("doctor");
    }

    #[test]
    fn doctor_report_describes_direct_runner_readiness_source_free() {
        let report = build_doctor_report(Path::new(env!("CARGO_MANIFEST_DIR")));

        assert!(report.ok);
        assert_eq!(report.schema_version, 1);
        assert!(report.privacy.source_free);
        assert!(report.direct_runners.iter().any(|runner| {
            runner.name == "claude-run"
                && runner.injects_source_free_event_contract
                && runner.capture_stream_supported
                && runner.suppresses_raw_output_by_default
        }));
        assert!(report
            .direct_runners
            .iter()
            .any(|runner| runner.name == "codex-run" && runner.isolated_clones));
        assert!(report
            .observation_modes
            .iter()
            .any(|mode| mode.name == "capture-stream"
                && mode.source_free
                && !mode.persists_raw_stream));
        assert!(report
            .supported_variants
            .contains(&AgentVariant::NativeSearch));

        let json = serde_json::to_string(&report).expect("json");
        assert!(!json.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(!json.contains("rawTranscriptLogged\":true"));
    }

    #[test]
    fn launch_readiness_report_classifies_checked_in_smoke_proof_source_free() {
        let root = Path::new(env!("CARGO_MANIFEST_DIR"));
        let report = build_launch_readiness_report(
            &root.join("suites/example-auth-bugs.json"),
            &root.join("reports/example-native.json"),
            &[root.join("reports/example-ctxhelm.json")],
            &[],
            &[],
            helmbench::MIN_RECOMMENDED_BENCHMARK_TASKS,
            2,
        )
        .expect("launch readiness");

        assert_eq!(report.schema_version, LAUNCH_READINESS_SCHEMA_VERSION);
        assert_eq!(report.suite_name, "example-auth-bugs");
        assert_eq!(report.status, LaunchReadinessStatus::SmokeProof);
        assert_eq!(report.benchmark.task_count, 1);
        assert_eq!(report.benchmark.real_agent_run_count, 2);
        assert!(report.benchmark.low_sample_warning);
        assert!(report.privacy.source_free);
        assert!(report.checks.iter().any(|check| {
            check.name == "recommended task count"
                && check.status == LaunchReadinessCheckStatus::Warn
        }));
        assert!(report.checks.iter().any(|check| {
            check.name == "verified run matrix" && check.status == LaunchReadinessCheckStatus::Warn
        }));

        let rendered = render_markdown_launch_readiness(&report);
        assert!(rendered.contains("Status: **smoke_proof**"));
        assert!(rendered.contains("Best success rate"));

        let json = serde_json::to_string(&report).expect("json");
        assert!(!json.contains(env!("CARGO_MANIFEST_DIR")));
        assert!(!json.contains("rawSourceLogged\":true"));
        assert!(json.contains("\"sourceFree\":true"));
    }

    #[test]
    fn collect_ctxhelm_paths_rejects_unsafe_paths() {
        let value = serde_json::json!({
            "targetFiles": [
                {"path": "src/auth/session.txt"},
                {"path": "../secret.env"}
            ]
        });
        let mut paths = Vec::new();
        let error =
            collect_ctxhelm_paths(&value, "targetFiles", &mut paths).expect_err("unsafe path");
        assert!(error.to_string().contains("parent traversal"));
    }

    #[test]
    fn name_only_paths_are_source_free_and_deduped() {
        let paths = parse_name_only_paths(
            "src/auth/session.txt\n.helmbench/events.jsonl\nsrc/auth/session.txt\n tests/auth/session.test.sh \n",
        )
        .expect("paths");

        assert_eq!(
            paths,
            vec![
                "src/auth/session.txt".to_string(),
                "tests/auth/session.test.sh".to_string()
            ]
        );

        let err = parse_name_only_paths("../secret.txt\n").expect_err("unsafe path");
        assert!(err.to_string().contains("parent traversal"));
    }

    #[test]
    fn pr_labels_are_source_free() {
        assert_eq!(source_free_pr_label("42"), "pr:42");
        let branch_label = source_free_pr_label("feature/read-auth-source");
        assert!(branch_label.starts_with("pr-ref:"));
        assert!(!branch_label.contains("feature"));
        assert!(!branch_label.contains('/'));
    }

    #[test]
    fn run_ctxhelm_json_command_supports_pack_options() {
        let config = CtxhelmRunConfig {
            ctxhelm_bin: PathBuf::from("ctxhelm"),
            mode: "bug-fix".to_string(),
            target_agent: "generic".to_string(),
            semantic: true,
            semantic_provider: Some("local_hash".to_string()),
            semantic_model: None,
            semantic_dimensions: Some(64),
            include_pack: true,
            pack_budget: "brief".to_string(),
        };

        assert!(config.semantic);
        assert_eq!(config.pack_budget, "brief");
        assert_eq!(config.semantic_dimensions, Some(64));
    }

    fn create_public_suite_fixture_repo(preset: PublicSuitePreset, repo: &Path) -> Result<()> {
        let suite = public_suite_for_preset(preset);
        std::fs::create_dir_all(repo).with_context(|| format!("create {}", repo.display()))?;
        for path in checked_files_for_suite(preset, &suite) {
            write_demo_file(repo, &path, "fixture\n")?;
        }
        init_git_repo(repo)
    }

    fn bundle_path(path: &str) -> helmbench::PathObservation {
        helmbench::PathObservation {
            path: path.to_string(),
            path_hash: None,
            observed_at_millis: None,
        }
    }

    const FAKE_DEMO_AGENT: &str = r#"#!/usr/bin/env sh
set -eu

case "$HELMBENCH_TASK_ID" in
  demo-auth-redirect-001)
    path=src/auth/session.txt
    printf 'expired sessions redirect to /login\nactive sessions redirect to /dashboard\n' > "$path"
    ;;
  demo-billing-rounding-001)
    path=src/billing/invoice.txt
    printf 'invoice rounding mode: round half up\ncurrency: USD\n' > "$path"
    ;;
  *)
    exit 2
    ;;
esac

mkdir -p "$(dirname "$HELMBENCH_EVENTS")"
printf '{"schemaVersion":1,"taskId":"%s","eventKind":"recommended_file","path":"%s","observedAtMillis":5}\n' "$HELMBENCH_TASK_ID" "$path" >> "$HELMBENCH_EVENTS"
printf '{"schemaVersion":1,"taskId":"%s","eventKind":"file_read","path":"%s","observedAtMillis":15}\n' "$HELMBENCH_TASK_ID" "$path" >> "$HELMBENCH_EVENTS"
"#;

    const FAKE_STREAM_AGENT: &str = r#"#!/usr/bin/env sh
set -eu

case "$HELMBENCH_TASK_ID" in
  demo-auth-redirect-001)
    path=src/auth/session.txt
    printf '{"tool":"Read","input":{"path":"%s"}}\n' "$path"
    printf 'expired sessions redirect to /login\nactive sessions redirect to /dashboard\n' > "$path"
    ;;
  demo-billing-rounding-001)
    path=src/billing/invoice.txt
    printf '{"tool":"Read","input":{"path":"%s"}}\n' "$path"
    printf 'invoice rounding mode: round half up\ncurrency: USD\n' > "$path"
    ;;
  *)
    exit 2
    ;;
esac
"#;

    const FAKE_CTXHELM: &str = r#"#!/usr/bin/env sh
set -eu

case "${1:-}" in
  prepare-task)
    printf '{"targetFiles":[{"path":"src/auth/session.txt"}],"relatedTests":[{"path":"auth.test"}]}\n'
    ;;
  get-pack)
    printf '{"tokenEstimate":321,"sections":[]}\n'
    ;;
  *)
    exit 2
    ;;
esac
"#;
}
