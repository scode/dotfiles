mod payload_symlink;
mod raw_symlink;

pub use payload_symlink::PayloadSymlink;
pub use raw_symlink::RawSymlink;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum FeatureResult {
    Changed,
    NoOp,
}

pub trait Feature: std::fmt::Debug + std::fmt::Display {
    fn install(&self) -> anyhow::Result<FeatureResult>;
    fn uninstall(&self) -> anyhow::Result<FeatureResult>;
}
