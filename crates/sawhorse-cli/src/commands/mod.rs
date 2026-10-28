//! Subcommand definitions for `sawhorse-cli`.

pub mod bench;
pub mod complete;
pub mod init;
pub mod ping;

pub use bench::run_bench;
pub use complete::run_complete;
pub use init::run_init;
pub use ping::run_ping;
