use std::time::{Duration, SystemTime, UNIX_EPOCH};

use serde::ser::SerializeStruct;
use serde::{Deserialize, Serialize, Serializer};

#[derive(Serialize, Deserialize, Debug, PartialEq)]
pub struct CmdData {
    count: u64,
    last_exec_time: u64,
}

#[derive(Deserialize, Debug)]
pub struct CmdRecord {
    cmdline: String,
    #[serde(default, flatten)]
    cmd_data: CmdData,
}

// Deriving Serialize does not work with csv crate.
impl Serialize for CmdRecord {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        let mut state = serializer.serialize_struct("CmdRecord", 3)?;
        state.serialize_field("cmdline", &self.cmdline)?;
        state.serialize_field("count", &self.cmd_data.count)?;
        state.serialize_field("last_exec_time", &self.cmd_data.last_exec_time)?;
        state.end()
    }
}

impl CmdData {
    pub fn merge(&self, other: &CmdRecord) -> CmdData {
        CmdData {
            count: self.count + other.cmd_data.count,
            last_exec_time: if self.last_exec_time < other.cmd_data.last_exec_time {
                other.cmd_data.last_exec_time
            } else {
                self.last_exec_time
            },
        }
    }
}

impl CmdRecord {
    pub fn new(cmdline: String) -> Self {
        CmdRecord {
            cmdline,
            cmd_data: CmdData {
                count: 1,
                last_exec_time: time_to_u64(&SystemTime::now()),
            },
        }
    }

    pub fn new_epoch(cmdline: String) -> Self {
        CmdRecord {
            cmdline,
            cmd_data: CmdData {
                count: 1,
                last_exec_time: time_to_u64(&UNIX_EPOCH),
            },
        }
    }

    pub fn new_with_data(cmdline: String, cmd_data: CmdData) -> Self {
        CmdRecord { cmdline, cmd_data }
    }

    pub fn cmdline(&self) -> &str {
        &self.cmdline
    }

    pub fn count(&self) -> u64 {
        self.cmd_data.count
    }

    pub fn last_exec_time(&self) -> SystemTime {
        UNIX_EPOCH + Duration::from_secs(self.cmd_data.last_exec_time)
    }

    pub fn key(&self) -> &str {
        normalize(&self.cmdline)
    }

    pub fn data(&self) -> &CmdData {
        &self.cmd_data
    }

    pub fn is_ignored(&self) -> bool {
        self.key().is_empty()
    }

    pub fn rank(&self, max: u64, time: &SystemTime) -> u64 {
        let secs = time
            .duration_since(self.last_exec_time())
            .map(|d| d.as_secs())
            .unwrap_or(0_u64);

        let t = (secs.saturating_add(1) as f64).log(86400_f64);
        if t > 0_f64 {
            let c = self.count();
            ((max as f64 / t) as u64).saturating_add(c)
        } else {
            u64::MAX
        }
    }
}

fn time_to_u64(time: &SystemTime) -> u64 {
    time.duration_since(UNIX_EPOCH)
        .unwrap_or_else(|_| Duration::from_secs(0))
        .as_secs()
}

fn normalize(cmdline: &str) -> &str {
    cmdline.trim()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[test]
    fn cmddata_merge() {
        let cmddata1 = CmdData {
            count: 1,
            last_exec_time: 0,
        };
        let cmddata2 = CmdData {
            count: 2,
            last_exec_time: 2000000000,
        };
        let cmddata3 = CmdData {
            count: 3,
            last_exec_time: 1000000000,
        };
        let cmdrec = CmdRecord::new_with_data("".into(), cmddata2);
        assert_eq!(
            cmddata1.merge(&cmdrec),
            CmdData {
                count: 3,
                last_exec_time: 2000000000
            }
        );
        assert_eq!(
            cmddata3.merge(&cmdrec),
            CmdData {
                count: 5,
                last_exec_time: 2000000000
            }
        );
    }

    #[test]
    fn cmdrecord_is_ignored() {
        let cmdrec_empty = CmdRecord::new("".into());
        let cmdrec_spaces = CmdRecord::new("   ".into());
        let cmdrec_nonempty = CmdRecord::new("foo bar".into());

        assert!(cmdrec_empty.is_ignored());
        assert!(cmdrec_spaces.is_ignored());
        assert!(!cmdrec_nonempty.is_ignored());
    }

    #[test]
    fn cmdrecord_rank_no_time_diff() {
        let mut cmdrec = CmdRecord::new("".into());
        assert_eq!(cmdrec.rank(1000, &cmdrec.last_exec_time()), u64::MAX);

        cmdrec.cmd_data.count = 200;
        assert_eq!(cmdrec.rank(1000, &cmdrec.last_exec_time()), u64::MAX);
    }

    #[test]
    fn cmdrecord_rank_compare() {
        let cmdrec_1count = CmdRecord::new("".into());
        let time_1sec = cmdrec_1count.last_exec_time() + Duration::from_secs(1);
        let time_1day = cmdrec_1count.last_exec_time() + Duration::from_secs(86400);
        let time_2day = cmdrec_1count.last_exec_time() + Duration::from_secs(86400 * 2);

        let mut cmdrec_1000count = CmdRecord::new("".into());
        cmdrec_1000count.cmd_data.count = 1000;
        cmdrec_1000count.cmd_data.last_exec_time = cmdrec_1count.cmd_data.last_exec_time;
        assert!(cmdrec_1count.rank(1000, &time_1sec) > cmdrec_1000count.rank(1000, &time_1day));

        let mut cmdrec_2000count = CmdRecord::new("".into());
        cmdrec_2000count.cmd_data.count = 2000;
        assert!(cmdrec_2000count.rank(2000, &time_2day) > cmdrec_1000count.rank(2000, &time_1day));
    }

    #[test]
    fn normalize_without_space() {
        assert_eq!(normalize("ls"), "ls");
        assert_eq!(normalize("cat foo.txt"), "cat foo.txt");
    }

    #[test]
    fn normalize_with_prefix_and_suffix_spaces() {
        assert_eq!(normalize(" 	cp foo.txt bar.txt"), "cp foo.txt bar.txt");
        assert_eq!(
            normalize("foo 2>&1 | tee foo.txt "),
            "foo 2>&1 | tee foo.txt"
        );
        assert_eq!(
            normalize(" ls  -l   foo bar  baz   "),
            "ls  -l   foo bar  baz"
        );
    }
}
