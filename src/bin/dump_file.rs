extern crate ldraw;

use ldraw::load_ldraw;
use std::env;

fn main() {
    let args = env::args();
    assert!(args.len() == 2);
    let file_path: String = args.last().unwrap();

    match load_ldraw(&file_path) {
        Err(parse_errors) => {
            for error in parse_errors {
                println!("{}:{}: {}",
                         file_path,
                         error.line_number,
                         error.error);
            }
        },
        Ok(file) => println!("{:?}\n", file),
    }
}
