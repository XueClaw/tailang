use clap::{Parser, Subcommand};
use std::fs;

#[derive(Parser)]
#[command(name = "tailangc")]
#[command(about = "Tailang compiler CLI")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    Compile {
        #[arg(long)]
        input: String,
        #[arg(long)]
        output: String,
    },
    PrintRust {
        #[arg(long)]
        input: String,
    },
}

fn main() {
    if let Err(err) = run() {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

fn run() -> Result<(), String> {
    let cli = Cli::parse();
    match cli.command {
        Commands::Compile { input, output } => {
            let content = fs::read_to_string(&input)
                .map_err(|err| format!("读取输入文件失败：{err}"))?;
            if looks_like_legacy_tai_json(&content) {
                tailang_compiler::compile_tai_snapshot_to_executable(&content, &output)
            } else {
                tailang_compiler::compile_tai_source_to_executable(&content, &output)
            }
        }
        Commands::PrintRust { input } => {
            let content = fs::read_to_string(&input)
                .map_err(|err| format!("读取输入文件失败：{err}"))?;
            let output = if looks_like_legacy_tai_json(&content) {
                tailang_compiler::compile_tai_snapshot_to_rust_source(&content)?
            } else {
                tailang_compiler::compile_tai_source_to_rust_source(&content)?
            };
            println!("{output}");
            Ok(())
        }
    }
}

fn looks_like_legacy_tai_json(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{')
        && (trimmed.contains("\"modules\"") || trimmed.contains("\"code_blocks\""))
}
