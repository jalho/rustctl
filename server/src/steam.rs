//! Utilities for working with Steam or apps distributed via Steam.

pub struct RustDedicated;

impl RustDedicated {
    const APP_ID: AppID = AppID::new(258550);

    pub fn app_id() -> AppID {
        Self::APP_ID
    }

    /// Use SteamCMD to query the latest available build ID from some online
    /// source of truth, whatever SteamCMD uses.
    ///
    /// RANT: It would be nice if a regular, professionally maintained and
    /// documented HTTP API existed instead, but unfortunately it doesn't and so
    /// we're forced to use SteamCMD!
    pub async fn query_latest_available_build_id(
        ctoken: tokio_util::sync::CancellationToken,
    ) -> Result<BuildID, String> {
        let local_app_info_cache: Option<String> = match Self::find_steam_app_info_local_cache(ctoken).await {
            Ok(n) => n,
            Err(_err) => return Err("failed to find local Steam app info cache to be pruned".to_string()),
        };

        if let Some(local_app_info_cache) = local_app_info_cache
            && let Err(err) = tokio::fs::remove_file(&local_app_info_cache).await
        {
            return Err(format!(
                "failed to remove local Steam app info cache: {local_app_info_cache}: {err}"
            ));
        }

        let mut cmd = tokio::process::Command::new(rustctl_backend::constants::paths::INSTALLER);
        cmd.args([
            "+login",
            "anonymous",
            "+app_info_print",
            &Self::APP_ID.to_string(),
            "+quit",
        ]);
        let output: String = match cmd.output().await {
            Ok(n) => match n.status.success() {
                true => match String::from_utf8(n.stdout) {
                    Ok(n) => n,
                    Err(_) => todo!(),
                },
                false => todo!(),
            },
            Err(_) => todo!(),
        };
        let build_id: BuildID = BuildID::from_vdf_steamcmd_contaminated(&output).unwrap();
        Ok(build_id)
    }

    /// Find locally cached Steam app info (i.e. a file named `appinfo.vdf`).
    ///
    /// One place it's been seen at as of 2025-10-18 is:
    /// ```
    /// ~/.local/share/Steam/appcache/appinfo.vdf
    /// ```
    async fn find_steam_app_info_local_cache(
        ctoken: tokio_util::sync::CancellationToken,
    ) -> Result<Option<String>, ()> {
        let results: Vec<std::path::PathBuf> = match tokio::task::spawn_blocking(move || {
            let mut results: Vec<std::path::PathBuf> = Vec::new();

            let walker: walkdir::IntoIter = walkdir::WalkDir::new("/").follow_links(false).into_iter();

            for n in walker {
                if ctoken.is_cancelled() {
                    return Vec::new();
                }

                let n: walkdir::DirEntry = match n {
                    Ok(n) => n,
                    Err(_) => continue,
                };

                let path: &std::path::Path = n.path();

                /*
                 * Skip Windows stuff (in case running in WSL). Checking the Windows
                 * filesystem from within WSL is very slow and also not necessary in
                 * our case!
                 */
                if path.starts_with("/mnt") {
                    continue;
                }

                let seems_cache: bool = match path.to_str() {
                    Some(n) => n.contains("cache"),
                    None => false,
                };

                if let Some(name) = path.file_name()
                    && name == "appinfo.vdf"
                    && seems_cache
                {
                    results.push(path.to_path_buf());
                }
            }

            results
        })
        .await
        {
            Ok(n) => n,
            Err(_) => return Err(()),
        };

        let target: &std::path::PathBuf = match results.len() {
            0 => return Ok(None),

            1 => &results[0],

            /*
             * Not supporting multiple Steam installations for now!
             */
            2.. => return Err(()),
        };

        let meta: std::fs::Metadata = match tokio::fs::metadata(&target).await {
            Ok(n) => n,
            Err(_) => return Err(()),
        };

        /*
         * Only a cache owned by the current user should be relevant.
         */
        let current_uid: u32 = unsafe { libc::geteuid() };
        let cache_uid: u32;
        {
            use std::os::unix::fs::MetadataExt;
            cache_uid = meta.uid();
        }
        if cache_uid != current_uid {
            return Err(());
        }

        match target.canonicalize() {
            Ok(n) => Ok(Some(n.to_string_lossy().to_string())),
            Err(_) => Err(()),
        }
    }

