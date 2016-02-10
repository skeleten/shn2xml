extern crate shn;
extern crate xml;
extern crate docopt;

use docopt::*;

#[allow(dead_code)]
const USAGE: &'static str = "
Usage: shn2xml [--encoding=<enc>] [--stdin | <input>] [--stdout | <output>]

Options:
    --encoding=<enc>        Sets the encoding
    --stdin                 Sets input to be stdin
    --stdout                Sets output to stdout
";

fn main() {
    let args = Docopt::new(USAGE)
                        .and_then(|d| d.argv(std::env::args()).parse())
                        .unwrap_or_else(|e| e.exit());

    let input: Box<Read> = if args.get_bool("--stdin") {
        Box::new(std::env::stdin())
    } else {
        panic!();
    }
}
