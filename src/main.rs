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
    history [--print0]
    import <FILE>
    import-csv <FILE>
    init bash
    prune
    record <COMMAND_LINE>
    stats
",
        env!("CARGO_BIN_NAME"),
        env!("CARGO_PKG_VERSION"),
        env!("CARGO_BIN_NAME"),
    );
}

fn main() {
    if let Err(e) = rireq() {
        eprintln!("{}", e);
    }
}

fn rireq() -> Result<(), Box<dyn std::error::Error>> {
    let mut args = env::args().skip(1);
    if let Some(ref subcmd) = args.next() {
        if subcmd == "export-csv" {
            let db = Db::open()?;
            return db.export_csv();
        } else if subcmd == "history" {
            let mut print0 = false;
            if let Some(option) = args.next() {
                if option == "--print0" {
                    print0 = true;
                } else {
                    eprintln!("Unknown history option: {}", option);
                    exit(1);
                }
            }
            let db = Db::open()?;
            return db.history(print0);
        } else if subcmd == "import" {
            if let Some(file) = args.next() {
                let db = Db::open()?;
                return db.import(&file);
            }
        } else if subcmd == "import-csv" {
            if let Some(file) = args.next() {
                let db = Db::open()?;
                return db.import_csv(&file);
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
        } else if subcmd == "prune" {
            let mut older = false;
            if let Some(option) = args.next() {
                if option == "--older" {
                    older = true;
                } else {
                    eprintln!("Unknown prune option: {}", option);
                    exit(1);
                }
            }
            let db = Db::open()?;
            return db.prune(older);
        } else if subcmd == "record" {
            if let Some(cmdline) = args.next() {
                let db = Db::open()?;
                db.record(CmdRecord::new(cmdline))?;
            }
            return Ok(());
        } else if subcmd == "stats" {
            let db = Db::open()?;
            return db.stats();
        }
    }
    usage();
    exit(1);
}
