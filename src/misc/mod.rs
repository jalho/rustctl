//! Dumpster for miscellaneous stuff yet to be better categorized.

fn make_logger_config() -> log4rs::Config {
    let stdout: log4rs::append::console::ConsoleAppender =
        log4rs::append::console::ConsoleAppender::builder()
            .encoder(Box::new(log4rs::encode::pattern::PatternEncoder::new(
                "[{d(%Y-%m-%dT%H:%M:%S)}] {h([{l}])} [{t}] - {m}{n}",
            )))
            .build();

    let logger_config: log4rs::Config = match log4rs::Config::builder()
        .appender(log4rs::config::Appender::builder().build("stdout", Box::new(stdout)))
        .build(
            log4rs::config::Root::builder()
                .appender("stdout")
                .build(log::LevelFilter::Trace),
        ) {
        Ok(n) => n,
        Err(_) => {
            unreachable!("logger configuration does not depend on any input so it should be either always valid or never valid");
        }
    };

    return logger_config;
}

/// Initialize a global logging utility.
pub fn init_logger() -> log4rs::Handle {
    let config: log4rs::Config = make_logger_config();
    let logger: log4rs::Handle = match log4rs::init_config(config) {
        Ok(n) => n,
        Err(_) => {
            unreachable!("logger initialization should always succeed because we only do it once");
        }
    };
    return logger;
}

pub fn is_dir_rwx(path: &std::path::Path) -> bool {
    if !path.is_dir() {
        return false;
    }
    let metadata: std::fs::Metadata = match std::fs::metadata(&path) {
        Ok(n) => n,
        Err(_) => return false,
    };

    let owner_uid: u32 = std::os::unix::fs::MetadataExt::uid(&metadata);
    let current_uid: u32 = unsafe { libc::getuid() };
    let is_owned: bool = owner_uid == current_uid;

    let file_gid: u32 = std::os::unix::fs::MetadataExt::gid(&metadata);
    let mut groups: Vec<u32> = Vec::new();
    unsafe {
        let group_count: i32 = libc::getgroups(0, std::ptr::null_mut());
        if group_count > 0 {
            let mut group_ids: Vec<u32> = vec![0; group_count as usize];
            libc::getgroups(group_count, group_ids.as_mut_ptr());
            groups = group_ids;
        }
    }
    let is_belong_group: bool = groups.contains(&file_gid);

    let permissions: u32 = std::os::unix::fs::PermissionsExt::mode(&metadata.permissions());

    let has_owner_rwx: bool = permissions & 0o700 == 0o700;
    let has_group_rwx: bool = permissions & 0o070 == 0o070;
    let has_other_rwx: bool = permissions & 0o007 == 0o007;

    if is_owned && has_owner_rwx {
        return true;
    }

    if is_belong_group && has_group_rwx {
        return true;
    }

    return has_other_rwx;
}
