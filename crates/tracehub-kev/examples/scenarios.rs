//! 試験用の合成データ(kevcsv の CSV)をディレクトリに書く。
//!
//!     cargo run -p tracehub-kev --example scenarios -- <出力ディレクトリ>

fn main() {
    let dir = std::env::args().nth(1).expect("usage: scenarios <OUTPUT_DIR>");
    std::fs::create_dir_all(&dir).expect("cannot create the output directory");
    for (name, csv) in tracehub_kev::qnx800::scenarios::scenarios() {
        let path = std::path::Path::new(&dir).join(format!("{name}.csv"));
        std::fs::write(&path, csv).expect("cannot write a scenario file");
        println!("{}", path.display());
    }
}
