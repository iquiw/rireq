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
    history
    import <FILE>
    init bash
    record <COMMAND_LINE>
    stats
",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_BIN_NAME"),
    );
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    rireq()
}

fn rireq() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    if let Some(ref subcmd) = args.next() {
        if subcmd == "history" {
            let db = Db::open()?;
            db.history()?;
            return Ok(());
        } else if subcmd == "import" {
            if let Some(file) = args.next() {
                let db = Db::open()?;
                db.import(&file)?;
                return Ok(());
            }
        } else if subcmd == "init" {
            if let Some(shell) = args.next() {
                if shell == "bash" {
                    println!("{}", include_str!("../script/init.bash"));
                    return Ok(());
                } else {
                    eprintln!("Unknown shell: {} (only \"bash\" supported)", shell);
                    return Ok(());
                }
            }
        } else if subcmd == "record" {
            if let Some(cmdline) = args.next() {
                let db = Db::open()?;
                db.record(CmdRecord::new(cmdline))?;
            }
            return Ok(());
        } else if subcmd == "stats" {
            let db = Db::open()?;
            db.stats()?;
            return Ok(())
        }
    }
    usage();
    exit(1);
}