    pub async fn install() -> Result<BuildID, String> {
        let mut command = tokio::process::Command::new(rustctl_backend::constants::paths::INSTALLER);
        command.current_dir(rustctl_backend::constants::paths::ROOT_DIR);
        command.args(vec![
            "+login",
            "anonymous",
            "+force_install_dir",
            rustctl_backend::constants::paths::ROOT_DIR,
            "+app_update",
            "258550",
            "validate",
            "+quit",
        ]);
        command.stdout(std::process::Stdio::null());
        command.stderr(std::process::Stdio::null());

        let process: tokio::process::Child = match command.spawn() {
            Ok(n) => n,
            Err(err) => {
                return Err(format!("failed to spawn game server installer: {command:?}: {err}"));
            }
        };

        let _output: std::process::Output = match process.wait_with_output().await {
            Ok(n) => n,
            Err(err) => {
                return Err(format!("failed to run game server installer to termination: {err}"));
            }
        };

        let buildid: BuildID =
            match BuildID::from_existing_installation_manifest(rustctl_backend::constants::paths::MANIFEST).await {
                Some(n) => n,
                None => {
                    return Err(format!(
                        r#"failed to extract buildid from manifest "{manifest_path}""#,
                        manifest_path = rustctl_backend::constants::paths::MANIFEST,
                    ));
                }
            };

        Ok(buildid)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct BuildID(u32);

impl From<BuildID> for u32 {
    fn from(value: BuildID) -> Self {
        value.0
    }
}

impl From<&BuildID> for u32 {
    fn from(value: &BuildID) -> Self {
        value.0
    }
}

impl BuildID {
    pub fn new(value: u32) -> Self {
        Self(value)
    }

    pub async fn from_existing_installation_manifest<P>(manifest: P) -> Option<Self>
    where
        P: AsRef<std::path::Path>,
    {
        let content: String = match tokio::fs::read_to_string(manifest).await {
            Ok(n) => n,
            Err(_) => return None,
        };
        let build_id = match Self::from_vdf_appmanifest(&content) {
            Ok(n) => n,
            Err(_) => return None,
        };
        Some(build_id)
    }

    /// By "contaminated VDF" we mean the esoteric output format of the following command:
    ///
    /// ```
    /// $ /usr/bin/steamcmd +login anonymous +app_info_print 258550 +quit
    /// ```
    pub fn from_vdf_steamcmd_contaminated(contaminated_vdf: &str) -> Result<Self, String> {
        /*
         * Strip the "contamination" i.e. whatever precedes the actual "VDF"
         * data.
         */
        let vdf_start_incl: usize = contaminated_vdf.find('"').ok_or("could not find start of VDF")?;
        let vdf_end_incl = contaminated_vdf.rfind('}').ok_or("could not find end of VDF")?;
        let data: &str = &contaminated_vdf[vdf_start_incl..=vdf_end_incl];

        Self::from_vdf_steamcmd(data)
    }

    fn from_vdf_steamcmd(data: &str) -> Result<Self, String> {
        let vdf: keyvalues_parser::Vdf =
            keyvalues_parser::Vdf::parse(data).map_err(|err| format!("failed to parse VDF: {err}"))?;

        let key: String = vdf.key.to_string();
        if key != RustDedicated::app_id().to_string() {
            return Err(format!("unexpected top level key in VDF: {key}"));
        }
        let value: keyvalues_parser::Value = vdf.value;

        let obj: &keyvalues_parser::Obj = match value.get_obj() {
            Some(n) => n,
            None => todo!(),
        };

        let depots: &keyvalues_parser::Obj = obj.get("depots").unwrap().first().unwrap().get_obj().unwrap();
        let branches: &keyvalues_parser::Obj = depots.get("branches").unwrap().first().unwrap().get_obj().unwrap();
        let public: &keyvalues_parser::Obj = branches.get("public").unwrap().first().unwrap().get_obj().unwrap();

        let buildid: &keyvalues_parser::Value = public.get("buildid").unwrap().first().unwrap();
        let buildid: String = buildid.to_string().trim_matches('"').to_owned();
        let buildid: u32 = buildid.parse::<u32>().unwrap();
        let buildid = Self::new(buildid);

        Ok(buildid)
    }

    fn from_vdf_appmanifest(data: &str) -> Result<Self, String> {
        let vdf: keyvalues_parser::Vdf =
            keyvalues_parser::Vdf::parse(data).map_err(|err| format!("failed to parse VDF: {err}"))?;

        let key: String = vdf.key.to_string();
        if key != "AppState" {
            return Err(format!("unexpected top level key in VDF: {key}"));
        }
        let value: keyvalues_parser::Value = vdf.value;

        let obj: &keyvalues_parser::Obj = match value.get_obj() {
            Some(n) => n,
            None => todo!(),
        };

        let buildid: &keyvalues_parser::Value = obj.get("buildid").unwrap().first().unwrap();
        let buildid: String = buildid.to_string().trim_matches('"').to_owned();
        let buildid: u32 = buildid.parse::<u32>().unwrap();
        let buildid = Self::new(buildid);

        Ok(buildid)
    }
}

#[test]
fn test_from_vdf_steamcmd_contaminated() {
    let input = r#"Redirecting stderr to '/home/test/.steam/logs/stderr.txt'
Logging directory: '/home/test/.steam/logs'
[  0%] Checking for available updates...
[----] Verifying installation...
UpdateUI: skip show logo
Steam Console Client (c) Valve Corporation - version 1751406682
-- type 'quit' to exit --
Loading Steam API...[0mOK
[0m
Connecting anonymously to Steam Public...[0mOK
[0mWaiting for client config...[0mOK
[0mWaiting for user info...[0mOK
[0mAppID : 258550, change number : 30975083/0, last change : Sat Sep  6 14:58:05 2025 
"258550"
{
	"common"
	{
		"name"		"Rust Dedicated Server"
		"type"		"Game"
		"ReleaseState"		"released"
		"oslist"		"windows,linux"
		"osarch"		""
		"osextended"		""
		"associations"
		{
		}
		"gameid"		"258550"
		"store_tags"
		{
		}
	}
	"extended"
	{
		"gamedir"		""
	}
	"config"
	{
		"contenttype"		"3"
		"installdir"		"rust_dedicated"
	}
	"depots"
	{
		"overridescddb"		"1"
		"markdlcdepots"		"1"
		"258551"
		{
			"systemdefined"		"1"
			"config"
			{
				"oslist"		"windows"
			}
			"manifests"
			{
				"public"
				{
					"gid"		"1510792641716687326"
					"size"		"785934886"
					"download"		"539828160"
				}
				"aux01"
				{
					"gid"		"7103381528489138813"
					"size"		"786188223"
					"download"		"539930688"
				}
				"aux02"
				{
					"gid"		"3068878779190991067"
					"size"		"785460378"
					"download"		"555923952"
				}
				"debug"
				{
					"gid"		"2591346971987511447"
					"size"		"631208934"
					"download"		"393851888"
				}
				"last-month"
				{
					"gid"		"3879466527584919659"
					"size"		"764942926"
					"download"		"535533664"
				}
				"release"
				{
					"gid"		"1510792641716687326"
					"size"		"785934886"
					"download"		"539828160"
				}
				"staging"
				{
					"gid"		"5345303349990664957"
					"size"		"785935378"
					"download"		"539828752"
				}
			}
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
					"gid"		"7826341510309710521"
					"size"		"862726932"
					"download"		"549710736"
				}
				"aux01"
				{
					"gid"		"7207883656471091371"
					"size"		"862436261"
					"download"		"549340832"
				}
				"aux02"
				{
					"gid"		"306600128080277368"
					"size"		"843401360"
					"download"		"553736544"
				}
				"debug"
				{
					"gid"		"8305401038939912362"
					"size"		"139415558"
					"download"		"34397936"
				}
				"last-month"
				{
					"gid"		"5902811470394100093"
					"size"		"823227576"
					"download"		"533611312"
				}
				"release"
				{
					"gid"		"7826341510309710521"
					"size"		"862726932"
					"download"		"549710736"
				}
				"staging"
				{
					"gid"		"7316748249696406933"
					"size"		"862727432"
					"download"		"549710992"
				}
			}
		}
		"258554"
		{
			"manifests"
			{
				"public"
				{
					"gid"		"4429398150162575463"
					"size"		"7354976769"
					"download"		"2128298960"
				}
				"aux01"
				{
					"gid"		"5152812939039282913"
					"size"		"7139403790"
					"download"		"2137060224"
				}
				"aux02"
				{
					"gid"		"5564981441159959955"
					"size"		"7052123996"
					"download"		"2077819536"
				}
				"debug"
				{
					"gid"		"9050441631359858310"
					"size"		"8000428058"
					"download"		"1942290304"
				}
				"last-month"
				{
					"gid"		"1093051191186529217"
					"size"		"7323389914"
					"download"		"2113526640"
				}
				"release"
				{
					"gid"		"4429398150162575463"
					"size"		"7354976769"
					"download"		"2128298960"
				}
				"staging"
				{
					"gid"		"4552523939505084007"
					"size"		"7090152321"
					"download"		"2101077936"
				}
			}
		}
		"branches"
		{
			"public"
			{
				"buildid"		"19874893"
				"timeupdated"		"1757147586"
			}
			"aux01"
			{
				"buildid"		"19873442"
				"description"		"Pre-Staging"
				"timeupdated"		"1757086531"
			}
			"aux02"
			{
				"buildid"		"19439083"
				"description"		"Up and coming"
				"timeupdated"		"1754023185"
			}
			"debug"
			{
				"buildid"		"17041002"
				"description"		"2021 testing"
				"timeupdated"		"1737126347"
			}
			"last-month"
			{
				"buildid"		"19776612"
				"description"		"last-month"
				"timeupdated"		"1757005497"
			}
			"release"
			{
				"buildid"		"19874893"
				"description"		"Release, before public"
				"timeupdated"		"1757094564"
			}
			"staging"
			{
				"buildid"		"19881072"
				"timeupdated"		"1757151030"
			}
		}
		"privatebranches"		"1"
	}
}
Unloading Steam API..."#;
    let build_id = BuildID::from_vdf_steamcmd_contaminated(input).unwrap();
    assert_eq!(build_id, BuildID::new(19874893));
}

impl std::fmt::Display for BuildID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

pub struct AppID(u32);

impl AppID {
    const fn new(value: u32) -> Self {
        Self(value)
    }
}

impl std::fmt::Display for AppID {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.0)
    }
}

#[test]
fn test_from_vdf_appmanifest() {
    let input = r#""AppState"
{
        "appid"         "258550"
        "Universe"              "1"
        "name"          "Rust Dedicated Server"
        "StateFlags"            "4"
        "installdir"            "rust_dedicated"
        "LastUpdated"           "1757159921"
        "LastPlayed"            "0"
        "SizeOnDisk"            "8217703701"
        "StagingSize"           "0"
        "buildid"               "19874893"
        "LastOwner"             "76561200247933079"
        "DownloadType"          "1"
        "UpdateResult"          "0"
        "BytesToDownload"               "4288512"
        "BytesDownloaded"               "4288512"
        "BytesToStage"          "4187256364"
        "BytesStaged"           "4187256364"
        "TargetBuildID"         "19874893"
        "AutoUpdateBehavior"            "0"
        "AllowOtherDownloadsWhileRunning"               "0"
        "ScheduledAutoUpdate"           "0"
        "InstalledDepots"
        {
                "258552"
                {
                        "manifest"              "7826341510309710521"
                        "size"          "862726932"
                }
                "258554"
                {
                        "manifest"              "4429398150162575463"
                        "size"          "7354976769"
                }
        }
        "UserConfig"
        {
        }
        "MountedConfig"
        {
        }
}"#;
    let _buildid: BuildID = BuildID::from_vdf_appmanifest(input).unwrap();
}
