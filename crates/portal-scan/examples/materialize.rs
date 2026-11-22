//! Builds a scanner fixture in a folder (for the design lab's demo reports):
//!   cargo run -p portal-scan --example materialize -- <fixture> <out>
#[path = "../tests/common/fixture.rs"]
mod fixture;

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    let [src, dst] = args.as_slice() else {
        eprintln!("usage: materialize <fixture> <out>");
        std::process::exit(2);
    };
    match fixture::materialize(std::path::Path::new(src), std::path::Path::new(dst)) {
        Ok(true) => {}
        Ok(false) => eprintln!("this fixture can't be built on this OS"),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}
