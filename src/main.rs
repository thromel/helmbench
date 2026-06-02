use anyhow::{Context, Result};
use clap::{Parser, Subcommand, ValueEnum};
use helmbench::{
    build_autopsy, build_report, compare_reports, example_suite, load_agent_events, load_suite,
    load_traces, project_root_for_cli, read_report, render_html_dashboard, render_markdown_autopsy,
    render_markdown_compare, render_markdown_report, trace_from_ctxhelm_prepare_json,
    traces_from_agent_events, validate_agent_event, validate_suite, write_json, AgentEvent,
    AgentEventKind, AgentVariant, CommandClass, PrivacyStatus, TaskStatus, TRACE_SCHEMA_VERSION,
};
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
}
