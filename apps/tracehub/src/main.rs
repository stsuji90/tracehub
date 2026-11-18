//! tracehub コマンド
//!
//!     tracehub traceprinter <kev ファイル> <出力ファイル>

use std::io::BufWriter;
use std::process::ExitCode;

const USAGE: &str = "usage: tracehub traceprinter <KEV_FILE> <OUTPUT_FILE>";

/// ライブラリの警告などを標準エラーに書く
struct StderrLogger;

impl log::Log for StderrLogger {
    fn enabled(&self, m: &log::Metadata) -> bool {
        m.level() <= log::Level::Warn
    }
    fn log(&self, r: &log::Record) {
        if self.enabled(r.metadata()) {
            let level = match r.level() {
                log::Level::Error => "error",
                _ => "warning",
            };
            eprintln!("{level}: {}", r.args());
        }
    }
    fn flush(&self) {}
}

static LOGGER: StderrLogger = StderrLogger;

fn main() -> ExitCode {
    let _ = log::set_logger(&LOGGER).map(|()| log::set_max_level(log::LevelFilter::Warn));
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
    let bytes = std::fs::read(input).map_err(|e| format!("cannot read {input}: {e}"))?;
    let kev = tracehub_kev::parse(&bytes).map_err(|e| format!("{input}: {e}"))?;
    let file = std::fs::File::create(output).map_err(|e| format!("cannot create {output}: {e}"))?;
    let mut out = BufWriter::new(file);
    tracehub_kev::traceprinter::write(&kev, &mut out).map_err(|e| format!("cannot write {output}: {e}"))
}
