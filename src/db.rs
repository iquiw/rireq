use std::cmp::{min, Ordering, Reverse};
use std::env;
use std::fs::{create_dir_all, File};
use std::io::{stdout, BufRead, BufReader};
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime, UNIX_EPOCH};

use libmdbx::{Database, Environment, NoWriteMap, Transaction, WriteFlags, RW};

use crate::record::{CmdData, CmdRecord};

const DB_VERSION: &str = "v1";

type Result<T> = std::result::Result<T, Box<dyn std::error::Error>>;

pub struct Db {
    env: Environment<NoWriteMap>,
}

impl Db {
    pub fn open() -> Result<Self> {
        let path = Self::db_path();
        if !path.exists() {
            create_dir_all(&path)?;
        }
        let env = Environment::new().open(&path)?;
        Ok(Db { env })
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
        let wtxn = self.env.begin_rw_txn()?;
        let db = wtxn.open_db(None)?;
        let mut count = 0;
        for line in reader.lines().flatten() {
            self.record_txn(&wtxn, &db, CmdRecord::new_epoch(line))?;
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
        let wtxn = self.env.begin_rw_txn()?;
        let db = wtxn.open_db(None)?;
        let mut count = 0;
        for result in reader.deserialize() {
            let cmdrec: CmdRecord = result?;
            self.record_txn(&wtxn, &db, cmdrec)?;
            count += 1;
        }
        wtxn.commit()?;
        println!("Imported {} history", count);
        Ok(())
    }

    pub fn record(&self, new_cmdrec: CmdRecord) -> Result<()> {
        let wtxn = self.env.begin_rw_txn()?;
        let db = wtxn.open_db(None)?;
        self.record_txn(&wtxn, &db, new_cmdrec)?;
        wtxn.commit()?;
        Ok(())
    }

    fn record_txn(
        &self,
        wtxn: &Transaction<'_, RW, NoWriteMap>,
        db: &Database<'_>,
        new_cmdrec: CmdRecord,
    ) -> Result<()> {
        if new_cmdrec.is_ignored() {
            return Ok(());
        }
        let wflags = WriteFlags::UPSERT;
        if let Some(cmd_data) = wtxn.get::<CmdData>(db, new_cmdrec.key().as_ref())? {
            let merged = cmd_data.merge(&new_cmdrec);
            let cmd_data = bincode::serialize(&merged)?;
            wtxn.put(db, new_cmdrec.key(), &cmd_data, wflags)?;
        } else {
            let cmd_data = bincode::serialize(&new_cmdrec.data())?;
            wtxn.put(db, new_cmdrec.key(), cmd_data, wflags)?;
        }
        Ok(())
    }

    pub fn stats(&self) -> Result<()> {
        let mut num_cmds = 0;
        let mut top_used_cmds = Vec::<(u64, String)>::new();
        let mut top_count = 0;
        let mut lr_used_cmd: Option<String> = None;
        let mut lr_used_time = UNIX_EPOCH;

        let rtxn = self.env.begin_ro_txn()?;
        let db = rtxn.open_db(None)?;
        for (key, data) in rtxn.cursor(&db)?.iter::<Vec<u8>, CmdData>().flatten() {
            let cmd = std::str::from_utf8(&key)?;
            let cmdrec = CmdRecord::new_with_data(cmd.into(), data);
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
            Self::db_path().display(),
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
        let rtxn = self.env.begin_ro_txn()?;
        let db = rtxn.open_db(None)?;
        let mut max_count = 0;
        let mut recs = rtxn
            .cursor(&db)?
            .iter::<Vec<u8>, CmdData>()
            .flatten()
            .map(|(k, d)| {
                let cmd = String::from_utf8_lossy(&k);
                let cmdrec = CmdRecord::new_with_data(cmd.into(), d);
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
