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
    export-csv
    history
    import <FILE>
    import-csv <FILE>
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
        if subcmd == "export-csv" {
            let db = Db::open()?;
            return Ok(db.export_csv()?);

        } else if subcmd == "history" {
            let db = Db::open()?;
            return Ok(db.history()?);

        } else if subcmd == "import" {
            if let Some(file) = args.next() {
                let db = Db::open()?;
                return Ok(db.import(&file)?);
            }
        } else if subcmd == "import-csv" {
            if let Some(file) = args.next() {
                let db = Db::open()?;
                return Ok(db.import_csv(&file)?);
            }
        } else if subcmd == "init" {
            if let Some(shell) = args.next() {
                if shell == "bash" {
                    println!("{}", include_str!("../script/init.bash"));
                    return Ok(());
                } else {
                    eprintln!("Unknown shell: {} (only \"bash\" supported)", shell);
                    exit(1);
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
            return Ok(db.stats()?);
        }
    }
    usage();
    exit(1);
}
