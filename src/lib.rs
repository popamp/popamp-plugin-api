//! The calls between Popamp and its plug-ins, shared by the player and by
//! plug-ins written in Rust so both sides are checked by the compiler.
//!
//! A plug-in is an [Extism](https://extism.org) module. Every call passes
//! JSON: the input is the JSON of the type named at each call, and so is
//! the output. A call fails by returning an Extism error; its message is
//! shown in the title display.
//!
//! A `source` plug-in exports:
//!
//! | Export | Input | Output |
//! |---|---|---|
//! | [`calls::API_VERSION`] | none | [`Version`] |
//! | [`calls::EXPAND`] | a location (`String`) | `Option<Vec<Track>>`; `None` when it is one track |
//! | [`calls::DESCRIBE`] | a location | `Option<Tags>`; `None` when it has none to give |
//! | [`calls::OPEN`] | a location | [`Play`] |
//!
//! Which locations a source takes is written in its manifest, so the
//! player can tell without calling it.
//!
//! A `dsp` plug-in changes the decoded sound, block by block, and exports:
//!
//! | Export | Input | Output |
//! |---|---|---|
//! | [`calls::API_VERSION`] | none | [`Version`] |
//! | [`calls::RESET`] | [`Format`] | none |
//! | [`calls::PROCESS`] | a block, raw bytes | the block changed, raw bytes |
//!
//! `reset` comes before the first block and again when a track starts,
//! the position jumps or the rate changes: whatever the plug-in keeps
//! from earlier blocks (a filter's state, a delay line) starts afresh.
//! A block is interleaved stereo 32-bit float samples, little-endian, of
//! any length; the answer must be as long. Blocks are not JSON, and
//! `process` must be quick: one that takes longer than half the block's
//! playing time is played unchanged, and so is one that fails.

use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use std::fmt;
use std::str::FromStr;

/// The API version this crate describes.
pub const VERSION: Version = Version { major: 1, minor: 0 };

/// Names of the functions a plug-in exports.
pub mod calls {
    pub const API_VERSION: &str = "api_version";
    pub const EXPAND: &str = "expand";
    pub const DESCRIBE: &str = "describe";
    pub const OPEN: &str = "open";
    pub const RESET: &str = "reset";
    pub const PROCESS: &str = "process";
}

/// Names of the functions the player gives plug-ins.
pub mod host {
    /// Runs a command written in the manifest's `exec`:
    /// [`super::ExecRequest`] in, [`super::ExecOutput`] out.
    pub const EXEC: &str = "popamp_exec";
    /// The time now, as milliseconds since 1970-01-01 UTC: no input, a
    /// `u64` out. Plug-ins have no clock of their own.
    pub const NOW: &str = "popamp_now";
    /// An HTTP(S) request to an address of the manifest's `net`:
    /// [`super::HttpRequest`] in, [`super::HttpResponse`] out.
    pub const HTTP: &str = "popamp_http";
    /// What a folder holds: its path as a `String` in, [`super::Listing`]
    /// out. An empty path lists the folders the user allowed.
    pub const LIST: &str = "popamp_list";
    /// A piece of a file in those folders: [`super::ReadRequest`] in,
    /// [`super::FileData`] out.
    pub const READ: &str = "popamp_read";
}

/// Keys of the plug-in configuration the player sets, besides the
/// plug-in's own settings (keyed as in its manifest).
pub mod config {
    /// The user's HTTP proxy, `[user:pass@]host:port`; empty for none.
    pub const PROXY: &str = "popamp.proxy";
}

/// `MAJOR.MINOR`. Minor versions only add calls or optional fields.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Version {
    pub major: u16,
    pub minor: u16,
}

impl Version {
    /// True when a player of this version loads a plug-in built for `plugin`.
    pub fn loads(self, plugin: Version) -> bool {
        self.major == plugin.major && plugin.minor <= self.minor
    }
}

impl fmt::Display for Version {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}.{}", self.major, self.minor)
    }
}

impl FromStr for Version {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, String> {
        let bad = || format!("bad API version {s:?}, expected MAJOR.MINOR");
        let (major, minor) = s.split_once('.').ok_or_else(bad)?;
        Ok(Version { major: major.parse().map_err(|_| bad())?, minor: minor.parse().map_err(|_| bad())? })
    }
}

impl Serialize for Version {
    fn serialize<S: serde::Serializer>(&self, s: S) -> Result<S::Ok, S::Error> {
        s.collect_str(self)
    }
}

impl<'de> Deserialize<'de> for Version {
    fn deserialize<D: serde::Deserializer<'de>>(d: D) -> Result<Self, D::Error> {
        String::deserialize(d)?.parse().map_err(serde::de::Error::custom)
    }
}

/// What the blocks a `dsp` plug-in gets next are like.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Format {
    /// Frames per second. There are always two channels.
    pub rate: u32,
}

/// Reads a block of [`calls::PROCESS`]; trailing bytes short of a sample
/// are dropped.
pub fn samples_of(bytes: &[u8]) -> Vec<f32> {
    bytes.as_chunks::<4>().0.iter().map(|b| f32::from_le_bytes(*b)).collect()
}

/// Writes a block of [`calls::PROCESS`].
pub fn bytes_of(samples: &[f32]) -> Vec<u8> {
    samples.iter().flat_map(|s| s.to_le_bytes()).collect()
}

/// One track of a list.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Track {
    pub location: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub length_ms: Option<u64>,
}

/// Tags of a track; every field may be missing.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct Tags {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artist: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub album: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub year: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub genre: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub comment: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub length_ms: Option<u64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub bitrate_kbps: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub sample_rate: Option<u32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub channels: Option<u16>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub codec: Option<String>,
}

