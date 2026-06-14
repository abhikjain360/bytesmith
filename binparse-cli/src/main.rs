use std::fs;
use std::io::{self, Read, Write};
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::Parser;

#[derive(Parser)]
#[command(name = "binparse-cli", version, about = "Compile binparse DSL specs into Rust source")]
struct Cli {
    #[arg(required = true, value_name = "FILE", help = "`.bp` spec files to compile (use `-` for stdin)")]
    inputs: Vec<PathBuf>,

    #[arg(short, long, value_name = "FILE", help = "Write generated code here instead of stdout (single input only)")]
    output: Option<PathBuf>,

    #[arg(long = "no-writers", help = "Only emit parsers; skip the zero-copy writers")]
    no_writers: bool,
}

fn main() -> ExitCode {
    let cli = Cli::parse();

    if cli.output.is_some() && cli.inputs.len() > 1 {
        eprintln!("error: --output requires a single input file; got {}", cli.inputs.len());
        return ExitCode::FAILURE;
    }

    let mut out: Box<dyn Write> = match &cli.output {
        Some(path) => match fs::File::create(path) {
            Ok(file) => Box::new(file),
            Err(e) => {
                eprintln!("error: cannot create {}: {e}", path.display());
                return ExitCode::FAILURE;
            }
        },
        None => Box::new(io::stdout().lock()),
    };

    for input in &cli.inputs {
        let src = match read_input(input) {
            Ok(src) => src,
            Err(e) => {
                eprintln!("error: cannot read {}: {e}", display_path(input));
                return ExitCode::FAILURE;
            }
        };

        let ast = match binparse_dsl_parse::parse_str(&src) {
            Ok(ast) => ast,
            Err(report) => {
                eprint!("{report}");
                return ExitCode::FAILURE;
            }
        };

        let code = if cli.no_writers {
            binparse_codegen::CodeGen::generate(&ast)
        } else {
            binparse_codegen::CodeGen::generate_writers(&ast)
        };
        let code = match code {
            Ok(code) => code,
            Err(e) => {
                eprintln!("error: codegen failed for {}: {e}", display_path(input));
                return ExitCode::FAILURE;
            }
        };

        if let Err(e) = out.write_all(code.as_bytes()) {
            eprintln!("error: failed to write output: {e}");
            return ExitCode::FAILURE;
        }
    }

    ExitCode::SUCCESS
}

fn read_input(path: &Path) -> io::Result<String> {
    if path.as_os_str() == "-" {
        let mut buf = String::new();
        io::stdin().read_to_string(&mut buf)?;
        Ok(buf)
    } else {
        fs::read_to_string(path)
    }
}

fn display_path(path: &Path) -> String {
    if path.as_os_str() == "-" {
        "<stdin>".to_string()
    } else {
        path.display().to_string()
    }
}
