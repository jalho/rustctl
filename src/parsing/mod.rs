pub fn parse_buildid_from_manifest(manifest_path: &std::path::Path) -> Option<u32> {
    if let Ok(content) = std::fs::read_to_string(manifest_path) {
        return parse_buildid_from_buffer(&content);
    }
    return None;
}

/// Parse a Steam app's build ID from a curious format found in various Steam
/// contexts such as app manifest file or SteamCMD executable's STDOUT for some
/// commands.
///
/// The format resembles JSON but is not quite it. Below is a sample:
/// ```
/// AppID : 258550, change number : 27353348/0, last change : Thu Feb  6 23:43:53 2025
/// "258550"
/// {
///     "common"
///     {
///         "name"		"Rust Dedicated Server"
///         "type"		"Game"
///         "ReleaseState"		"released"
///         "oslist"		"windows,linux"
///         "osarch"		""
///         "osextended"		""
///         "associations"
///         {
///         }
///         "gameid"		"258550"
///         "store_tags"
///         {
///         }
///     }
///     "extended"
///     {
///         "gamedir"		""
///     }
///     "config"
///     {
///         "contenttype"		"3"
///         "installdir"		"rust_dedicated"
///     }
///     "depots"
///     {
///         "overridescddb"		"1"
///         "markdlcdepots"		"1"
///         "258551"
///         {
///             "systemdefined"		"1"
///             "config"
///             {
///                 "oslist"		"windows"
///             }
///             "manifests"
///             {
///                 "public"
///                 {
///                     "gid"		"3887947441418003849"
///                     "size"		"629457924"
///                     "download"		"408963248"
///                 }
///                 "aux01"
///                 {
///                     "gid"		"54235556778710650"
///                     "size"		"620673941"
///                     "download"		"406281904"
///                 }
///                 "aux02"
///                 {
///                     "gid"		"8378924999468492889"
///                     "size"		"629581615"
///                     "download"		"409009840"
///                 }
///                 "debug"
///                 {
///                     "gid"		"2591346971987511447"
///                     "size"		"631208934"
///                     "download"		"393851888"
///                 }
///                 "last-month"
///                 {
///                     "gid"		"5434512232985231584"
///                     "size"		"594678309"
///                     "download"		"384215456"
///                 }
///                 "release"
///                 {
///                     "gid"		"5588691717074837775"
///                     "size"		"629459084"
///                     "download"		"408944880"
///                 }
///                 "staging"
///                 {
///                     "gid"		"4604597478204556032"
///                     "size"		"629594732"
///                     "download"		"408988832"
///                 }
///             }
///         }
///         "258552"
///         {
///             "config"
///             {
///                 "oslist"		"linux"
///             }
///             "manifests"
///             {
///                 "public"
///                 {
///                     "gid"		"8349664598014094040"
///                     "size"		"647585258"
///                     "download"		"382506992"
///                 }
///                 "aux01"
///                 {
///                     "gid"		"949630714142880006"
///                     "size"		"633254403"
///                     "download"		"379782080"
///                 }
///                 "aux02"
///                 {
///                     "gid"		"8297355357763306305"
///                     "size"		"647370869"
///                     "download"		"382371760"
///                 }
///                 "debug"
///                 {
///                     "gid"		"8305401038939912362"
///                     "size"		"139415558"
///                     "download"		"34397936"
///                 }
///                 "last-month"
///                 {
///                     "gid"		"5464837686115944412"
///                     "size"		"631381847"
///                     "download"		"376852432"
///                 }
///                 "release"
///                 {
///                     "gid"		"4255743540337384038"
///                     "size"		"647586418"
///                     "download"		"382483328"
///                 }
///                 "staging"
///                 {
///                     "gid"		"7937960338018858959"
///                     "size"		"647383482"
///                     "download"		"382348128"
///                 }
///             }
///         }
///         "258554"
///         {
///             "manifests"
///             {
///                 "public"
///                 {
///                     "gid"		"8648086317383607729"
///                     "size"		"8239237106"
///                     "download"		"2017291472"
///                 }
///                 "aux01"
///                 {
///                     "gid"		"8920793311077920065"
///                     "size"		"8553881438"
///                     "download"		"2212480736"
///                 }
///                 "aux02"
///                 {
///                     "gid"		"5857600455914731676"
///                     "size"		"8239701885"
///                     "download"		"2017520016"
///                 }
///                 "debug"
///                 {
///                     "gid"		"9050441631359858310"
///                     "size"		"8000428058"
///                     "download"		"1942290304"
///                 }
///                 "last-month"
///                 {
///                     "gid"		"1453623735786069882"
///                     "size"		"8004379972"
///                     "download"		"1943609440"
///                 }
///                 "release"
///                 {
///                     "gid"		"4937557610765474446"
///                     "size"		"8239252034"
///                     "download"		"2017305072"
///                 }
///                 "staging"
///                 {
///                     "gid"		"2097931034996216866"
///                     "size"		"8239252018"
///                     "download"		"2017303760"
///                 }
///             }
///         }
///         "branches"
///         {
///             "public"
///             {
///                 "buildid"		"17264843"
///                 "timeupdated"		"1738866735"
///             }
///             "aux01"
///             {
///                 "buildid"		"16999813"
///                 "description"		"Pre-Staging"
///                 "timeupdated"		"1736859300"
///             }
///             "aux02"
///             {
///                 "buildid"		"17265557"
///                 "description"		"Up and coming"
///                 "timeupdated"		"1738841565"
///             }
///             "debug"
///             {
///                 "buildid"		"17041002"
///                 "description"		"2021 testing"
///                 "timeupdated"		"1737126347"
///             }
///             "last-month"
///             {
///                 "buildid"		"17118157"
///                 "description"		"last-month"
///                 "timeupdated"		"1738866558"
///             }
///             "release"
///             {
///                 "buildid"		"17272528"
///                 "timeupdated"		"1738877764"
///             }
///             "staging"
///             {
///                 "buildid"		"17271927"
///                 "timeupdated"		"1738874343"
///             }
///         }
///         "privatebranches"		"1"
///     }
/// }
/// ```
pub fn parse_buildid_from_buffer(buffer: &str) -> Option<u32> {
    for line in buffer.lines() {
        let trimmed: &str = line.trim();
        if trimmed.starts_with("\"buildid\"") {
            if let Some(_) = trimmed.find('\"') {
                let parts: Vec<&str> = trimmed.split_whitespace().collect();
                if parts.len() >= 2 {
                    if let Ok(buildid) = parts[1].trim_matches('"').parse::<u32>() {
                        return Some(buildid);
                    }
                }
            }
        }
    }
    return None;
}

#[derive(clap::Parser)]
pub struct Cli {
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

#[derive(clap::Subcommand)]
pub enum Subcommand {
    GameStart {
        #[arg(
            long,
            help = "Exclude a directory from the game start process's search for the game executable.",
            long_help = r#"Exclude a directory from the game start process's search for the game
executable. This is useful, for example, when developing on WSL (Windows
Subsystem for Linux), where performing a whole system wide search tends to be
particularly slow. In such cases, you may want to exclude `/mnt/c/`"#,
            value_name = "DIRECTORY",
            default_value = None
        )]
        exclude: Option<std::path::PathBuf>,
    },
}
