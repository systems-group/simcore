# SimCore

SimCore is discrete event simulation framework aimed to provide a solid foundation for building simulation models of distributed systems. The framework is built around a generic event-driven [programming model](examples/intro) that can be used to model different domains even beyond distributed systems. It allows to use both callbacks and asynchronous waiting to conveniently model any execution logic. The versatility of SimCore is demonstrated by using it for building several [simulation libraries](https://github.com/osukhoroslov/dslab/tree/main/crates) for research and education.

## Overview

SimCore supports writing arbitrary simulation models which consist of user-defined components emitting and processing events.

The simulation is configured and managed via `Simulation`, which includes methods for registering simulation components, stepping through the simulation, obtaining the current simulation time, etc. The library manages simulation state, which includes clock, event queue and random number generator. The latter is initialized with user-defined seed to ensure deterministic execution and reproduction of results.

It is possible to use any user-defined Rust types as simulation components. The components access simulation state and produce events via `SimulationContext`. Each component typically uses a unique simulation context, which allows to differentiate events produced by different components. To be able to consume events, the component should implement the `EventHandler` trait, which is invoked to pass events to the component. Each simulation component is registered with unique name and identifier, which can be used for specifying the event source or destination, logging purposes, etc.

The simulation represents a sequence of events. Each event has a unique identifier, timestamp, source, destination and user-defined payload. The library supports using arbitrary data types (implementing `Clone` and `Serialize` traits) as event payloads, the structure of payload is opaque to the library. The events are processed by retrieving the next event from the queue ordered by event timestamps, advancing the simulation clock to the event time and invoking the EventHandler implementation of component specified as the event destination. When processing the event, the component can create and emit new events with arbitrary future timestamps via its SimulationContext. The new events are placed in the event queue for further processing. It is also possible to cancel the previously emitted events before they are processed.

The library also provides convenient facilities for logging of events or arbitrary messages during the simulation with inclusion of component names, logging levels, etc.

## Example

```rust
use std::cell::RefCell;
use std::rc::Rc;
use serde::Serialize;
use simcore::{cast, Event, EventHandler, Id, Simulation, SimulationContext};

// Event data types (should implement Serialize)
#[derive(Clone, Serialize)]
struct Ping {
    info: f64,
}

#[derive(Clone, Serialize)]
struct Pong {
    info: f64,
}

// Simulation component types (here we have a single one - Process) 
struct Process {
    // generally components store the context,
    // without it they cannot emit events
    ctx: SimulationContext,
}

impl Process {
    pub fn new(ctx: SimulationContext) -> Self {
        Self { ctx }
    }

    fn send_ping(&self, dst: Id) {
        let info = self.ctx.time() + 0.5;
        // emit Ping event to another process with delay 0.5
        // info contains the expected event delivery time
        self.ctx.emit(Ping { info }, dst, 0.5);
    }
}

// To be able to consume events, the component should implement EventHandler trait
impl EventHandler for Process {
    // this method is invoked to deliver an event to the component 
    fn on(&mut self, event: Event) {
        // use cast! macro for convenient matching of event data types
        cast!(match event.data {
            Ping { info } => {
                // check that the current time equals the time in info
                assert_eq!(self.ctx.time(), info);
                let info = self.ctx.time() + 1.2;
                // emit Pong event back to another process with delay 1.2
                // info contains the expected event delivery time
                self.ctx.emit(Pong { info }, event.src, 1.2);
            }
            Pong { info } => {
                // check that the current time equals the time in info
                assert_eq!(self.ctx.time(), info);
            }
        })
    }
}

// Simulation setup and execution
fn main() {
    // create simulation with random seed
    let mut sim = Simulation::new(123);
    // create pinger, a Process component instance
    let pinger = Rc::new(RefCell::new(Process::new(sim.create_context("pinger"))));
    // register event handler for pinger
    let _pinger_id = sim.add_handler("pinger", pinger.clone());
    // create ponger, another Process component instance
    let ponger = Rc::new(RefCell::new(Process::new(sim.create_context("ponger"))));
    // register event handler for ponger
    let ponger_id = sim.add_handler("ponger", ponger.clone());
    // it is fine to call component methods directly instead of sending them events
    // here we ask pinger to send a Ping event to ponger
    pinger.borrow().send_ping(ponger_id);
    // run simulation until there are no pending events
    sim.step_until_no_events();
    // check current simulation time, should be equal to the time of last event
    assert_eq!(sim.time(), 1.7)
}
```

## License

Licensed under either of

* Apache License, Version 2.0, ([LICENSE-APACHE](LICENSE-APACHE) or https://www.apache.org/licenses/LICENSE-2.0)
* MIT license ([LICENSE-MIT](LICENSE-MIT) or https://opensource.org/licenses/MIT)

at your option.

### Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted for inclusion in the work by you, as defined in the Apache-2.0 license, shall be dual licensed as above, without any additional terms or conditions.
