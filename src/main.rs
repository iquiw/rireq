use std::env;
use std::process::exit;

mod db;
mod record;

use db::Db;
use record::CmdRecord;

fn usage() {
    eprintln!(
        "{} {}
USAGE:
    {} <SUBCOMMAND>

SUBCOMMANDS:
    record <COMMAND_LINE>
    history
    stats
",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_BIN_NAME"),
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    if let Some(ref subcmd) = args.next() {
        if subcmd == "history" {
            let db = Db::open()?;
            db.history()?;
        } else if subcmd == "record" {
            if let Some(cmdline) = args.next() {
                let db = Db::open()?;
                db.record(CmdRecord::new(cmdline))?;
            }
        } else if subcmd == "stats" {
            let db = Db::open()?;
            db.stats()?;
        } else {
            usage();
            exit(1);
        }
    } else {
        usage();
        exit(1);
    }
    Ok(())
}
