type Result = std::result::Result<(), cuna::error::Error>;

#[cfg(test)]
mod time {
    use super::*;
    use cuna::time::*;
    #[test]
    fn create() {
        let timestamp = TimeStamp::new(61, 29, 73);
        assert_eq!(TimeStamp::from_msf_opt(61, 29, 73), Some(timestamp.clone()));
        assert_eq!(TimeStamp::from_msf_opt(61, 29, 77), None);
        assert_eq!(TimeStamp::from_msf(61, 28, 73 + 75), timestamp);
    }
    #[test]
    fn display() {
        let timestamp = TimeStamp::new(61, 29, 73);
        assert_eq!(timestamp.to_string(), "61:29:73");
    }
    #[test]
    fn parse() -> Result {
        assert_eq!("61:29:73".parse::<TimeStamp>()?, TimeStamp::new(61, 29, 73));
        assert!("xd".parse::<TimeStamp>().is_err());
        assert!("6:772:11".parse::<TimeStamp>().is_err());
        assert!("6:72:111".parse::<TimeStamp>().is_err());
        Ok(())
    }
    #[test]
    fn modify() {
        let mut timestamp = TimeStamp::new(21, 29, 73);
        timestamp.set_frames(21);
        assert_eq!(timestamp, TimeStamp::new(21, 29, 21));
        timestamp.set_seconds(33);
        assert_eq!(timestamp, TimeStamp::new(21, 33, 21));
        timestamp.set_minutes(28);
        assert_eq!(timestamp, TimeStamp::new(28, 33, 21));
    }
    #[test]
    #[should_panic]
    fn modify_panic() {
        let mut timestamp = TimeStamp::new(61, 29, 73);
        timestamp.set_frames(88);
    }
}
#[cfg(test)]
mod command {
    use super::*;
    use cuna::parser::Command;

    #[test]
    fn new() -> Result {
        let cmd = r#"PERFORMER "Supercell""#;
        Command::new(cmd)?;
        Ok(())
    }
    #[test]
    fn display() -> Result {
        let cmds = r#"REM COMMENT ExactAudioCopy v0.99pb5
        PERFORMER "Supercell"
        TITLE "My Dearest"
        FILE "Supercell - My Dearest.flac" WAVE"#;
        for (cmd, ori) in cmds.lines().map(Command::new).zip(cmds.lines()) {
            assert_eq!(cmd?.to_string(), ori.trim().to_string())
        }
        Ok(())
    }
}
#[cfg(test)]
mod cue_sheet {
    use super::*;
    use cuna::CueSheet;

    const CUE: &str = include_str!(r"EGOIST - Departures ～あなたにおくるアイの歌～.cue");

    #[test]
    fn new() -> Result {
        let sheet = CueSheet::from_utf8_with_bom(CUE)?;
        assert_eq!(sheet.header.title, Some(vec!["Departures ～あなたにおくるアイの歌～".to_owned()]));
        assert_eq!(sheet.files.len(), 1);
        assert_eq!(&sheet.files[0].name, "EGOIST - Departures ～あなたにおくるアイの歌～.flac");
        assert_eq!(sheet.last_track().unwrap().performer(), Some(&vec!["EGOIST".to_owned()]));
        Ok(())
    }
}