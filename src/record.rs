use std::time::SystemTime;

use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug)]
pub struct CmdData {
    count: u64,
    last_exec_time: SystemTime,
}

#[derive(Debug)]
pub struct CmdRecord {
    cmdline: String,
    cmd_data: CmdData,
}

impl CmdData {
    pub fn update(&mut self, time: &SystemTime) {
        self.count += 1;
        self.last_exec_time = *time;
    }
}

impl CmdRecord {
    pub fn new(cmdline: String) -> Self {
        CmdRecord {
            cmdline,
            cmd_data: CmdData {
                count: 1,
                last_exec_time: SystemTime::now(),
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

    pub fn last_exec_time(&self) -> &SystemTime {
        &self.cmd_data.last_exec_time
    }

    pub fn key(&self) -> &str {
        normalize(&self.cmdline)
    }

    pub fn data(&self) -> &CmdData {
        &self.cmd_data
    }

    pub fn rank(&self, max: u64, time: &SystemTime) -> u64 {
        let secs = time
            .duration_since(*self.last_exec_time())
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

fn normalize(cmdline: &str) -> &str {
    cmdline.trim()
}

#[cfg(test)]
mod test {
    use super::*;
    use std::time::Duration;

    #[test]
    fn cmdrecord_rank_no_time_diff() {
        let mut cmdrec = CmdRecord::new("".into());
        assert_eq!(cmdrec.rank(1000, cmdrec.last_exec_time()), u64::MAX);

        cmdrec.cmd_data.count = 200;
        assert_eq!(cmdrec.rank(1000, cmdrec.last_exec_time()), u64::MAX);
    }

    #[test]
    fn cmdrecord_rank_compare() {
        let cmdrec_1count = CmdRecord::new("".into());
        let time_1sec = *cmdrec_1count.last_exec_time() + Duration::from_secs(1);
        let time_1day = *cmdrec_1count.last_exec_time() + Duration::from_secs(86400);
        let time_2day = *cmdrec_1count.last_exec_time() + Duration::from_secs(86400 * 2);

        let mut cmdrec_1000count = CmdRecord::new("".into());
        cmdrec_1000count.cmd_data.count = 1000;
        cmdrec_1000count.cmd_data.last_exec_time = *cmdrec_1count.last_exec_time();
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
