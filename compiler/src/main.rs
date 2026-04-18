use clap::{Parser, Subcommand};
use std::fs;
use tailang_compiler::{CompileOptions, CompilerBackend, OptimizationLevel};

#[derive(Parser)]
#[command(name = "tailangc")]
#[command(about = "Tailang native compiler CLI")]
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
        #[arg(long, default_value = "self-native")]
        backend: String,
        #[arg(long = "opt-level", default_value = "1")]
        opt_level: String,
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
        Commands::Compile {
            input,
            output,
            backend,
            opt_level,
        } => {
            let raw = fs::read(&input)
                .map_err(|err| format!("读取输入文件失败：{err}"))?;
            let content = decode_utf8_source(&raw)?;
            let options = CompileOptions {
                backend: backend.parse::<CompilerBackend>()?,
                opt_level: opt_level.parse::<OptimizationLevel>()?,
            };
            if looks_like_legacy_tai_json(&content) {
                tailang_compiler::compile_tai_snapshot_to_executable_with_options(
                    &content,
                    &output,
                    options,
                )
            } else {
                tailang_compiler::compile_tai_source_to_executable_with_options(
                    &content,
                    &output,
                    options,
                )
            }
        }
    }
}

fn looks_like_legacy_tai_json(content: &str) -> bool {
    let trimmed = content.trim_start();
    trimmed.starts_with('{')
        && (trimmed.contains("\"modules\"") || trimmed.contains("\"code_blocks\""))
}

fn decode_utf8_source(raw: &[u8]) -> Result<String, String> {
    if raw.starts_with(&[0xFF, 0xFE]) || raw.starts_with(&[0xFE, 0xFF]) {
        return Err("输入文件必须是 UTF-8，禁止使用 UTF-16".to_string());
    }

    let text = std::str::from_utf8(raw)
        .map_err(|_| "输入文件必须是 UTF-8，禁止使用 GBK/ANSI/UTF-16 等其他编码".to_string())?;
    Ok(text.to_string())
}
