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
    let mut seen_keyword_branches: bool = false;
    let mut seen_keyword_public: bool = false;

    let mut buildid_line: Option<&str> = None;

    // todo!("only pick the buildid that follows 'public' branch, and use the malformed steam app info error variant from crate::error::fatal for error");
    for line in buffer.lines() {
        let trimmed: &str = line.trim();

        if trimmed.contains("branches") {
            seen_keyword_branches = true;
            continue;
        } else if trimmed.contains("public") {
            seen_keyword_public = true;
            continue;
        } else if trimmed.contains("buildid") {
            if seen_keyword_branches && seen_keyword_public {
                buildid_line = Some(trimmed);
                break;
            } else if !seen_keyword_branches && !seen_keyword_public {
                buildid_line = Some(trimmed);
                break;
            }
        }
    }
    return None;
}

/// From a given trimmed line from a _SteamCMD response_ buffer, parse a numeric
/// _buildid_ value. `None` is returned in case none is found.
// fn buildid_from_trimmed_line(line: &str) -> Option<u32> {
//     let foo = line.find();
//     return None;
// }

/// From a given _SteamCMD response_ buffer, get a trimmed _buildid_ line. In
/// case no seeked line is found, an empty slice is returned.
fn pick_buildid_line_trimmed(buffer: &str) -> &str {
    let mut seen_keyword_branches: bool = false;
    let mut seen_keyword_public: bool = false;

    let mut buildid_line: Option<&str> = None;

    for line in buffer.lines() {
        let trimmed: &str = line.trim();

        if trimmed.contains("branches") {
            seen_keyword_branches = true;
            continue;
        } else if trimmed.contains("public") {
            seen_keyword_public = true;
            continue;
        } else if trimmed.contains("buildid") {
            if seen_keyword_branches && seen_keyword_public {
                buildid_line = Some(trimmed);
                break;
            } else if !seen_keyword_branches && !seen_keyword_public {
                buildid_line = Some(trimmed);
                break;
            }
        }
    }

    return match buildid_line {
        Some(n) => n,
        None => "",
    };
}

#[cfg(test)]
mod test {
    /// Unit test for parsing the response, i.e. STDOUT of command, for:
    /// ```
    /// steamcmd +login anonymous +app_info_print 258550 +quit
    /// ```
    #[test]
    fn steamcmd_app_info_print() {
        /* Not representative of an actual response, but effectively describes the
        parser logic in a generalized way. */
        assert_eq!(
            super::pick_buildid_line_trimmed(
                r#"
"branches"
"public"
"buildid" "123"
"#
            ),
            r#""buildid" "123""#,
            "generalized example"
        );

        assert_eq!(
            super::pick_buildid_line_trimmed(
                "
no matching line here
"
            ),
            "",
            "empty slice returned in case no match"
        );

        assert_eq!(
            super::pick_buildid_line_trimmed(
                "
     buildid whatever      
"
            ),
            "buildid whatever",
            "line is trimmed"
        );

        /* A more comprehensive example with content that is close to an actual
        response's content. */
        assert_eq!(
            super::pick_buildid_line_trimmed(
                r#"
AppID : 258550, change number : 27353348/0, last change : Thu Feb  6 23:43:53 2025
"258550"
{
    "common"
    {
        "name"		"Rust Dedicated Server"
        "gameid"		"258550"
    }
    "config"
    {
        "installdir"		"rust_dedicated"
    }
    "depots"
    {
        "258551"
        {
            "manifests"
            {
                "public"
                {
                    "gid"		"3887947441418003849"
                    "size"		"629457924"
                    "download"		"408963248"
                }
        "258552"
        {
            "config"
            {
                "oslist"		"linux"
            }
            "manifests"
            {
                "public"
                {
                    "gid"		"8349664598014094040"
                    "size"		"647585258"
                    "download"		"382506992"
                }
                "aux01"
                {
                    "gid"		"949630714142880006"
                    "size"		"633254403"
                    "download"		"379782080"
                }
            }
        }
        "branches"
        {
            "public"
            {
                "buildid"		"17264843"
                "timeupdated"		"1738866735"
            }
            "aux01"
            {
                "buildid"		"16999813"
                "description"		"Pre-Staging"
                "timeupdated"		"1736859300"
            }
            "release"
            {
                "buildid"		"17272528"
                "timeupdated"		"1738877764"
            }
            "staging"
            {
                "buildid"		"17271927"
                "timeupdated"		"1738874343"
            }
        }
    }
}
"#
            ),
            r#""buildid"		"17264843""#,
            "more comprehensive sample"
        );
    }
}

#[derive(clap::Parser)]
pub struct Cli {
    #[arg(short, long, default_value = "info", value_parser = parse_log_level)]
    pub log_level: log::LevelFilter,
    #[command(subcommand)]
    pub subcommand: Subcommand,
}

fn parse_log_level(input: &str) -> std::result::Result<log::LevelFilter, std::string::String> {
    const SUPPORTED_LEVELS: [(&str, log::LevelFilter); 6] = [
        ("off", log::LevelFilter::Off),
        ("error", log::LevelFilter::Error),
        ("warn", log::LevelFilter::Warn),
        ("info", log::LevelFilter::Info),
        ("debug", log::LevelFilter::Debug),
        ("trace", log::LevelFilter::Trace),
    ];

    SUPPORTED_LEVELS
        .iter()
        .find(|(name, _)| {
            let name: &str = *name;
            name == input
        })
        .map(|&(_, level)| level)
        .ok_or_else(|| {
            let supported = SUPPORTED_LEVELS
                .iter()
                .map(|(name, _)| *name)
                .collect::<std::vec::Vec<&str>>()
                .join(", ");
            format!("supported values: {supported}")
        })
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
