//! Bounce Tales clone. The library holds everything that can run without a
//! window (simulation, replays, CLI parsing) so integration tests can use it.

pub mod cli;
pub mod level;
pub mod replay;
pub mod sim;
pub mod tileset;
