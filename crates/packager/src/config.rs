use std::path::{Path, PathBuf};

pub use cargo_packager_config::*;

#[derive(Debug, Clone)]
pub(crate) struct Resource {
    pub src: PathBuf,
    pub target: PathBuf,
}

pub trait ConfigExt {
    /// Returns the windows specific configuration
    fn windows(&self) -> Option<&WindowsConfig>;
    /// Returns the nsis specific configuration
    fn nsis(&self) -> Option<&NsisConfig>;
    /// Returns the wix specific configuration
    fn wix(&self) -> Option<&WixConfig>;
    /// Returns the debian specific configuration
    fn deb(&self) -> Option<&DebianConfig>;
    /// Returns the architecture for the binary being packaged (e.g. "arm", "x86" or "x86_64").
    fn target_arch(&self) -> crate::Result<&str>;
    /// Returns the path to the specified binary.
    fn binary_path(&self, binary: &Binary) -> PathBuf;
    /// Returns the package identifier
    fn identifier(&self) -> &str;
    /// Returns the package publisher
    fn publisher(&self) -> String;
}

impl ConfigExt for Config {
    fn windows(&self) -> Option<&WindowsConfig> {
        self.windows.as_ref()
    }

    fn nsis(&self) -> Option<&NsisConfig> {
        self.nsis.as_ref()
    }

    fn wix(&self) -> Option<&WixConfig> {
        self.wix.as_ref()
    }

    fn deb(&self) -> Option<&DebianConfig> {
        self.deb.as_ref()
    }

    fn target_arch(&self) -> crate::Result<&str> {
        Ok(if self.target_triple.starts_with("x86_64") {
            "x86_64"
        } else if self.target_triple.starts_with('i') {
            "x86"
        } else if self.target_triple.starts_with("arm") {
            "arm"
        } else if self.target_triple.starts_with("aarch64") {
            "aarch64"
        } else if self.target_triple.starts_with("universal") {
            "universal"
        } else {
            return Err(crate::Error::UnexpectedTargetTriple(
                self.target_triple.clone(),
            ));
        })
    }

    fn binary_path(&self, binary: &Binary) -> PathBuf {
        self.out_dir.join(&binary.name)
    }

    fn identifier(&self) -> &str {
        self.identifier.as_deref().unwrap_or("")
    }

    fn publisher(&self) -> String {
        let identifier = self.identifier();
        self.publisher
            .clone()
            .unwrap_or_else(|| identifier.split('.').nth(1).unwrap_or(identifier).into())
    }
}

pub(crate) trait ConfigExtInternal {
    fn main_binary(&self) -> crate::Result<&Binary>;
    fn main_binary_name(&self) -> crate::Result<&String>;
    fn resources(&self) -> Option<Vec<Resource>>;
    fn find_ico(&self) -> Option<PathBuf>;
    fn copy_resources(&self, path: &Path) -> crate::Result<()>;
    fn copy_binaries(&self, path: &Path) -> crate::Result<()>;
}

impl ConfigExtInternal for Config {
    fn main_binary(&self) -> crate::Result<&Binary> {
        self.binaries
            .iter()
            .find(|bin| bin.main)
            .ok_or_else(|| crate::Error::MainBinaryNotFound)
    }

    fn main_binary_name(&self) -> crate::Result<&String> {
        self.binaries
            .iter()
            .find(|bin| bin.main)
            .map(|b| &b.name)
            .ok_or_else(|| crate::Error::MainBinaryNotFound)
    }

    fn resources(&self) -> Option<Vec<Resource>> {
        self.resources.as_ref().map(|resources| {
            let mut out = Vec::new();
            let cwd = std::env::current_dir().expect("failed to get current directory");
            match resources {
                Resources::List(l) => {
                    for resource in l {
                        out.extend(glob::glob(resource).unwrap().filter_map(|src| {
                            src.ok().and_then(|src| {
                                use relative_path::PathExt;
                                let src =
                                    dunce::canonicalize(src).expect("failed to canonicalize path");
                                let target = src.relative_to(&cwd);
                                target.ok().map(|target| Resource {
                                    src,
                                    target: target.to_path(""),
                                })
                            })
                        }));
                    }
                }
                Resources::Map(m) => {
                    for (src, target) in m.iter() {
                        out.extend(glob::glob(src).unwrap().filter_map(|src| {
                            src.ok().map(|src| {
                                let src =
                                    dunce::canonicalize(src).expect("failed to canonicalize path");
                                let target = PathBuf::from(target).join(
                                    src.file_name()
                                        .expect("Failed to get filename of a resource file"),
                                );
                                Resource { src, target }
                            })
                        }))
                    }
                }
            }
            out
        })
    }

    fn find_ico(&self) -> Option<PathBuf> {
        self.icons
            .as_ref()
            .and_then(|icons| {
                icons
                    .iter()
                    .find(|i| PathBuf::from(i).extension().and_then(|s| s.to_str()) == Some("ico"))
                    .or_else(|| {
                        icons.iter().find(|i| {
                            PathBuf::from(i).extension().and_then(|s| s.to_str()) == Some("png")
                        })
                    })
            })
            .map(PathBuf::from)
    }

    fn copy_resources(&self, path: &Path) -> crate::Result<()> {
        if let Some(resources) = self.resources() {
            for resource in resources {
                let dest = path.join(resource.target);
                std::fs::create_dir_all(dest.parent().ok_or(crate::Error::ParentDirNotFound)?)?;
                std::fs::copy(resource.src, dest)?;
            }
        }
        Ok(())
    }

    fn copy_binaries(&self, path: &Path) -> crate::Result<()> {
        if let Some(external_binaries) = &self.external_binaries {
            for src in external_binaries {
                let src = PathBuf::from(src);
                let dest = path.join(
                    src.file_name()
                        .expect("failed to extract external binary filename")
                        .to_string_lossy()
                        .replace(&format!("-{}", self.target_triple), ""),
                );
                std::fs::create_dir_all(dest.parent().ok_or(crate::Error::ParentDirNotFound)?)?;
                std::fs::copy(src, dest)?;
            }
        }

        Ok(())
    }
}
