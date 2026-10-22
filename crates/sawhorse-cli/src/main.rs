//! CLI binary entrypoint for `sawhorse`.

use std::path::PathBuf;

use anyhow::Result;
use clap::{Parser, Subcommand, ValueEnum};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

mod commands;

use commands::complete::CompleteArgs;
use commands::{run_bench, run_complete, run_init, run_ping};
use sawhorse_core::config::SawhorseConfig;
use sawhorse_core::types::{GrammarFormat, ModelRole};

#[derive(Parser, Debug)]
#[command(name = "sawhorse", about = "Provider-agnostic 3-mode LLM runtime")]
struct Cli {
    /// Path to sawhorse.toml configuration file
    #[arg(short, long, global = true)]
    config: Option<PathBuf>,

    /// Enable verbose diagnostic debug logging
    #[arg(short, long, global = true)]
    verbose: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliRole {
    Planner,
    System1Router,
    HoleFiller,
    Oracle,
}

impl From<CliRole> for ModelRole {
    fn from(r: CliRole) -> Self {
        match r {
            CliRole::Planner => ModelRole::Planner,
            CliRole::System1Router => ModelRole::System1Router,
            CliRole::HoleFiller => ModelRole::HoleFiller,
            CliRole::Oracle => ModelRole::Oracle,
        }
    }
}

#[derive(ValueEnum, Clone, Copy, Debug)]
enum CliGrammarFormat {
    Gbnf,
    Lark,
    Regex,
}

impl From<CliGrammarFormat> for GrammarFormat {
    fn from(f: CliGrammarFormat) -> Self {
        match f {
            CliGrammarFormat::Gbnf => GrammarFormat::Gbnf,
            CliGrammarFormat::Lark => GrammarFormat::Lark,
            CliGrammarFormat::Regex => GrammarFormat::Regex,
        }
    }
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Scaffolds a starter `sawhorse.toml` file
    Init {
        /// Target path or directory
        #[arg(default_value = ".")]
        path: PathBuf,
        /// Overwrite existing file
        #[arg(short, long)]
        force: bool,
    },
    /// Pings configured providers and reports latency
    Ping {
        /// Specific provider to test
        #[arg(short, long)]
        provider: Option<String>,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
    /// Executes prompt completion across configured roles
    Complete {
        /// Prompt context or instruction
        prompt: String,
        /// Semantic model role
        #[arg(short, long, default_value = "hole-filler")]
        role: CliRole,
        /// Override provider directly
        #[arg(long)]
        provider: Option<String>,
        /// Path to grammar file or inline grammar string
        #[arg(short, long)]
        grammar: Option<String>,
        /// Grammar format notation
        #[arg(long, default_value = "gbnf")]
        grammar_format: CliGrammarFormat,
        /// System instruction
        #[arg(short, long)]
        system: Option<String>,
        /// Maximum generation tokens
        #[arg(short, long, default_value_t = 1024)]
        max_tokens: u16,
        /// Sampling temperature
        #[arg(short, long, default_value_t = 0.0)]
        temperature: f32,
        /// Stream output tokens
        #[arg(short, long)]
        stream: bool,
        /// Output result as JSON
        #[arg(long)]
        json: bool,
    },
    /// Runs comparative latency and throughput benchmarks
    Bench {
        /// Optional custom benchmark prompt
        #[arg(short, long)]
        prompt: Option<String>,
        /// Output results as JSON
        #[arg(long)]
        json: bool,
    },
}

#[tokio::main]
async fn main() -> Result<()> {
    let cli = Cli::parse();

    let log_level = if cli.verbose { "debug" } else { "warn" };
    tracing_subscriber::registry()
        .with(tracing_subscriber::fmt::layer().with_writer(std::io::stderr))
        .with(tracing_subscriber::EnvFilter::new(log_level))
        .init();

    if let Commands::Init { path, force } = &cli.command {
        return run_init(path, *force);
    }

    let config = if let Some(ref path) = cli.config {
        SawhorseConfig::load_from_path(path)?
    } else {
        SawhorseConfig::discover()?
    };

    match cli.command {
        Commands::Init { .. } => unreachable!(),
        Commands::Ping { provider, json } => run_ping(config, provider, json).await,
        Commands::Complete {
            prompt,
            role,
            provider,
            grammar,
            grammar_format,
            system,
            max_tokens,
            temperature,
            stream,
            json,
        } => {
            let args = CompleteArgs {
                prompt,
                role: role.into(),
                provider,
                grammar,
                grammar_format: Some(grammar_format.into()),
                system_prompt: system,
                max_tokens,
                temperature,
                stream,
                as_json: json,
            };
            run_complete(config, args).await
        }
        Commands::Bench { prompt, json } => run_bench(config, prompt, json).await,
    }
}

