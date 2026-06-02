use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use helmbench::{
    build_report, compare_reports, example_suite, load_agent_events, load_suite, load_traces,
    project_root_for_cli, read_report, render_markdown_compare, render_markdown_report,
    trace_from_ctxhelm_prepare_json, traces_from_agent_events, validate_agent_event,
    validate_suite, write_json, AgentEvent, AgentEventKind, AgentVariant, CommandClass,
    PrivacyStatus, TaskStatus, TRACE_SCHEMA_VERSION,
};
use std::path::PathBuf;
use std::process::Command as ProcessCommand;
use std::time::Instant;

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
        Command::Doctor { repo } => {
            let root = project_root_for_cli(repo)?;
            println!("helmbench doctor");
            println!("- repo: {}", root.display());
            println!("- source-free reports: enforced");
            println!("- supported variants:");
            for variant in [
                AgentVariant::Native,
                AgentVariant::CtxhelmPlan,
                AgentVariant::CtxhelmMcp,
                AgentVariant::CtxhelmPack,
            ] {
                println!("  - {:?}", variant);
            }
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

fn append_event(path: &PathBuf, event: &AgentEvent) -> Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).with_context(|| format!("create {}", parent.display()))?;
    }
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
