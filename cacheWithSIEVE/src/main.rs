use std::collections::HashMap;
use std::fmt;
use core::hash::Hash;
use std::ptr;
use std::sync::RwLock;
use std::sync::atomic::{ AtomicPtr, Ordering };

fn main() {
    println!("Hello, world!");
}

struct Node<T> {
    val: Option<T>, 
    marked: bool,
    prev: AtomicPtr<Node<T>>,
    next: AtomicPtr<Node<T>>,
}

impl<T> Node<T> {
    fn new(val: Option<T>) -> Self {
        Node {
         val,
         marked: false,
         prev: AtomicPtr::new(ptr::null_mut()),
         next: AtomicPtr::new(ptr::null_mut()),
        }
    }
}

impl<T: fmt::Debug> Queue<T> {
    // Define a method to print all node data in the queue
    fn print_all_nodes(&self) {
        let mut current = self.head.load(Ordering::SeqCst);
        loop {
            if current == self.tail.load(Ordering::SeqCst) {
                break;
            }
            let node = unsafe { &*current };
            println!("{:?} {:?}", node.val, node.marked);
            current = node.next.load(Ordering::SeqCst);
        }
    }
}

struct Queue<T> {
    head: AtomicPtr<Node<T>>,
    tail: AtomicPtr<Node<T>>
}


impl<T> Queue<T> 
where 
T: Copy,
T: Ord,
{
    fn new() -> Self {
        let head = Box::new(Node::new(None));
        let tail = Box::new(Node::new(None));
        let head_ptr = Box::into_raw(head);
        let tail_ptr = Box::into_raw(tail);
        unsafe {
            (*head_ptr).next.store(tail_ptr, Ordering::SeqCst);
            (*tail_ptr).prev.store(head_ptr, Ordering::SeqCst);
        }
        Queue {
            head: AtomicPtr::new(head_ptr),
            tail: AtomicPtr::new(tail_ptr)
        }
    }

    fn insert(&self, val: T) -> Option<*mut Node<T>> {
       let new_node = Box::new(Node::new(Some(val)));
       new_node.prev.store(self.head.load(Ordering::SeqCst), Ordering::SeqCst);

       new_node.next.store(unsafe { &*self.head.load(Ordering::SeqCst) }.next.load(Ordering::SeqCst), Ordering::SeqCst);

       // not thread safe 
       let new_node_ptr = Box::into_raw(new_node);
       let current_next = unsafe { &*self.head.load(Ordering::SeqCst) }.next.load(Ordering::SeqCst);
       unsafe { &*current_next }.prev.store(new_node_ptr, Ordering::SeqCst);
       unsafe { &*self.head.load(Ordering::SeqCst) }.next.store(new_node_ptr, Ordering::SeqCst);
       Some(new_node_ptr) 
    }

    fn get(&self, val: T) -> Option<T> {
        let mut curr = self.head.load(Ordering::SeqCst);
        loop {
            if curr == self.tail.load(Ordering::SeqCst) {
                return None;
            }
            if unsafe { &*curr }.val.map(|k| k == val).unwrap_or(false) {
                return Some(val);
            }

            curr = unsafe { &*curr }.next.load(Ordering::SeqCst);
        }
    }

    fn delete(&self, val: T) -> Option<T> {
        let mut curr = self.head.load(Ordering::SeqCst);
        let mut pre= self.head.load(Ordering::SeqCst);
        loop {
            if curr == self.tail.load(Ordering::SeqCst) {
                return None;
            }
            if unsafe { &*curr }.val.map(|k| k == val).unwrap_or(false) {
                let next_node = unsafe { &*curr }.next.load(Ordering::SeqCst);
                unsafe { &*pre }.next.store(next_node, Ordering::SeqCst);
                unsafe { &*next_node }.prev.store(pre, Ordering::SeqCst);
                return Some(val);
            }

            pre = curr;
            curr = unsafe { &*curr }.next.load(Ordering::SeqCst);
        }
        

    }
}

struct SieveCache<T> {
    cap: usize,
    store: RwLock<HashMap<T, AtomicPtr<Node<T>>>>,
    queue: Queue<T>,
    size: usize,
    hand: AtomicPtr<Node<T>>,
}

impl<T> SieveCache<T> 
where 
T: Copy,
T: Ord, 
T: Hash,
{
    fn new(cap: usize) -> Self {
        SieveCache {
            cap,
            store: RwLock::new(HashMap::new()),
            queue: Queue::new(),
            size: 0,
            hand: AtomicPtr::new(ptr::null_mut()),
        }
    }

    fn get(&self, k: T) -> Option<T> {
        let guard = self.store.read().unwrap();
        let value = guard.get(&k);
        if let Some(ptr) = value {
            let node = ptr.load(Ordering::SeqCst);
            unsafe { &mut *node }.marked = true;
            return Some(k);
        } else {
            None
        }

    }

    fn evict(&mut self) {

        let mut hand = self.hand.load(Ordering::SeqCst);
      
        loop {

          if hand == self.queue.head.load(Ordering::SeqCst) || hand.is_null() {
            hand = self.queue.tail.load(Ordering::SeqCst);
          }
          if hand != self.queue.tail.load(Ordering::SeqCst) && !unsafe { &*hand }.marked {
            let k = unsafe { &*hand }.val.unwrap();

            self.store.write().unwrap().remove(&k);
            self.queue.delete(k);
            break;
          }
          if hand != self.queue.tail.load(Ordering::SeqCst) {
            unsafe { &mut *hand }.marked = false;
            // Cannot assign to `&*hand`.marked because it is behind a `&` reference
            // This is a `&` reference, so the data it refers to cannot be written
            // Consider using a mutable reference to modify the data
          }
          hand = unsafe { &*hand }.prev.load(Ordering::SeqCst);

        }
        let new_hand = hand;
        self.hand.store(new_hand, Ordering::SeqCst);
        // self.store.write().unwrap().remove(&k);
        // self.queue.delete(k);
        // Cannot assign to `self.size` because it is behind a `&` reference
        // This is a `&` reference, so the data it refers to cannot be written
        // Consider using a mutable reference to modify the data
        // self.size = self.size - 1;

    }

    fn insert(&mut self, k: T) {
        if self.size == self.cap {
            self.evict();
            let node = self.queue.insert(k);
            if let Some(valid) = node {
            self.store.write().unwrap().insert(k, AtomicPtr::new(valid));

            }
            
            
            return;
        }
        self.size += 1;
        let node = self.queue.insert(k);
        if let Some(valid) = node {
        self.store.write().unwrap().insert(k, AtomicPtr::new(valid));
      }
    }
}

impl<T: fmt::Debug> fmt::Debug for SieveCache<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        // Print the fields you want to include in the debug representation
        write!(f, "SieveCache {{ cap: {:?}, size: {:?}, hand: {:?}, queue: ", self.cap, self.size, self.hand)?;
        // Traverse the queue and print its contents
        // Assuming Queue<T> has a method to traverse its contents
        // Replace `traverse_method` with the actual method name
        write!(f, "{:?}", self.queue.print_all_nodes())?;
        write!(f, " }}")
    }
}


#[cfg(test)]
mod tests {
    

    use super::*;

    #[test]
    fn basic_test() {
        let mut cache = SieveCache::new(3);
        cache.insert(1);
        cache.insert(2);
        cache.insert(3);
        cache.insert(4);
        println!("{:?}",cache);
    }

}



