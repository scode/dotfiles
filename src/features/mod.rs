mod file_symlink;

pub use file_symlink::FileSymlink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureResult {
    Changed,
    NoOp,
}

pub trait Feature: std::fmt::Debug + std::fmt::Display {
    fn install(&self) -> anyhow::Result<FeatureResult>;
    fn uninstall(&self) -> anyhow::Result<FeatureResult>;
}
