//! Core functionality of the program.

pub trait JoinWith {
    fn join_with(&self, joiner: &str) -> String;
}

impl JoinWith for Vec<std::path::PathBuf> {
    fn join_with(&self, delim: &str) -> String {
        self.iter()
            .map(|n| n.to_string_lossy().into_owned())
            .collect::<Vec<String>>()
            .join(delim)
    }
}

#[derive(Debug)]
pub enum SteamCMDArgv {
    InstallGame(Vec<String>),
    FetchGameInfo(Vec<String>),
}

impl<'arg> IntoIterator for &'arg SteamCMDArgv {
    type Item = &'arg str;

    type IntoIter = std::vec::IntoIter<Self::Item>;

    fn into_iter(self) -> Self::IntoIter {
        match self {
            SteamCMDArgv::InstallGame(vec) => {
                let iter: std::slice::Iter<'_, String> = vec.iter();
                let iter_map = iter.map(std::string::String::as_str);
                let vec_slices: Vec<&str> = iter_map.collect::<Vec<&str>>();
                let iter_slices: std::vec::IntoIter<&str> = vec_slices.into_iter();
                iter_slices
            }
            SteamCMDArgv::FetchGameInfo(vec) => {
                let iter: std::slice::Iter<'_, String> = vec.iter();
                let iter_map = iter.map(std::string::String::as_str);
                let vec_slices: Vec<&str> = iter_map.collect::<Vec<&str>>();
                let iter_slices: std::vec::IntoIter<&str> = vec_slices.into_iter();
                iter_slices
            }
        }
    }
}

impl SteamCMDArgv {
    pub fn join(&self, joiner: &'static str) -> String {
        match self {
            SteamCMDArgv::InstallGame(argv) => argv.join(joiner),
            SteamCMDArgv::FetchGameInfo(argv) => argv.join(joiner),
        }
    }
}
