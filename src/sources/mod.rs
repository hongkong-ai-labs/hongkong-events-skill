//! Compile-time selectable event sources.
//!
//! Every provider lives in its own module behind a `source-*` Cargo feature.
//! The default build enables all of them; a slim build can select only the
//! feeds it needs, for example:
//!
//! ```text
//! cargo build --no-default-features --features source-lcsd
//! ```

#[cfg(feature = "source-lcsd")]
pub mod lcsd;
#[cfg(feature = "source-popticket")]
pub mod popticket;
#[cfg(feature = "source-venues")]
pub mod venues;

/// A source compiled into this binary.
#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize)]
pub struct SourceInfo {
    pub name: &'static str,
    pub kind: &'static str,
}

pub fn available() -> Vec<SourceInfo> {
    vec![
        #[cfg(feature = "source-lcsd")]
        SourceInfo {
            name: "lcsd",
            kind: "official",
        },
        #[cfg(feature = "source-popticket")]
        SourceInfo {
            name: "popticket",
            kind: "ticketing",
        },
        #[cfg(feature = "source-taikwun")]
        SourceInfo {
            name: "taikwun",
            kind: "venue",
        },
        #[cfg(feature = "source-awe")]
        SourceInfo {
            name: "awe",
            kind: "venue",
        },
        #[cfg(feature = "source-mplus")]
        SourceInfo {
            name: "mplus",
            kind: "venue",
        },
        #[cfg(feature = "source-livenation")]
        SourceInfo {
            name: "livenation",
            kind: "promoter",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reports_compiled_sources() {
        assert!(!available().is_empty());
    }
}
