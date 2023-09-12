use std::path::{Path, PathBuf};

pub use cargo_packager_config::*;
use relative_path::PathExt;

use crate::util;

#[derive(Debug, Clone)]
pub(crate) struct IResource {
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
    /// Returns the target triple for the package to be built (e.g. "aarch64-unknown-linux-gnu").
    fn target_triple(&self) -> String;
    /// Returns the architecture for the package to be built (e.g. "arm", "x86" or "x86_64").
    fn target_arch(&self) -> crate::Result<&str>;
    /// Returns the path to the specified binary.
    fn binary_path(&self, binary: &Binary) -> PathBuf;
    /// Returns the package identifier
    fn identifier(&self) -> &str;
    /// Returns the package publisher
    fn publisher(&self) -> String;
    /// Returns the out dir
    fn out_dir(&self) -> PathBuf;
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

    fn target_triple(&self) -> String {
        self.target_triple
            .clone()
            .unwrap_or_else(|| util::target_triple().unwrap())
    }

    fn target_arch(&self) -> crate::Result<&str> {
        let target = self.target_triple();
        Ok(if target.starts_with("x86_64") {
            "x86_64"
        } else if target.starts_with('i') {
            "x86"
        } else if target.starts_with("arm") {
            "arm"
        } else if target.starts_with("aarch64") {
            "aarch64"
        } else if target.starts_with("universal") {
            "universal"
        } else {
            return Err(crate::Error::UnexpectedTargetTriple(target.clone()));
        })
    }

    fn binary_path(&self, binary: &Binary) -> PathBuf {
        self.out_dir().join(&binary.filename)
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

    fn out_dir(&self) -> PathBuf {
        dunce::canonicalize(&self.out_dir).unwrap_or_else(|_| self.out_dir.clone())
    }
}

pub(crate) trait ConfigExtInternal {
    fn main_binary(&self) -> crate::Result<&Binary>;
    fn main_binary_name(&self) -> crate::Result<&String>;
    fn resources(&self) -> crate::Result<Vec<IResource>>;
    fn find_ico(&self) -> Option<PathBuf>;
    fn copy_resources(&self, path: &Path) -> crate::Result<()>;
    fn copy_external_binaries(&self, path: &Path) -> crate::Result<()>;
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
            .map(|b| &b.filename)
            .ok_or_else(|| crate::Error::MainBinaryNotFound)
    }

    fn resources(&self) -> crate::Result<Vec<IResource>> {
        if let Some(resources) = &self.resources {
            let cwd = std::env::current_dir()?;
            let cwd = dunce::simplified(&cwd);

            #[inline]
            fn create_resources_from_dir(
                src: &PathBuf,
                target: &PathBuf,
            ) -> crate::Result<Vec<IResource>> {
                let mut out = Vec::new();
                for entry in walkdir::WalkDir::new(&src) {
                    let entry = entry?;
                    let path = entry.path();
                    if path.is_file() {
                        let relative = path.relative_to(&src)?.to_path("");
                        let resource = IResource {
                            src: dunce::canonicalize(path.to_path_buf())?,
                            target: target.join(relative),
                        };
                        out.push(resource);
                    }
                }
                Ok(out)
            }

            let mut out = Vec::new();
            for r in resources {
                match r {
                    Resource::Single(src) => {
                        let src_p = PathBuf::from(src);
                        if src_p.is_dir() {
                            out.extend(create_resources_from_dir(&src_p, &src_p)?);
                        } else {
                            for src in glob::glob(src).unwrap() {
                                let src = src?;
                                let src = dunce::canonicalize(src)?;
                                let target = src.relative_to(&cwd)?;
                                out.push(IResource {
                                    src,
                                    target: target.to_path(""),
                                })
                            }
                        }
                    }
                    Resource::Mapped { src, target } => {
                        let src = dunce::canonicalize(src)?;
                        let target = PathBuf::from(target);
                        if src.is_file() {
                            out.push(IResource { src, target })
                        } else if src.is_dir() {
                            out.extend(create_resources_from_dir(&src, &target)?);
                        }
                    }
                }
            }

            Ok(out)
        } else {
            Ok(vec![])
        }
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
        for resource in self.resources()? {
            let dest = path.join(resource.target);
            std::fs::create_dir_all(dest.parent().ok_or(crate::Error::ParentDirNotFound)?)?;
            std::fs::copy(resource.src, dest)?;
        }
        Ok(())
    }

    fn copy_external_binaries(&self, path: &Path) -> crate::Result<()> {
        if let Some(external_binaries) = &self.external_binaries {
            for src in external_binaries {
                let src = dunce::canonicalize(PathBuf::from(src))?;
                let dest = path.join(
                    src.file_name()
                        .expect("failed to extract external binary filename")
                        .to_string_lossy()
                        .replace(&format!("-{}", self.target_triple()), ""),
                );
                std::fs::copy(src, dest)?;
            }
        }

        Ok(())
    }
}
