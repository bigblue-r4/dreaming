use clap::{Parser, Subcommand, ValueEnum};
use dreaming::{
    engine::Engine,
    evaluator,
    provider::{
        Cancellation, ModelProvider, OpenAiCompatible, ProviderCapabilities, ScriptedProvider,
    },
    store::Store,
    types::*,
};
use std::path::{Path, PathBuf};
#[derive(Parser)]
#[command(version, about = "Reversible exploration and failure diagnosis")]
struct Cli {
    #[arg(long, global = true, default_value = "dreaming.sqlite")]
    db: PathBuf,
    #[arg(long, global = true)]
    config: Option<PathBuf>,
    #[arg(long, global = true)]
    endpoint: Option<String>,
    #[arg(long, global = true)]
    script: Option<PathBuf>,
    #[command(subcommand)]
    command: Command,
}
#[derive(Clone, ValueEnum)]
enum RunMode {
    Dream,
    Lucid,
    Diversity,
}
#[derive(Subcommand)]
enum Command {
    Baseline {
        #[arg(long)]
        input: PathBuf,
    },
    Run {
        #[arg(long, value_enum)]
        mode: RunMode,
        #[arg(long)]
        input: Option<PathBuf>,
        #[arg(long)]
        case: Option<PathBuf>,
    },
    Results {
        #[arg(long)]
        run: String,
    },
    Memory {
        #[command(subcommand)]
        command: MemoryCommand,
    },
    Benchmark {
        #[command(subcommand)]
        command: BenchmarkCommand,
    },
}
#[derive(Subcommand)]
enum MemoryCommand {
    Candidates,
    List,
    Validate {
        id: String,
        #[arg(long)]
        experiment: String,
        #[arg(long)]
        evidence: String,
    },
    Reject {
        id: String,
        #[arg(long)]
        experiment: String,
        #[arg(long)]
        evidence: String,
    },
    Retire {
        id: String,
        #[arg(long)]
        experiment: String,
        #[arg(long)]
        evidence: String,
    },
    Rollback {
        id: String,
        #[arg(long)]
        experiment: String,
        #[arg(long)]
        version: i64,
        #[arg(long)]
        evidence: String,
    },
}
#[derive(Subcommand)]
enum BenchmarkCommand {
    Aggregate {
        #[arg(long, value_delimiter = ',', required = true)]
        runs: Vec<String>,
    },
    Compare {
        #[arg(long)]
        baseline: String,
        #[arg(long)]
        run: String,
    },
}
fn read<T: serde::de::DeserializeOwned>(path: &Path) -> Result<T> {
    if std::fs::metadata(path).map_err(|e| e.to_string())?.len() > 1048576 {
        return Err("input file exceeds 1 MiB".into());
    }
    let data = std::fs::read(path).map_err(|e| e.to_string())?;
    if data.len() > 1048576 {
        return Err("input file exceeds 1 MiB".into());
    }
    serde_json::from_slice(&data).map_err(|e| e.to_string())
}
fn print<T: serde::Serialize>(value: &T) -> Result<()> {
    println!(
        "{}",
        serde_json::to_string_pretty(value).map_err(|e| e.to_string())?
    );
    Ok(())
}
#[tokio::main]
async fn main() {
    if let Err(e) = execute().await {
        eprintln!("dreaming: {e}");
        std::process::exit(1);
    }
}
async fn execute() -> Result<()> {
    let cli = Cli::parse();
    let mut store = Store::open(&cli.db)?;
    match &cli.command {
        Command::Results { run } => return print(&store.run(run)?),
        Command::Benchmark {
            command: BenchmarkCommand::Compare { baseline, run },
        } => {
            return print(&evaluator::compare(
                &store.run(baseline)?,
                &store.run(run)?,
            )?)
        }
        Command::Benchmark {
            command: BenchmarkCommand::Aggregate { runs },
        } => {
            let records = runs
                .iter()
                .map(|id| store.run(id))
                .collect::<Result<Vec<_>>>()?;
            return print(&evaluator::aggregate(&records)?);
        }
        Command::Memory { command } => {
            match command {
                MemoryCommand::Candidates => return print(&store.memories(Some("candidate"))?),
                MemoryCommand::List => return print(&store.memories(None)?),
                MemoryCommand::Validate {
                    id,
                    experiment,
                    evidence,
                } => store.revise(id, "validated", experiment, evidence, None)?,
                MemoryCommand::Reject {
                    id,
                    experiment,
                    evidence,
                } => store.revise(id, "rejected", experiment, evidence, None)?,
                MemoryCommand::Retire {
                    id,
                    experiment,
                    evidence,
                } => store.revise(id, "retired", experiment, evidence, None)?,
                MemoryCommand::Rollback {
                    id,
                    experiment,
                    version,
                    evidence,
                } => store.revise(id, "candidate", experiment, evidence, Some(*version))?,
            }
            return print(&store.memories(None)?);
        }
        _ => {}
    }
    let config = if let Some(path) = &cli.config {
        let text = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        toml::from_str::<Config>(&text).map_err(|e| e.to_string())?
    } else {
        Config::default()
    };
    let provider: Box<dyn ModelProvider> = if let Some(path) = &cli.script {
        Box::new(ScriptedProvider::new(read(path)?))
    } else {
        let endpoint = cli
            .endpoint
            .as_deref()
            .ok_or("set --endpoint or --script; live inference is never implicit")?;
        Box::new(OpenAiCompatible::new(
            endpoint,
            std::env::var("DREAMING_API_KEY").ok(),
            ProviderCapabilities::default(),
        )?)
    };
    let cancel = Cancellation::default();
    let signal = cancel.clone();
    let signal_task = tokio::spawn(async move {
        if tokio::signal::ctrl_c().await.is_ok() {
            signal.cancel();
        }
    });
    let (mode, task, case) = match cli.command {
        Command::Baseline { input } => (Mode::Baseline, Some(read::<Task>(&input)?), None),
        Command::Run {
            mode: RunMode::Lucid,
            case: Some(path),
            input: None,
        } => (Mode::Lucid, None, Some(read::<FailureCase>(&path)?)),
        Command::Run {
            mode: RunMode::Dream,
            input: Some(path),
            case: None,
        } => (Mode::Dream, Some(read::<Task>(&path)?), None),
        Command::Run {
            mode: RunMode::Diversity,
            input: Some(path),
            case: None,
        } => (Mode::Diversity, Some(read::<Task>(&path)?), None),
        _ => return Err("dream/diversity require --input; lucid requires --case".into()),
    };
    let result = Engine {
        provider: provider.as_ref(),
        config,
    }
    .run(mode, task.as_ref(), case.as_ref(), &mut store, cancel)
    .await;
    signal_task.abort();
    let run = result?;
    print(&run)?;
    if run.status != "completed" {
        return Err(format!("run {} ended {}", run.id, run.status));
    }
    Ok(())
}