/// How the player plays a track; the plug-in never handles audio.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
// One answer per track opened: its size does not matter, and a box would
// only make plug-ins harder to write.
#[allow(clippy::large_enum_variant)]
#[serde(tag = "kind", rename_all = "lowercase", deny_unknown_fields)]
pub enum Play {
    /// The player opens the URL like any HTTP stream.
    Http { url: String },
    /// The player runs the manifest's `command` with these values and
    /// plays what it writes to standard output: interleaved stereo 32-bit
    /// float samples, little-endian, at `rate`. A `{start}` in the command
    /// is the position, when the track opens and on every seek; it is
    /// there only if the track is `seekable`.
    Pipe {
        command: String,
        #[serde(default)]
        vars: BTreeMap<String, String>,
        rate: u32,
        seekable: bool,
        #[serde(default)]
        tags: Tags,
    },
}

/// Input of [`host::EXEC`]. Standard input is closed.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecRequest {
    /// The `id` of a command in the manifest's `exec`.
    pub command: String,
    /// The values of the command's variables. One left out drops the part
    /// of the command it is in, if the manifest made that part optional.
    #[serde(default)]
    pub vars: BTreeMap<String, String>,
}

/// Output of [`host::EXEC`]. A command the manifest does not have, or a
/// value its variable's type refuses, fails the call itself.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecOutput {
    /// Why the program did not run to its end: not found, or stopped
    /// because the call's time was up. The other fields are empty then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// The exit code; `None` when the program was killed by a signal.
    pub status: Option<i32>,
    /// Standard output and error, as text (invalid UTF-8 is replaced).
    pub stdout: String,
    pub stderr: String,
}

/// Input of [`host::HTTP`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpRequest {
    pub method: String,
    /// `https://` or `http://`, at an address the manifest lists and the
    /// user left ticked. A port other than the usual one must be listed too.
    pub url: String,
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub body: Option<String>,
}

/// Output of [`host::HTTP`]. A redirect is not followed: it comes back as
/// it is, and going there is a new request, to an address that is listed.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HttpResponse {
    /// Why there is no answer: the host is not allowed, no connection, an
    /// answer too long. The other fields are empty then.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    pub status: u16,
    /// Names in lowercase.
    #[serde(default)]
    pub headers: BTreeMap<String, String>,
    /// As text (invalid UTF-8 is replaced).
    #[serde(default)]
    pub body: String,
}

/// Output of [`host::LIST`].
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Listing {
    /// Why the folder cannot be listed.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    /// Only what the plug-in may open.
    #[serde(default)]
    pub entries: Vec<Entry>,
}

/// A file or folder of a [`Listing`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct Entry {
    pub path: String,
    pub folder: bool,
    /// In bytes; 0 for a folder.
    pub size: u64,
}

/// Input of [`host::READ`].
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ReadRequest {
    pub path: String,
    #[serde(default)]
    pub offset: u64,
    /// At most 1 MiB comes back, however many are asked for.
    pub length: u32,
}

/// Output of [`host::READ`]; less than was asked for at the end of a file.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct FileData {
    /// Why the file cannot be read.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(default)]
    pub data: Vec<u8>,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_player_loads_plugins_of_its_major_version_and_no_newer_minor() {
        let v = |s: &str| s.parse::<Version>().unwrap();
        assert!(v("1.2").loads(v("1.0")));
        assert!(v("1.2").loads(v("1.2")));
        assert!(!v("1.2").loads(v("1.3")));
        assert!(!v("2.0").loads(v("1.0")));
        for bad in ["1", "1.x", "", "1.0.0"] {
            assert!(bad.parse::<Version>().is_err(), "{bad}");
        }
        assert_eq!(serde_json::to_string(&v("1.0")).unwrap(), r#""1.0""#);
    }

    #[test]
    fn the_crate_version_is_the_api_version() {
        let crate_api = format!("{}.{}", env!("CARGO_PKG_VERSION_MAJOR"), env!("CARGO_PKG_VERSION_MINOR"));
        assert_eq!(crate_api, VERSION.to_string());
    }

    #[test]
    fn play_answers_read_as_the_documented_json() {
        let pipe: Play =
            serde_json::from_str(r#"{"kind":"pipe","command":"play","vars":{"url":"https://a"},"rate":48000,"seekable":true}"#).unwrap();
        assert_eq!(
            pipe,
            Play::Pipe {
                command: "play".into(),
                vars: BTreeMap::from([("url".to_string(), "https://a".to_string())]),
                rate: 48000,
                seekable: true,
                tags: Tags::default()
            }
        );
        let http: Play = serde_json::from_str(r#"{"kind":"http","url":"http://a/b"}"#).unwrap();
        assert_eq!(http, Play::Http { url: "http://a/b".into() });
        assert!(serde_json::from_str::<Play>(r#"{"kind":"http","url":"u","extra":1}"#).is_err());
    }

    #[test]
    fn a_block_reads_back_as_written() {
        let block = [0.5, -1.0, 0.0, 0.25];
        let bytes = bytes_of(&block);
        assert_eq!(bytes[..4], 0.5f32.to_le_bytes());
        assert_eq!(samples_of(&bytes), block);
        assert_eq!(samples_of(&bytes[..6]), [0.5]);
        assert_eq!(serde_json::to_string(&Format { rate: 48000 }).unwrap(), r#"{"rate":48000}"#);
    }

    #[test]
    fn a_list_of_tracks_omits_what_is_unknown() {
        let t = Track { location: "x".into(), ..Default::default() };
        assert_eq!(serde_json::to_string(&Some(vec![t])).unwrap(), r#"[{"location":"x"}]"#);
        assert_eq!(serde_json::from_str::<Option<Vec<Track>>>("null").unwrap(), None);
    }
}
