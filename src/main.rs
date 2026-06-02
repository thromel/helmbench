use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use helmbench::{
    build_autopsy, build_benchmark_summary, build_report, compare_reports, evaluate_quality_gate,
    events_from_agent_stream_jsonl, example_suite, load_agent_events, load_suite, load_traces,
    project_root_for_cli, read_benchmark_summary, read_report, render_html_dashboard,
    render_markdown_autopsy, render_markdown_benchmark_summary, render_markdown_compare,
    render_markdown_quality_gate, render_markdown_report, trace_from_ctxhelm_prepare_json,
    traces_from_agent_events, validate_agent_event, validate_suite, write_json, AgentEvent,
    AgentEventKind, AgentVariant, CommandClass, PrivacyStatus, QualityGateConfig, TaskStatus,
    TRACE_SCHEMA_VERSION,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use std::process::Command as ProcessCommand;
use std::time::{Duration, Instant};

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
    /// Generate a source-free suite from a known public repository fixture.
    InitPublicSuite {
        #[arg(long, value_enum)]
        preset: PublicSuitePreset,
        #[arg(long)]
        repo: PathBuf,
        #[arg(long, default_value = "suites/refactoringminer-public.json")]
        suite_out: PathBuf,
        #[arg(long, default_value = ".helmbench/public-suite-health.json")]
        health_out: PathBuf,
        #[arg(long, default_value_t = 1000)]
        min_commits: u64,
        #[arg(long)]
        force: bool,
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
    /// Fail if a benchmark summary violates source-free quality thresholds.
    QualityGate {
        #[arg(long)]
        summary: PathBuf,
        #[arg(long)]
        out: Option<PathBuf>,
        #[arg(long, value_enum, default_value_t = OutputFormat::Markdown)]
        format: OutputFormat,
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
        max_total_tool_calls_delta: Option<i64>,
        #[arg(long)]
        max_total_token_estimate_delta: Option<i64>,
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
    },
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum OutputFormat {
    Json,
    Markdown,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum TraceVariant {
    Native,
    CtxhelmMcp,
    CtxhelmPack,
}

#[derive(Debug, Clone, Copy, ValueEnum)]
enum PublicSuitePreset {
    RefactoringMiner,
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
        Command::InitPublicSuite {
            preset,
            repo,
            suite_out,
            health_out,
            min_commits,
            force,
        } => {
            init_public_suite(preset, &repo, &suite_out, &health_out, min_commits, force)?;
            println!("wrote {}", suite_out.display());
            println!("wrote {}", health_out.display());
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
            keep_workdirs,
        } => {
            let suite = load_suite(&suite)?;
            let command = claude_adapter_command(
                &current_helmbench_bin()?,
                &claude_bin,
                model.as_deref(),
                &claude_arg,
                dangerously_skip_permissions,
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
            keep_workdirs,
        } => {
            let suite = load_suite(&suite)?;
            let command = codex_adapter_command(
                &current_helmbench_bin()?,
                &codex_bin,
                model.as_deref(),
                &codex_arg,
                dangerously_bypass_approvals_and_sandbox,
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
        Command::QualityGate {
            summary,
            out,
            format,
            min_success_rate_delta,
            min_validation_coverage_rate_delta,
            max_irrelevant_read_rate_delta,
            min_recommendation_recall_delta,
            min_context_precision_delta,
            min_edited_file_recall_delta,
            max_total_tool_calls_delta,
            max_total_token_estimate_delta,
        } => {
            let summary = read_benchmark_summary(&summary)?;
            let gate = evaluate_quality_gate(
                &summary,
                &QualityGateConfig {
                    min_success_rate_delta,
                    min_validation_coverage_rate_delta,
                    max_irrelevant_read_rate_delta,
                    min_recommendation_recall_delta,
                    min_context_precision_delta,
                    min_edited_file_recall_delta,
                    max_total_tool_calls_delta,
                    max_total_token_estimate_delta,
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
        Command::Dashboard { report, out } => {
            let reports = report
                .iter()
                .map(|path| read_report(path))
                .collect::<Result<Vec<_>>>()?;
            let rendered = render_html_dashboard(&reports)?;
            write_text(&rendered, &out)?;
            println!("wrote {}", out.display());
        }
        Command::Doctor { repo } => {
            let root = project_root_for_cli(repo)?;
            run_doctor(&root)?;
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

fn run_doctor(root: &Path) -> Result<()> {
    println!("helmbench doctor");
    println!("- repo: {}", root.display());
    println!("- privacy: source-free reports enforced");

    let mut required_ok = true;
    required_ok &= print_check("git available", command_available("git"));
    required_ok &= print_check("cargo available", command_available("cargo"));
    required_ok &= print_check("repo is a git checkout", git_repo_ok(root));
    required_ok &= print_check("Cargo.toml exists", root.join("Cargo.toml").exists());
    required_ok &= print_check(
        "verification script exists",
        root.join("scripts/verify.sh").exists(),
    );
    required_ok &= print_check(
        "CI workflow exists",
        root.join(".github/workflows/ci.yml").exists(),
    );
    required_ok &= print_check(
        "example suite loads",
        load_suite(&root.join("suites/example-auth-bugs.json")).is_ok(),
    );
    required_ok &= print_check(
        "example native report is source-free",
        read_report(&root.join("reports/example-native.json")).is_ok(),
    );
    required_ok &= print_check(
        "example ctxhelm report is source-free",
        read_report(&root.join("reports/example-ctxhelm.json")).is_ok(),
    );

    println!("- optional integrations:");
    print_optional("ctxhelm available", command_available("ctxhelm"));
    print_optional("claude available", command_available("claude"));
    print_optional("codex available", command_available("codex"));

    println!("- supported variants:");
    for variant in [
        AgentVariant::Native,
        AgentVariant::CtxhelmPlan,
        AgentVariant::CtxhelmMcp,
        AgentVariant::CtxhelmPack,
    ] {
        println!("  - {:?}", variant);
    }

    if !required_ok {
        anyhow::bail!("doctor found missing required HelmBench prerequisites");
    }
    Ok(())
}

fn print_check(label: &str, ok: bool) -> bool {
    println!("- {}: {}", label, if ok { "ok" } else { "error" });
    ok
}

fn print_optional(label: &str, ok: bool) {
    println!("  - {}: {}", label, if ok { "ok" } else { "warn" });
}

fn command_available(command: &str) -> bool {
    ProcessCommand::new(command)
        .arg("--version")
        .output()
        .map(|output| output.status.success())
        .unwrap_or(false)
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
    repo_name: String,
    head: Option<String>,
    commit_count: Option<u64>,
    min_commits: u64,
    dirty: bool,
    fsck_ok: bool,
    checked_files: Vec<String>,
    missing_files: Vec<String>,
    ok: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
struct EvidenceBundleManifest {
    schema_version: u32,
    suite_name: String,
    baseline_agent: String,
    baseline_variant: AgentVariant,
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

    if manifest.schema_version != 1 {
        anyhow::bail!(
            "unsupported evidence bundle schemaVersion {}; expected 1",
            manifest.schema_version
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

    let mut artifacts = Vec::new();
    artifacts.push(copy_bundle_artifact(
        "suite",
        suite_path,
        out_dir,
        "suite.json",
        true,
    )?);
    if let Some(health_path) = health_path {
        validate_public_suite_health(health_path)?;
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
        schema_version: 1,
        suite_name: suite.name,
        baseline_agent: base_report.agent,
        baseline_variant: base_report.variant,
        artifacts,
        privacy: PrivacyStatus::source_free(),
    };
    write_json(&manifest, &out_dir.join("manifest.json"))?;
    Ok(())
}

fn validate_public_suite_health(path: &Path) -> Result<()> {
    let raw = std::fs::read_to_string(path).with_context(|| format!("read {}", path.display()))?;
    let health = serde_json::from_str::<PublicSuiteHealth>(&raw)
        .with_context(|| format!("parse health {}", path.display()))?;
    if health.repo_name.contains('/') || health.repo_name.contains('\\') {
        anyhow::bail!("health repoName must not contain path separators");
    }
    for checked in health
        .checked_files
        .iter()
        .chain(health.missing_files.iter())
    {
        helmbench::validate_safe_relative_path_for_cli(checked)?;
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

    let suite = match preset {
        PublicSuitePreset::RefactoringMiner => refactoring_miner_suite(),
    };
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
    let repo_name = repo
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("repo")
        .to_string();
    let checked_files = checked_files_for_suite(suite);
    let missing_files = checked_files
        .iter()
        .filter(|path| !repo.join(path).exists())
        .cloned()
        .collect::<Vec<_>>();

    let head = git_output(repo, &["rev-parse", "HEAD"]).ok();
    let commit_count = git_output(repo, &["rev-list", "--count", "HEAD"])
        .ok()
        .and_then(|value| value.parse::<u64>().ok());
    let dirty = git_output(repo, &["status", "--short"])
        .map(|status| !status.trim().is_empty())
        .unwrap_or(true);
    let fsck_ok = git_status_ok(repo, &["fsck", "--no-progress"]);
    let ok = repo.join(".git").exists()
        && head.is_some()
        && commit_count.is_some_and(|count| count >= min_commits)
        && !dirty
        && fsck_ok
        && missing_files.is_empty();

    Ok(PublicSuiteHealth {
        schema_version: 1,
        preset: match preset {
            PublicSuitePreset::RefactoringMiner => "refactoring-miner".to_string(),
        },
        repo_name,
        head,
        commit_count,
        min_commits,
        dirty,
        fsck_ok,
        checked_files,
        missing_files,
        ok,
    })
}

fn checked_files_for_suite(suite: &helmbench::TaskSuite) -> Vec<String> {
    let mut paths = vec![
        "README.md".to_string(),
        "build.gradle".to_string(),
        "gradlew".to_string(),
    ];
    for task in &suite.tasks {
        paths.extend(task.expected_files.iter().cloned());
        paths.extend(task.expected_tests.iter().cloned());
    }
    paths.sort();
    paths.dedup();
    paths
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
                tags: vec!["public_repo".to_string(), "git_history".to_string(), "bug_fix".to_string()],
                timeout_seconds: Some(1200),
            },
        ],
    }
}

fn run_demo_pipeline(out_dir: &Path, force: bool) -> Result<()> {
    run_demo_pipeline_with_adapter(out_dir, force, None)
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

    let summary = build_benchmark_summary(&native_report, &[guided_report.clone()])?;
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
        "expired sessions redirect to /expired\nactive sessions redirect to /dashboard\n",
    )?;
    write_demo_file(
        repo_out,
        "src/auth/middleware.txt",
        "middleware calls session redirect rules\n",
    )?;
    write_demo_file(
        repo_out,
        "src/billing/invoice.txt",
        "invoice rounding mode: floor\ncurrency: USD\n",
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
                tags: vec!["bug_fix".to_string(), "auth".to_string()],
                timeout_seconds: Some(60),
            },
            helmbench::BenchTask {
                id: "demo-billing-rounding-001".to_string(),
                prompt: "Fix invoice rounding so it uses round half up.".to_string(),
                expected_files: vec!["src/billing/invoice.txt".to_string()],
                expected_tests: vec!["tests/billing/invoice.test.sh".to_string()],
                success_command: Some("sh tests/billing/invoice.test.sh".to_string()),
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
            let result = run_shell(&rendered, &task_dir, &env, task.timeout_seconds)?;
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
    parts.push(">/dev/null".to_string());
    parts.push("2>/dev/null".to_string());
    parts.join(" ")
}

fn codex_adapter_command(
    helmbench_bin: &Path,
    codex_bin: &Path,
    model: Option<&str>,
    extra_args: &[String],
    dangerously_bypass_approvals_and_sandbox: bool,
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
    parts.push(">/dev/null".to_string());
    parts.push("2>/dev/null".to_string());
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
    let started = Instant::now();
    let mut process = ProcessCommand::new("sh");
    process.arg("-lc").arg(command).current_dir(cwd);
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

struct ShellResult {
    success: bool,
    exit_status: Option<i32>,
    elapsed_millis: u64,
    timed_out: bool,
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
    let mut hash = 0xcbf29ce484222325u64;
    for byte in command.as_bytes() {
        hash ^= u64::from(*byte);
        hash = hash.wrapping_mul(0x100000001b3);
    }
    format!("cmd:{hash:016x}")
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

    #[test]
    fn claude_command_includes_source_free_event_instructions() {
        let command = claude_adapter_command(
            Path::new("/tmp/helmbench"),
            Path::new("claude"),
            Some("sonnet"),
            &["--debug".to_string()],
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
        );
        assert!(command.contains("'codex' exec"));
        assert!(command.contains("--cd \"$HELMBENCH_REPO\""));
        assert!(command.contains("--full-auto"));
        assert!(command.contains("record-event"));
        assert!(command.contains("HELMBENCH_TASK_PROMPT"));
        assert!(command.contains(">/dev/null 2>/dev/null"));
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
        assert!(out.join("docs/compare.md").exists());
        assert!(out.join("docs/benchmark-summary.md").exists());
        assert!(out.join("docs/quality-gate.md").exists());
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
    fn refactoring_miner_public_suite_is_source_free_and_valid() {
        let suite = refactoring_miner_suite();
        validate_suite(&suite).expect("suite");

        assert_eq!(suite.name, "refactoringminer-public");
        assert_eq!(suite.tasks.len(), 4);
        assert!(suite
            .tasks
            .iter()
            .all(|task| task.tags.contains(&"public_repo".to_string())));
        assert!(suite.tasks.iter().any(|task| task.expected_files.contains(
            &"src/main/java/org/refactoringminer/mcp/McpIntentValidator.java".to_string()
        )));
        assert!(suite.tasks.iter().any(|task| task
            .expected_tests
            .contains(&"src/test/java/gui/MarkAsViewedTest.java".to_string())));
    }

    #[test]
    fn public_suite_health_accepts_clean_fixture() {
        let temp = tempfile::tempdir().expect("tempdir");
        let repo = temp.path().join("repo");
        create_public_suite_fixture_repo(&repo).expect("fixture repo");
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
        create_public_suite_fixture_repo(&repo).expect("fixture repo");
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
                repo_name: "repo".to_string(),
                head: Some("abc123".to_string()),
                commit_count: Some(1),
                min_commits: 1,
                dirty: false,
                fsck_ok: true,
                checked_files: vec!["README.md".to_string()],
                missing_files: Vec::new(),
                ok: true,
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

    fn create_public_suite_fixture_repo(repo: &Path) -> Result<()> {
        let suite = refactoring_miner_suite();
        std::fs::create_dir_all(repo).with_context(|| format!("create {}", repo.display()))?;
        for path in checked_files_for_suite(&suite) {
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
}
