pub mod steam {
    //! Utilities for handling _Steam_ stuff, such as parsing output of the
    //! _SteamCMD_ tool which comes in a curious non-standard format.

    /// From a given _SteamCMD response_ buffer, get a _buildid_ (of the _public
    /// branch_).
    ///
    /// ## Example: _app info query_
    ///
    /// The input is a snippet from output of command:
    /// `steamcmd +login anonymous +app_info_print 258550 +quit`
    ///
    /// ```rust
    /// assert_eq!(rustctl::steam::parse_buildid_from_buffer(r#"
    ///     }
    /// }
    /// "branches"
    /// {
    ///     "public"
    ///     {
    ///         "buildid"          "17264843"
    ///         "timeupdated"              "1738866735"
    ///     }
    ///     "last-month"
    ///     {
    ///         "buildid"          "17118157"
    ///         "description"              "last-month"
    ///         "timeupdated"              "1738866558"
    ///     }
    ///     "release"
    ///     {
    ///         "buildid"          "17272528"
    /// "#), Some(17264843), "buildid of public branch is retrieved");
    /// ```
    ///
    /// ## Example: _app manifest file_
    ///
    /// The input is a snippet from the content of the Steam app manifest file:
    /// `./steamapps/appmanifest_258550.acf`
    /// (location relative to the game server executable)
    ///
    /// ```rust
    /// assert_eq!(rustctl::steam::parse_buildid_from_buffer(r#"
    /// "AppState"
    /// {
    /// 	"appid"		"258550"
    /// 	"Universe"		"1"
    /// 	"name"		"Rust Dedicated Server"
    /// 	"StateFlags"		"4"
    /// 	"installdir"		"rust_dedicated"
    /// 	"LastUpdated"		"1740164675"
    /// 	"LastPlayed"		"0"
    /// 	"SizeOnDisk"		"8887284810"
    /// 	"StagingSize"		"0"
    /// 	"buildid"		"17422839"
    /// "#), Some(17422839), "buildid is retrieved");
    /// ```
    pub fn parse_buildid_from_buffer(buffer: &str) -> Option<u32> {
        let line: &str = buildid_line_trimmed(buffer);
        let buildid: Option<u32> = buildid(line);
        return buildid;
    }

    fn buildid(line: &str) -> Option<u32> {
        let regex = regex::Regex::new("\\d+").expect("regex should be valid");
        let first_match: regex::Match = regex.find(line)?;
        let first_match: &str = first_match.into();
        let buildid: u32 = first_match
            .parse::<u32>()
            .expect("result matching the regex should be parseable as u32");
        return Some(buildid);
    }

    /// From a given _SteamCMD response_ buffer, get a trimmed _buildid_ line
    /// (of the _public branch_).
    ///
    /// In case no seeked line is found, an empty slice is returned.
    ///
    /// In case no branches are defined, any first _buildid_ line is returned.
    fn buildid_line_trimmed(buffer: &str) -> &str {
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
}
