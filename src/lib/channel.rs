// Pedagogical Impl of std::sync::mpsc::channel
// Crust of Rust: Channels, Gjenset 2020
// https://www.youtube.com/watch?v=b4mS5UPHh20

use std::sync::{Arc, Condvar, Mutex};
use std::collections::VecDeque;

// Channel flavours: 
//  - Synchronous channels (Bounded channels): send() can block. Bounded capacity.
//    - Mutex + Condvar + VecDeque
//    - Atomic VecDeque + thread::park + thread::Thread::notify
//  - Asynchronous channels (Unbounded channels): send() won't block because it has unbounded capacity.
//    - Mutex + Condvar + VecDeque
//    - Mutex + Condvar + LinkedList
//    - Atomic Linked List
//    - Atomic block linked list (LL<VecDeque<T>>)
//  - Rendezvous channels: A Bounded channel that has capacity 0. Send always blocks unless there is a 
//    blocking recv() waiting at that exact point. At which point both send() and recv()'s blocks cease,
//    hence a "rendezvous".
//  - Oneshot channels: Could be bounded or unbounded, but send() is onle ever called once.

pub struct Sender<T> {
    shared: Arc<Shared<T>>,
}

impl<T> Sender<T> {
    pub fn send(self: &mut Self, t: T) -> () {
        let mut behind_mutex = self.shared.mutex.lock().unwrap();
        behind_mutex.quque.push_back(t);  // <-- [1]
        drop(behind_mutex);
        self.shared.avail.notify_one();
        // In this implementation the sender cannot block. 
        // If data is sent at a greater rate than it is being consumed, 
        // The Vec grows without bound, and there is no backpressure.
        // Maybe, we want the producer to get blocked if the Vec reaches
        // a certain size. That is the std::sync::mpsc::SyncSender.
        // [1]: This inocuous push_back() is not free in a unbounded-queue
        // channel. If the Vec has capacity 16 and you push the 17th
        // element, this is expensive.
    }
}

// Should have (Sender: Clone) because "multiple producer"
impl<T> Clone for Sender<T> {
    fn clone(self: &Self) -> Self {
        let mut behind_mutex = self.shared.mutex.lock().unwrap();
        behind_mutex.senders_count += 1;
        drop(behind_mutex);
        Sender { shared: Arc::clone(&self.shared) }
    }
    // Clone cannot be derived because derive necessitates T: Clone
    // here, that constraint does not make sense, hence manual impl. 
}

impl<T> Drop for Sender<T> {
    fn drop(&mut self) {
        let mut behind_mutex = self.shared.mutex.lock().unwrap();
        behind_mutex.senders_count -= 1;
        let i_am_the_last = behind_mutex.senders_count == 0;
        drop(behind_mutex);
        // Must drop the mutex guard (free the mutex) before notifying.
        // Because the consumer will acquire the mutex after catching this signal, 
        // If by that time the mutex is still acquired then that's a race condition.
        if i_am_the_last { self.shared.avail.notify_one() }
        // After sending this notify_one(), this is the matching arm the consumer will use:
        //   match behind_mutex.queue.pop_front() {
        //      ...
        //      None if behind_mutex.senders_count == 0 => { return None }
        //      ...
        //  }
    }
}

pub struct Receiver<T> {
    shared: Arc<Shared<T>>,
    swap_buffer: VecDeque<T>,
    // [2] This swap_buffer is a commonplace optimization,
    // if we acquire the shared.mutex.queue, and it's not empty, then 
    // we might as well steal the entire queue and consume that one by one
    // This way we only ever need to lock the behind_mutex once for each 
    // queue. 
}

impl<T> Receiver<T> {
    pub fn recv(self: &mut Self) -> Option<T> {
        if let Some(t) = self.swap_buffer.pop_front() { return Some(t) }
        let mut behind_mutex = self.shared.mutex.lock().unwrap();
        loop {
            match behind_mutex.quque.pop_front() {
                Some(t) => {
                    // <-- [2]
                    if !behind_mutex.quque.is_empty() { std::mem::swap(&mut self.swap_buffer, &mut behind_mutex.quque) }
                    return Some(t)
                }
                None if behind_mutex.senders_count == 0 => { return None }
                // ^ In this case, we see that all senders are dropped. Reading further is
                // pointless and we convey that fact by returning None. 
                None => { behind_mutex = self.shared.avail.wait(behind_mutex).unwrap() }
                // ^ In this case, wait for the shared.avail signal. When the signal is raised,
                // we assume that the mutex is acquireable, and we acquire it, then restart the 
                // loop, and we see if we can do anything.
            }
        } 
    }
}

impl<T> Iterator for Receiver<T> {
    type Item = T;
    fn next(self: &mut Self) -> Option<Self::Item> {
        self.recv()
    }
}

struct Shared<T> {
    mutex: Mutex<BehindMutex<T>>,
    avail: Condvar,
}

struct BehindMutex<T> {
    quque: VecDeque<T>,
    senders_count: usize,
}

impl<T> Default for BehindMutex<T> {
    fn default() -> Self {
        BehindMutex { quque: VecDeque::new(), senders_count: 1 }
    }
}

pub fn channel<T>() -> (Sender<T>, Receiver<T>) {
    let shared = Shared { mutex: Mutex::default(), avail: Condvar::new() };
    let arc_shared = Arc::new(shared);
    return (
        Sender { shared: arc_shared.clone() },
        Receiver { shared: arc_shared.clone(), swap_buffer: VecDeque::new() }
    );
}

mod test {

    use super::*;

    #[derive(PartialEq, Eq, Debug, Clone, Copy)]
    struct SomeData<'a> {
        data_str: &'a str,
        data_number: i128,
        data_bool: bool,
        data_vec: &'a [&'a str],
    }

    #[test]
    fn ping_pong() {
        let (tx, mut rx) = channel::<()>();
        drop(tx);
        assert_eq!(rx.recv(), None);

        let (mut tx, mut rx) = channel();
        tx.send(42);
        assert_eq!(rx.recv(), Some(42));

        let somedata = SomeData {
            data_str: "frnersogvjeriosger",
            data_number: 27592479563726957697,
            data_bool: true,
            data_vec: &["tegtesht", "Getsghtrershb", "gtrsgwteht", "Gteshtrsjhrt"]
        };
        let (mut tx, mut rx) = channel();
        tx.send(somedata);
        assert_eq!(rx.recv(), Some(somedata));

    }
}