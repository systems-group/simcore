//! SimCore is discrete event simulation framework aimed to provide a solid foundation for building simulation models
//! of distributed systems. The framework is built around a generic event-driven programming model that can be used to
//! model different domains even beyond distributed systems. It allows to use both callbacks and asynchronous waiting to
//! conveniently model any execution logic.
//!
//! ## Contents
//!
//! - [Basic Concepts](crate#header)
//! - [Example](crate#example)
//! - [Programming Interfaces](crate#programming-interfaces)
//! - [Receiving Events via Callbacks](crate#receiving-events-via-callbacks)
//! - [Async Mode](crate#async-mode)
//!
//! ## Basic Concepts
//!
//! SimCore supports writing arbitrary _simulation models_ which consist of user-defined _components_ emitting and
//! processing _events_.
//!
//! **Component.** A component represents a part of the model with some internal state and execution logic. Each
//! component is assigned a unique _identifier_ which can be used to emit events for this component. A component can
//! access simulation state, emit or wait for events via _context_ provided by the framework. The model execution is
//! driven by the framework which calls components upon occurrence of events. The called component can examine the
//! received event, read the current simulation time, modify internal state and emit new events according to the code
//! written by the user. The components are added by creating named contexts and registering _event handlers_. The
//! former is required for components which emit events, while the latter is required for components which process
//! events.
//!
//! **Event.** An event contains a timestamp, identifiers of event source and destination, and a used-defined _payload_.
//! The event timestamp corresponds to the simulation time at which the event is supposed to occur. This time must be
//! specified when the event is emitted. Due to performance reasons, event timestamps cannot be changed. However, an
//! event can be canceled before it occurred and rescheduled by creating a new event. Each event is associated with
//! exactly one destination component. The framework allows to model different types of events by using arbitrary data
//! structures as event payloads. The event payload is opaque to the framework and is passed in a zero-copy fashion
//! between the event source and destination.
//!
//! The initial set of events is created before the simulation start via some of the components. For example, in case
//! of a trace-driven simulation, a dedicated component can be used as a source of external events from a trace.
//!
//! **Simulation.** Following the [discrete-event simulation](https://en.wikipedia.org/wiki/Discrete-event_simulation)
//! approach, the execution of a user-defined model is implemented by processing a sequence of events emitted by the
//! model components. The framework processes events in their timestamp order by  advancing the simulation clock to the
//! event's timestamp and invoking the component specified as the event destination. When processing the event, the
//! component can create and emit new events with arbitrary future timestamps via its context. It is also possible to
//! cancel the previously emitted events before they are processed.
//!
//! The described approach for building simulation models is chosen based on the following considerations.
//!
//! First, it suits well for modeling _distributed systems_. Indeed, such systems are frequently modeled as a set of
//! _processes_ which communicate with each other by sending _messages_ via a network. In such models, events can be
//! either internal process events or receptions of messages from other processes. The described approach allows to
//! model both types of events and fits well to message passing - a message can be sent to another process by simply
//! emitting an event to the corresponding component with delay equal to the message transmission time.
//!
//! Second, the described model is abstract and flexible enough to support different simulation needs, even beyond
//! distributed systems. If the framework were instead based on a more specific and restricted model, such as message
//! passing, it would complicate the modeling of other activities, such as computations. An alternative approach chosen
//! by some frameworks is to provide a predefined set of built-in activities and events. However, this would introduce
//! a trade-off between the ease of use for modeling specific types of systems and the flexibility of the framework.
//!
//! We overcome this trade-off by keeping the framework as general as possible and by building separate libraries with
//! domain-specific models on top of it. This allows the users to choose only features they need and to create new
//! libraries when some features are missing without bloating the framework. For example, there is no notion of
//! processes, hosts and network in SimCore. Such abstractions and their models can be added if needed via separate
//! libraries. Depending on a purpose, some users may need a complex network model, while for others a fixed delay
//! supported by the framework is sufficient.
//!
//! ## Example
//!
//! This example demonstrates the use of SimCore programming interfaces and receiving events via callbacks. See the
//! next sections for details and an alternative to callback-based approach.
//!
//! ```rust
//! use std::cell::RefCell;
//! use std::rc::Rc;
//! use serde::Serialize;
//! use simcore::{cast, Event, EventHandler, Id, Simulation, SimulationContext};
//!
//! // Event data types (must implement Clone and Serialize)
//! #[derive(Clone, Serialize)]
//! struct Request {
//!     time: f64,
//! }
//!
//! #[derive(Clone, Serialize)]
//! struct Response {
//!     req_time: f64,
//! }
//!
//! // Implementation of component which processes the above events
//! struct Process {
//!     net_delay: f64,
//!     // Generally components store the context inside to be able to emit events, etc.
//!     ctx: SimulationContext,
//! }
//!
//! impl Process {
//!     pub fn new(net_delay: f64, ctx: SimulationContext) -> Self {
//!         Self { net_delay, ctx }
//!     }
//!
//!     fn send_request(&self, dst: Id) {
//!         // Emit Request event to another process with network delay
//!         self.ctx.emit(Request { time: self.ctx.time() }, dst, self.net_delay);
//!     }
//!
//!     fn on_request(&self, src: Id, req_time: f64) {
//!         // Generate the random request processing delay
//!         let proc_delay = self.ctx.gen_range(0.5..1.0);
//!         // Emit Response event to another process with processing + network delay
//!         self.ctx.emit(Response { req_time }, src, proc_delay + self.net_delay);
//!     }
//!
//!     fn on_response(&self, req_time: f64) {
//!         // Calculate and print the response time
//!         let response_time = self.ctx.time() - req_time;
//!         println!("Response time: {:.2}", response_time);
//!     }
//! }
//!
//! // Components can receive events by implementing EventHandler trait
//! impl EventHandler for Process {
//!     // This method is invoked to deliver an event to the component
//!     fn on(&mut self, event: Event) {
//!         // Use cast! macro for convenient matching of event data types
//!         cast!(match event.data {
//!             Request { time } => {
//!                 self.on_request(event.src, time)
//!             }
//!             Response { req_time } => {
//!                 self.on_response(req_time)
//!             }
//!         })
//!     }
//! }
//!
//! fn main() {
//!     // Create simulation with random seed 123
//!     let mut sim = Simulation::new(123);
//!
//!     // Create and register components
//!     let proc1 = Process::new(0.1, sim.create_context("proc1"));
//!     let proc1_ref = Rc::new(RefCell::new(proc1));
//!     sim.add_handler("proc1", proc1_ref.clone());
//!     let proc2 = Process::new(0.1, sim.create_context("proc2"));
//!     let proc2_ref = Rc::new(RefCell::new(proc2));
//!     let proc2_id = sim.add_handler("proc2", proc2_ref);
//!
//!     // Ask proc1 to send request to proc2
//!     proc1_ref.borrow().send_request(proc2_id);
//!
//!     // Run simulation until there are no pending events and print the final simulation time
//!     sim.step_until_no_events();
//!     println!("Simulation time: {:.2}", sim.time());
//! }
//! ```
//!
//! ## Programming Interfaces
//!
//! [`Simulation`] is the main interface of the framework which allows to configure and execute a simulation model. As
//! demonstrated in the example above, it can be instantiated with a user-defined random seed and then used to create
//! simulation contexts and register event handlers for components of user-defined type `Process`, run the simulation
//! and obtain the current simulation time. Besides the [`step_until_no_events`](crate::Simulation::step_until_no_events)
//! method, it provides other methods for precise stepping through the simulation. It also provides access to the
//! simulation-wide random number generator which is initialized with the user-defined seed to support deterministic
//! simulations.
//!
//! [`SimulationContext`] is the interface for accessing the simulation state and emitting events from components. Each
//! component is associated with a uniquely named context which is created via the
//! [`Simulation::create_context`](crate::Simulation::create_context) method. The context is typically passed to the
//! component's constructor and is stored inside the component as illustrated in the example above. This example also
//! illustrates the use of the stored context to emit the user-defined events `Request` and `Response`, to obtain the
//! current simulation time, and to generate random numbers using the simulation-wide generator.
//!
//! SimCore allows a user to keep a reference to a component to call it directly, as illustrated by `proc1_ref` in the
//! example above. Moving components completely inside the framework and allowing to interact with them only via events
//! or framework interfaces would harm the usability. It would be more cumbersome to emit a special event to `proc1`
//! instead of calling `send_request` method. This also allows to easily inspect component states during the simulation.
//!
//! The same observation applies to the interaction between components - if immediate request/response is assumed, it
//! is both more convenient and efficient to interact via direct calls instead of events. For example, a component
//! modeling CPU can be called directly by other components running on the same simulated machine to request a
//! computation. In response, the CPU component can return the request handle and notify the requester via an event
//! when the computation is completed. Therefore, the framework does not restrict the interaction with and between
//! components to happen only via events. This is in contrast to similar but more strict models such as actor model for
//! message passing.
//!
//! The described interfaces deal only with calling SimCore from a user's code. However, the framework should also be
//! able to call user's components to notify them about occurred events. There are two supported approaches for
//! programming this logic described below.
//!
//! ## Receiving Events via Callbacks
//!
//! The default approach for receiving events in components is based on implementing the [`EventHandler`] interface.
//! This interface contains a single [`on`](crate::EventHandler::on) method which is called by the framework to pass an
//! event to the destination component. This approach is illustrated in the example above where the `Process` component
//! implements this interface to receive `Request` and `Response` events. The pattern matching syntax is used to
//! identify the type of received event. When a component implements the [`EventHandler`] interface it must be
//! registered in the framework via the [`Simulation::add_handler`](crate::Simulation::add_handler) method.
//!
//! Consider in detail the provided example. It describes a simulation model consisting of two components `proc1` and
//! `proc2`. The behavior of these components is defined by the `Process` type. This type implements the
//! [`EventHandler`] interface to receive and process events of two types: `Request` and `Response`:
//!
//! - The logic for processing `Request` is defined in the `on_request` callback method - the process emits `Response`
//! to the source of `Request` with some delay including the random request processing time and the network delay. The
//! request sending time stored in `Request` is copied to the corresponding `Response`.
//!
//! - The logic for processing `Response` is defined in the `on_response` callback method - the process reads the
//! request time from `Response` to calculate and print the response time, i.e. the time elapsed between the sending
//! of request and receiving the response.
//!
//! The process implementation also includes the `send_request` method to trigger emitting of `Request` to another
//! process.
//!
//! The example models a simple scenario where `proc1` emits a request to `proc2` and the simulation runs until `proc1`
//! receives a response.
//!
//! ### Limitations of Callbacks
//!
//! While the callback-based approach is simple and intuitive by organizing all event processing logic in `EventHandler`,
//! it may also complicate the implementation of a more complex logic inside components. In particular, when modeling
//! some multistep activity, where each step requires awaiting some events, these steps should be spread across several
//! event handler functions. This makes the implementation of such complex activities more verbose and hard to follow.
//!
//! For example, in the provided example, the sending of request and receiving of response are split into two separate
//! methods, while it would be more convenient to `await` a response event in the code immediately after sending the
//! request. This also complicates the calculation of response time because, in order to do it in `on_response` callback
//! method, the request sending time should be passed inside events or stored inside the process.
//!
//! Also, the random processing time is modeled in `on_request` by simply adding it to the response event delay, while
//! it would be more natural to `sleep` for this time inside the code before emitting the response. The trick with delay
//! would also not work when the processing time is not known in advance. For example, the processing of request may
//! include some computation which completion is determined by a separate model and signaled to the process via an event.
//! In this case, the request processing logic should also be split into several methods making it harder to follow.
//!
//! ## Async Mode
//!
//! To overcome the described limitations of callback-based approach, the SimCore interfaces have been enriched with
//! primitives for spawning asynchronous activities and awaiting events and timers. This functionality, dubbed
//! _async mode_, is implemented as an optional feature that can be enabled by a user and used in conjunction with
//! the callback-based approach.
//!
//! The code below illustrates the use of async mode to improve the previously described callback-based implementation.
//!
//! ```rust
//! use std::rc::Rc;
//! use serde::Serialize;
//! use simcore::{cast, Event, Id, Simulation, SimulationContext, StaticEventHandler};
//!
//! // Event data types (must implement Clone and Serialize)
//! #[derive(Clone, Serialize)]
//! struct Request {}
//!
//! #[derive(Clone, Serialize)]
//! struct Response {}
//!
//! // Implementation of component which processes the above events
//! struct Process {
//!     net_delay: f64,
//!     // Generally components store the context inside to be able to emit events, etc.
//!     ctx: SimulationContext,
//! }
//!
//! impl Process {
//!     pub fn new(net_delay: f64, ctx: SimulationContext) -> Self {
//!         Self { net_delay, ctx }
//!     }
//!
//!     fn send_request(self: Rc<Self>, dst: Id) {
//!         // Spawn asynchronous activity for sending request and receiving response
//!         self.ctx.spawn(self.clone().send_request_and_get_response(dst))
//!     }
//!
//!     async fn send_request_and_get_response(self: Rc<Self>, dst: Id) {
//!         let send_time = self.ctx.time();
//!         // Emit Request event to another process with network delay
//!         self.ctx.emit(Request {}, dst, self.net_delay);
//!         // Wait for response event
//!         self.ctx.recv_event::<Response>().await;
//!         // Calculate and print the response time
//!         let response_time = self.ctx.time() - send_time;
//!         println!("Response time: {:.2}", response_time);
//!     }
//!
//!     async fn process_request(self: Rc<Self>, src: Id) {
//!         // Model random request processing time using sleep()
//!         self.ctx.sleep(self.ctx.gen_range(0.5..1.0)).await;
//!         // Emit Response event to another process with network delay
//!         self.ctx.emit(Response {}, src, self.net_delay);
//!     }
//! }
//!
//! // When using async mode, components must implement the StaticEventHandler trait
//! impl StaticEventHandler for Process {
//!     // This method is invoked to deliver an event to the component
//!     // (only if such event is not currently awaited via async mode methods!)
//!     fn on(self: Rc<Self>, event: Event) {
//!         // Use cast! macro for convenient matching of event data types
//!         cast!(match event.data {
//!             Request {} => {
//!                 // Spawn asynchronous activity for processing the request
//!                 self.ctx.spawn(self.clone().process_request(event.src))
//!             }
//!         })
//!     }
//! }
//!
//! fn main() {
//!     // Create simulation with random seed 123
//!     let mut sim = Simulation::new(123);
//!
//!     // Create and register components
//!     let proc1 = Process::new(0.1, sim.create_context("proc1"));
//!     let proc1_ref = Rc::new(proc1);
//!     // When using async mode, components must register StaticEventHandler implementation
//!     // using the Simulation::add_static_handler method
//!     sim.add_static_handler("proc1", proc1_ref.clone());
//!     let proc2 = Process::new(0.1, sim.create_context("proc2"));
//!     let proc2_ref = Rc::new(proc2);
//!     let proc2_id = sim.add_static_handler("proc2", proc2_ref);
//!
//!     // Ask proc1 to send request to proc2
//!     proc1_ref.send_request(proc2_id);
//!
//!     // Run simulation until there are no pending events and print the final simulation time
//!     sim.step_until_no_events();
//!     println!("Simulation time: {:.2}", sim.time());
//! }
//! ```
//!
//! First, the sending of request and receiving of response are now conveniently located in a single
//! `send_request_and_get_response` method. This method represents the asynchronous activity spawned in `send_request`
//! via [`SimulationContext::spawn`](crate::SimulationContext::spawn). Waiting for response event inside this activity
//! is implemented via [`SimulationContext::recv_event`](crate::SimulationContext::recv_event) method, which returns a
//! future that can be awaited without blocking the simulation. Collocating the request-response logic inside a single
//! method allows to calculate the response time without having to pass the request time inside events.
//!
//! Second, the request processing is now modeled in `process_request` method which represents the asynchronous activity
//! spawned upon receiving of request. The random request processing time is modeled in `process_request` by calling the
//! [`SimulationContext::sleep`](crate::SimulationContext::sleep) method, which allows to suspend the component
//! execution for a specified time.
//!
//! The code for configuring and running the simulation is slightly changed. To be able to spawn asynchronous
//! activities, components must implement the special [`StaticEventHandler`] trait and register its implementation
//! using the [`Simulation::add_static_handler`](crate::Simulation::add_static_handler) method.
//!
//! As demonstrated, the async mode eliminates the described limitations of the callback-based approach. This example
//! also illustrates that SimCore allows to use both approaches simultaneously to combine their advantages. While
//! callbacks are convenient for describing a simple event processing logic or receiving events triggering a complex
//! logic, the latter can be conveniently described using the async mode primitives.
//!
//! Another notable feature of async mode is the support for selective receive of events by a user-defined key (see
//! [`SimulationContext::recv_event_by_key`](crate::SimulationContext::recv_event_by_key)). This is convenient in cases
//! when component performs multiple asynchronous activities, and each activity must wait for events of the same type.
//! It is also possible to wait for multiple events simultaneously using the `join` and `select` primitives from the
//! [futures](https://crates.io/crates/futures) crate.
//!
//! On the downside, async mode has additional performance overhead in comparison to callbacks. The observed slowdown
//! depends on an application and is around 10-50% according to our experience.

#![warn(missing_docs)]
#![allow(clippy::needless_doctest_main)]
#![cfg_attr(docsrs, feature(doc_auto_cfg))]

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
