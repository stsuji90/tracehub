//! tracehub コマンド
//!
//!     tracehub traceprinter <kev ファイル> <出力ファイル>

use std::io::BufWriter;
use std::process::ExitCode;

const USAGE: &str = "使い方: tracehub traceprinter <kev ファイル> <出力ファイル>";

fn main() -> ExitCode {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let result = match args.as_slice() {
        [cmd, input, output] if cmd == "traceprinter" => traceprinter(input, output),
        _ => Err(USAGE.to_string()),
    };
    match result {
        Ok(()) => ExitCode::SUCCESS,
        Err(e) => {
            eprintln!("{e}");
            ExitCode::FAILURE
        }
    }
}

/// traceprinter 互換出力(traceprinter -n と同じ)
fn traceprinter(input: &str, output: &str) -> Result<(), String> {
    let bytes = std::fs::read(input).map_err(|e| format!("{input} を読めません: {e}"))?;
    let kev = tracehub_kev::parse(&bytes).map_err(|e| format!("{input}: {e}"))?;
    let file = std::fs::File::create(output).map_err(|e| format!("{output} を作れません: {e}"))?;
    let mut out = BufWriter::new(file);
    tracehub_kev::traceprinter::write(&kev, &mut out).map_err(|e| format!("{output} に書けません: {e}"))
}
