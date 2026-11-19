//! 試験・確認用の道具(tests/kev/tools/ の compare.py・event_catalog.py が使う)。
//!
//!     cargo run -p tracehub-kev --example test_support -- scenarios <出力ディレクトリ>
//!     cargo run -p tracehub-kev --example test_support -- catalog [実機の kev ...]
//!
//! scenarios: 試験用の合成データ(kevcsv の CSV)をディレクトリに書く。
//!
//! catalog: 確認用のイベント一覧を TSV で標準出力に書く。列: class_id, class, event_id, event, int_class, int_event, display_class, display_name,
//!     fast, wide, fast_labels, wide_labels, synthetic, synthetic_displayed, example_fast, example_wide,
//!     real, real_displayed(実機の kev の件数の合計)
//! 実機の kev で読めないものは標準エラーに書いて飛ばす。

use std::collections::BTreeMap;

use tracehub_kev::qnx800::test_support::{catalog, scenarios};

fn cell(s: &str) -> String {
    s.chars()
        .map(|c| match c {
            '\t' => r"\t".to_string(),
            '\n' => r"\n".to_string(),
            '\r' => r"\r".to_string(),
            c if (c as u32) < 0x20 || (0x7f..=0xff).contains(&(c as u32)) => format!(r"\x{:02x}", c as u32),
            c => c.to_string(),
        })
        .collect()
}

const USAGE: &str = "usage: test_support scenarios <OUTPUT_DIR> | test_support catalog [KEV ...]";

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    match args.first().map(String::as_str) {
        Some("scenarios") if args.len() == 2 => write_scenarios(&args[1]),
        Some("catalog") => write_catalog(&args[1..]),
        _ => {
            eprintln!("{USAGE}");
            std::process::exit(2);
        }
    }
}

fn write_scenarios(dir: &str) {
    std::fs::create_dir_all(dir).expect("cannot create the output directory");
    for (name, csv) in scenarios::scenarios() {
        let path = std::path::Path::new(dir).join(format!("{name}.csv"));
        std::fs::write(&path, csv).expect("cannot write a scenario file");
        println!("{}", path.display());
    }
}

fn write_catalog(paths: &[String]) {
    let mut real: BTreeMap<_, (usize, usize)> = BTreeMap::new();
    for path in paths {
        let kev = match std::fs::read(path).map(|b| tracehub_kev::parse(&b)) {
            Ok(Ok(k)) => k,
            Ok(Err(e)) => {
                eprintln!("{path}: {e}");
                continue;
            }
            Err(e) => {
                eprintln!("{path}: {e}");
                continue;
            }
        };
        for (k, (n, d)) in catalog::count(&kev) {
            let e = real.entry(k).or_default();
            e.0 += n;
            e.1 += d;
        }
    }
    println!(
        "class_id\tclass\tevent_id\tevent\tint_class\tint_event\tdisplay_class\tdisplay_name\tfast\twide\tfast_labels\twide_labels\tsynthetic\tsynthetic_displayed\texample_fast\texample_wide\treal\treal_displayed"
    );
    for e in catalog::entries() {
        let (rn, rd) = real.get(&(e.class_id, e.event_id)).copied().unwrap_or_default();
        let cols = [
            e.class_id.0.to_string(),
            e.class.to_string(),
            e.event_id.0.to_string(),
            e.event.clone(),
            e.int_class.to_string(),
            format!("0x{:03x}", e.int_event),
            e.display_class.clone(),
            e.display_name.clone(),
            e.fast.clone(),
            e.wide.clone(),
            e.fast_labels.join(","),
            e.wide_labels.join(","),
            e.synthetic.to_string(),
            e.synthetic_displayed.to_string(),
            e.example_fast.clone(),
            e.example_wide.clone(),
            rn.to_string(),
            rd.to_string(),
        ];
        println!("{}", cols.iter().map(|c| cell(c)).collect::<Vec<_>>().join("\t"));
    }
}
