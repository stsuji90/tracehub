//! 試験用の合成データ(kevcsv の CSV)をディレクトリに書く。
//!
//!     cargo run -p tracehub-kev --example scenarios -- <出力ディレクトリ>

fn main() {
    let dir = std::env::args().nth(1).expect("出力ディレクトリを指定してください");
    std::fs::create_dir_all(&dir).expect("ディレクトリを作れません");
    for (name, csv) in tracehub_kev::qnx800::scenarios::scenarios() {
        let path = std::path::Path::new(&dir).join(format!("{name}.csv"));
        std::fs::write(&path, csv).expect("書けません");
        println!("{}", path.display());
    }
}
