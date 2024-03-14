//! Implement the schedulers in this module
//!
//! You might want to create separate files
//! for each scheduler and export it here
//! like
//!
//! ```ignore
//! mod scheduler_name
//! pub use scheduler_name::SchedulerName;
//! ```
//!

// TODO delete this example
mod empty;
mod round_robinn;
pub use empty::Empty;
pub use round_robinn::RoundRobinScheduler;
// TODO import your schedulers here
