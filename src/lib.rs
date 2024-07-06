//! SimCore is discrete event simulation framework aimed to provide a solid foundation for building simulation models
//! of distributed systems. The framework is built around a generic event-driven
//! [programming model](https://github.com/systems-group/simcore/tree/main/examples/intro) that can be used to model
//! different domains even beyond distributed systems. It allows to use both callbacks and asynchronous waiting to
//! conveniently model any execution logic. The versatility of SimCore is demonstrated by using it for building several
//! [simulation libraries](https://github.com/osukhoroslov/dslab/tree/main/crates) for research and education.

#![warn(missing_docs)]
#![allow(clippy::needless_doctest_main)]

pub mod async_mode;
pub mod component;
pub mod context;
pub mod event;
pub mod handler;
pub mod log;
pub mod simulation;
mod state;

pub use colored;
pub use component::Id;
pub use context::SimulationContext;
pub use event::{Event, EventData, EventId, TypedEvent};
pub use handler::{EventCancellationPolicy, EventHandler};
pub use simulation::Simulation;
pub use state::EPSILON;

async_mode_enabled!(
    pub use handler::StaticEventHandler;
);
