use std::cmp::{min, Ordering, Reverse};
use std::env;
use std::fs::{create_dir_all, File};
use std::io::{stdout, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use heed::types::{SerdeBincode, Str};
use heed::{Database, Env, EnvOpenOptions, Result, RwTxn};

use crate::record::{CmdData, CmdRecord};

const DB_VERSION: &str = "v1";

pub struct Db {
    db: Database<Str, SerdeBincode<CmdData>>,
    env: Env,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = Db::db_path();
        if !path.exists() {
            create_dir_all(&path)?;
        }
        let env = EnvOpenOptions::new().open(&path)?;
        let db: Database<Str, SerdeBincode<CmdData>> = env.create_database(None)?;
        Ok(Db { db, env })
    }

    pub fn export_csv(&self) -> std::result::Result<(), Box<dyn std::error::Error>> {
        let mut writer = csv::Writer::from_writer(stdout());
        for cmdrec in self.sorted_history()? {
            writer.serialize(cmdrec)?;
        }
        Ok(())
    }

    pub fn history(&self, print0: bool) -> Result<()> {
        for cmdrec in self.sorted_history()? {
            if print0 {
                print!("{}\0", cmdrec.cmdline());
            } else {
                println!("{}", cmdrec.cmdline());
            }
        }
        Ok(())
    }

    pub fn import<P>(&self, path: &P) -> Result<()>
    where
        P: AsRef<Path>,
    {
        let f = File::open(path)?;
        let reader = BufReader::new(f);
        let mut wtxn = self.env.write_txn()?;
        let mut count = 0;
        for line in reader.lines().flatten() {
            self.record_txn(&mut wtxn, CmdRecord::new_epoch(line))?;
            count += 1;
        }
        wtxn.commit()?;
        println!("Imported {} history", count);
        Ok(())
    }

    pub fn import_csv<P>(&self, path: &P) -> std::result::Result<(), Box<dyn std::error::Error>>
    where
        P: AsRef<Path>,
    {
        let mut reader = csv::Reader::from_path(path)?;
        let mut wtxn = self.env.write_txn()?;
        let mut count = 0;
        for result in reader.deserialize() {
            let cmdrec: CmdRecord = result?;
            self.record_txn(&mut wtxn, cmdrec)?;
            count += 1;
        }
        wtxn.commit()?;
        println!("Imported {} history", count);
        Ok(())
    }

    pub fn record(&self, new_cmdrec: CmdRecord) -> Result<()> {
        let mut wtxn = self.env.write_txn()?;
        self.record_txn(&mut wtxn, new_cmdrec)?;
        wtxn.commit()?;
        Ok(())
    }

    fn record_txn(&self, wtxn: &mut RwTxn, new_cmdrec: CmdRecord) -> Result<()> {
        if new_cmdrec.is_ignored() {
            return Ok(());
        }
        if let Some(cmd_data) = self.db.get(wtxn, new_cmdrec.key())? {
            let merged = cmd_data.merge(&new_cmdrec);
            self.db.put(wtxn, new_cmdrec.key(), &merged)?;
        } else {
            self.db.put(wtxn, new_cmdrec.key(), new_cmdrec.data())?;
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<()> {
        let mut num_cmds = 0;
        let mut top_used_cmds = Vec::<(u64, String)>::new();
        let mut top_count = 0;
        let mut lr_used_cmd: Option<String> = None;
        let mut lr_used_time = UNIX_EPOCH;

        let rtxn = self.env.read_txn()?;
        for (key, data) in self.db.iter(&rtxn)?.flatten() {
            let cmdrec = CmdRecord::new_with_data(key.into(), data);
            num_cmds += 1;
            if cmdrec.count().cmp(&top_count) == Ordering::Greater {
                if top_used_cmds.len() >= 5 {
                    top_used_cmds.sort_by(|x, y| y.0.cmp(&x.0));
                    top_used_cmds.pop();
                }
                top_used_cmds.push((cmdrec.count(), cmdrec.cmdline().into()));
                top_count = min(
                    cmdrec.count(),
                    top_used_cmds
                        .first()
                        .map(|x| x.0)
                        .unwrap_or_else(|| cmdrec.count()),
                );
            }
            if lr_used_time == UNIX_EPOCH || cmdrec.last_exec_time() < lr_used_time {
                lr_used_time = cmdrec.last_exec_time();
                lr_used_cmd = Some(cmdrec.cmdline().into())
            }
        }
        println!(
            "Rireq Command History Stats
DB path: {}

Number of commands     : {}
",
            Db::db_path().display(),
            num_cmds
        );
        println!(
            "Top {} used commands:
   count | command",
            top_used_cmds.len()
        );
        top_used_cmds.sort_by(|x, y| y.0.cmp(&x.0));
        for (count, cmd) in top_used_cmds {
            println!("   {:5} | {}", count, cmd);
        }

        println!(
            "
Least recently used command  : {}
Least recently used time     : {} sec(s) ago",
            lr_used_cmd.unwrap_or_else(|| "N/A".into()),
            if lr_used_time == SystemTime::UNIX_EPOCH {
                0
            } else {
                SystemTime::now()
                    .duration_since(lr_used_time)
                    .unwrap_or_else(|_| Duration::from_secs(0))
                    .as_secs()
            },
        );
        Ok(())
    }

    fn sorted_history(&self) -> Result<Vec<CmdRecord>> {
        let rtxn = self.env.read_txn()?;
        let mut max_count = 0;
        let mut recs = self
            .db
            .iter(&rtxn)?
            .flatten()
            .map(|(k, d)| {
                let cmdrec = CmdRecord::new_with_data(k.into(), d);
                if cmdrec.count() > max_count {
                    max_count = cmdrec.count();
                }
                cmdrec
            })
            .collect::<Vec<CmdRecord>>();
        let time = SystemTime::now();
        recs.sort_by_key(|a| Reverse(a.rank(max_count, &time))); // descending order
        Ok(recs)
    }

    #[cfg(windows)]
    fn db_path() -> PathBuf {
        let local_app_data = env::var("LOCALAPPDATA").expect("LOCALAPPDATA is set");
        let mut path = PathBuf::from(local_app_data);
        path.push("rireq");
        path.push("db");
        path.push(DB_VERSION);
        path
    }

    #[cfg(not(windows))]
    fn db_path() -> PathBuf {
        let home = env::var("HOME").expect("HOME is set");
        let mut path = PathBuf::from(home);
        path.push(".local");
        path.push("share");
        path.push("rireq");
        path.push("db");
        path.push(DB_VERSION);
        path
    }
}
