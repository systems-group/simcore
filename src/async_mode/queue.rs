//! Queue for producer-consumer communication between asynchronous tasks.

use std::cell::RefCell;
use std::collections::VecDeque;
use std::future::Future;
use std::pin::Pin;
use std::rc::Rc;
use std::task::{Context, Poll};

use rustc_hash::FxHashSet;
use serde::Serialize;

use crate::SimulationContext;

/// A simple implementation of unbounded multi-producer multi-consumer queue with items of type `T`.
///
/// The items are guarantied to be delivered to consumers in the order of [`take`](UnboundedQueue::take) calls.
pub struct UnboundedQueue<T> {
    items: RefCell<VecDeque<T>>,
    send_ticket: Ticket,
    receive_ticket: Ticket,
    dropped_tickets: Rc<RefCell<FxHashSet<TicketID>>>,
    ctx: SimulationContext,
}

impl<T> UnboundedQueue<T> {
    pub(crate) fn new(ctx: SimulationContext) -> Self {
        ctx.register_key_getter_for::<ConsumerNotify>(|notify| notify.ticket_id);
        Self {
            items: RefCell::new(VecDeque::new()),
            send_ticket: Ticket::new(),
            receive_ticket: Ticket::new(),
            dropped_tickets: Rc::new(RefCell::new(FxHashSet::default())),            
            ctx,
        }
    }

    /// Inserts the specified item into the queue without blocking.
    pub fn put(&self, item: T) {
        self.send_ticket.next();
        let mut dropped_tickets = self.dropped_tickets.borrow_mut();
        while dropped_tickets.remove(&self.send_ticket.value()) {
            self.send_ticket.next();
        }
        self.items.borrow_mut().push_back(item);
        // notify awaiting consumer if needed
        if self.receive_ticket.is_after(&self.send_ticket) {
            self.ctx.emit_self_now(ConsumerNotify {
                ticket_id: self.send_ticket.value(),
            });
        }
    }

    /// Removes the head of the queue and returns it, waiting if necessary until an item becomes available.
    ///
    /// This function is asynchronous and its result (future) must be awaited.
    /// If multiple consumers are waiting for item, the items will be delivered in the order of [`take`](Self::take) calls.
    pub async fn take(&self) -> T {
        self.receive_ticket.next();
        ElementFutureWrapper::from_future(
            async {
                // wait for notification from producer side if the queue is empty
                if self.items.borrow().is_empty() {
                    self.ctx
                        .recv_event_by_key_from_self::<ConsumerNotify>(self.receive_ticket.value())
                        .await;
                }
                self.items.borrow_mut().pop_front().unwrap()
            },
            self.receive_ticket.value(),
            self.dropped_tickets.clone(),
        )
        .await
    }
}

type TicketID = u64;

#[derive(Serialize, Clone)]
struct ConsumerNotify {
    ticket_id: TicketID,
}

struct Ticket {
    value: RefCell<TicketID>,
}

impl Ticket {
    fn new() -> Self {
        Self { value: RefCell::new(0) }
    }

    fn next(&self) {
        *self.value.borrow_mut() += 1;
    }

    fn is_after(&self, other: &Self) -> bool {
        *self.value.borrow() >= *other.value.borrow()
    }

    fn value(&self) -> TicketID {
        *self.value.borrow()
    }
}

struct ElementFutureWrapper<'a, T> {
    element_future: Pin<Box<dyn Future<Output = T> + 'a>>,
    ticket_id: TicketID,
    dropped_tickets: Rc<RefCell<FxHashSet<TicketID>>>,    
    completed: bool,
}

impl<'a, T> ElementFutureWrapper<'a, T> {
    fn from_future(
        element_future: impl Future<Output = T> + 'a,
        ticket_id: TicketID,
        dropped_tickets: Rc<RefCell<FxHashSet<TicketID>>>,
    ) -> Self {
        Self {
            element_future: Box::pin(element_future),
            ticket_id,
            dropped_tickets,            
            completed: false,
        }
    }
}

impl<'a, T> Future for ElementFutureWrapper<'a, T> {
    fn poll(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        match self.element_future.as_mut().poll(cx) {
            Poll::Ready(output) => {
                self.completed = true;
                Poll::Ready(output)
            }
            Poll::Pending => Poll::Pending,
        }
    }

    type Output = T;
}

impl<'a, T> Drop for ElementFutureWrapper<'a, T> {
    fn drop(&mut self) {
        if !self.completed {
            self.dropped_tickets.borrow_mut().insert(self.ticket_id);
        }
    }
}
