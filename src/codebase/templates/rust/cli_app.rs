// @LITE_DESC Rust CLI application with clap derive API, file I/O operations, anyhow error handling, and colored terminal output
// @LITE_SCENE A complete CLI tool template demonstrating argument parsing, file processing, error handling, and user-friendly colored output
// @LITE_TAGS rust, cli, clap, tool, terminal

use anyhow::{Context, Result};
use clap::Parser;
use colored::Colorize;
use std::fs;
use std::path::PathBuf;

/// A simple CLI tool for processing text files
#[derive(Parser, Debug)]
#[command(name = "cli-tool")]
#[command(about = "A Rust CLI tool for text file processing", long_about = None)]
struct Args {
    /// Input file path to process
    #[arg(short, long)]
    input: PathBuf,

    /// Output file path (optional, prints to stdout if not provided)
    #[arg(short, long)]
    output: Option<PathBuf>,

    /// Enable verbose output
    #[arg(short, long)]
    verbose: bool,

    /// Transform text to uppercase
    #[arg(long)]
    uppercase: bool,

    /// Transform text to lowercase
    #[arg(long)]
    lowercase: bool,

    /// Count lines, words, and characters
    #[arg(long)]
    count: bool,
}

fn main() -> Result<()> {
    let args = Args::parse();

    if args.verbose {
        println!("{}", "Starting CLI tool...".green().bold());
    }

    // Read input file
    let content = fs::read_to_string(&args.input)
        .with_context(|| format!("Failed to read file: {}", args.input.display()))?;

    if args.verbose {
        println!(
            "{}",
            format!("Successfully read {} bytes", content.len()).cyan()
        );
    }

    // Process content based on flags
    let processed = if args.uppercase {
        content.to_uppercase()
    } else if args.lowercase {
        content.to_lowercase()
    } else {
        content
    };

    // Output results
    if args.count {
        print_stats(&processed);
    }

    match args.output {
        Some(output_path) => {
            fs::write(&output_path, processed)
                .with_context(|| format!("Failed to write to file: {}", output_path.display()))?;
            println!(
                "{}",
                format!("Output written to: {}", output_path.display()).green()
            );
        }
        None => {
            println!("{}", "\n--- Output ---".yellow());
            println!("{}", processed);
        }
    }

    Ok(())
}

fn print_stats(content: &str) {
    let lines = content.lines().count();
    let words = content.split_whitespace().count();
    let chars = content.chars().count();

    println!("{}", "\n--- Statistics ---".blue().bold());
    println!("  {}: {}", "Lines".cyan(), lines.to_string().white());
    println!("  {}: {}", "Words".cyan(), words.to_string().white());
    println!("  {}: {}", "Characters".cyan(), chars.to_string().white());
}
