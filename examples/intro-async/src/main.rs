use serde::Serialize;
use simcore::{cast, Event, Id, Simulation, SimulationContext, StaticEventHandler};
use std::rc::Rc;

// Event data types (must implement Clone and Serialize)
#[derive(Clone, Serialize)]
struct Request {}

#[derive(Clone, Serialize)]
struct Response {}

// Implementation of component which processes the above events
struct Process {
    net_delay: f64,
    // Generally components store the context inside to be able to emit events, etc.
    ctx: SimulationContext,
}

impl Process {
    pub fn new(net_delay: f64, ctx: SimulationContext) -> Self {
        Self { net_delay, ctx }
    }

    fn send_request(self: Rc<Self>, dst: Id) {
        // Spawn asynchronous activity for sending request and receiving response
        self.ctx.spawn(self.clone().send_request_and_get_response(dst))
    }

    async fn send_request_and_get_response(self: Rc<Self>, dst: Id) {
        let send_time = self.ctx.time();
        // Emit Request event to another process with network delay
        self.ctx.emit(Request {}, dst, self.net_delay);
        // Wait for response event
        self.ctx.recv_event::<Response>().await;
        // Calculate and print the response time
        let response_time = self.ctx.time() - send_time;
        println!("Response time: {:.2}", response_time);
    }

    async fn process_request(self: Rc<Self>, src: Id) {
        // Model random request processing time using sleep()
        self.ctx.sleep(self.ctx.gen_range(0.5..1.0)).await;
        // Emit Response event to another process with network delay
        self.ctx.emit(Response {}, src, self.net_delay);
    }
}

// When using async mode, components must implement the StaticEventHandler trait
impl StaticEventHandler for Process {
    // This method is invoked to deliver an event to the component
    // (only if such event is not currently awaited via async mode methods!)
    fn on(self: Rc<Self>, event: Event) {
        // Use cast! macro for convenient matching of event data types
        cast!(match event.data {
            Request {} => {
                // Spawn asynchronous activity for processing the request
                self.ctx.spawn(self.clone().process_request(event.src))
            }
        })
    }
}

fn main() {
    // Create simulation with random seed 123
    let mut sim = Simulation::new(123);

    // Create and register components
    let proc1 = Process::new(0.1, sim.create_context("proc1"));
    let proc1_ref = Rc::new(proc1);
    // When using async mode, components must register StaticEventHandler implementation
    // using the Simulation::add_static_handler method
    sim.add_static_handler("proc1", proc1_ref.clone());
    let proc2 = Process::new(0.1, sim.create_context("proc2"));
    let proc2_ref = Rc::new(proc2);
    let proc2_id = sim.add_static_handler("proc2", proc2_ref);

    // Ask proc1 to send request to proc2
    proc1_ref.send_request(proc2_id);

    // Run simulation until there are no pending events and print the final simulation time
    sim.step_until_no_events();
    println!("Simulation time: {:.2}", sim.time());
}
