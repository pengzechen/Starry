/// vuart buffer capacity
pub const BUF_CAP: usize = 256;

/// vuart
#[derive(Clone, Debug)]
pub struct Vuart {
    /// vuart id
    pub id: usize,
    /// vuart transmit fifo
    pub transmit_fifo: Bufqueue<BUF_CAP>,
    /// vuart receive fifo
    pub receive_fifo: Bufqueue<BUF_CAP>,
    /// vuart ris
    pub ris: u32,
    /// vuart icr
    pub icr: u32,
}

impl Vuart {
    /// Create a new Vuart
    pub fn new(id:usize) -> Self {
        let mut receive_fifo = Bufqueue::new();
        for c in "".chars() {
            receive_fifo.push(c as u8);
        }
        Self {
            id: id,
            transmit_fifo: Bufqueue::new(),
            receive_fifo: receive_fifo,
            ris: 0, 
            icr: 0,
        }
    }
}

/// fifo queue
#[derive(Clone, Debug)]
pub struct Bufqueue<const CAP: usize> {
    buffer: [u8; CAP],
    buffer_len: usize,
    buffer_head: usize,
    buffer_tail: usize,
}

impl<const CAP: usize> Bufqueue<CAP> {
    /// Create a new Bufqueue
    pub fn new() -> Self {
        Bufqueue {
            buffer: [0; CAP],
            buffer_len: 0,
            buffer_head: 0,
            buffer_tail: 0,
        }
    }

    /// Push an item to the queue
    pub fn push(&mut self, item: u8) {
        if self.buffer_len == CAP {
            self.buffer_tail = (self.buffer_tail + 1) % CAP;
        } else {
            self.buffer_len += 1;
        }
        self.buffer[self.buffer_head] = item;
        self.buffer_head = (self.buffer_head + 1) % CAP;
    }

    /// Pop an item from the queue
    pub fn pop(&mut self) -> u8 {
        if self.buffer_len == 0 {
            panic!("no item in the queue");
        } else {
            let item = self.buffer[self.buffer_tail];
            self.buffer_tail = (self.buffer_tail + 1) % CAP;
            self.buffer_len -= 1;
            item
        }
    }

    /// Get the length of the queue
    pub fn is_empty(&self) -> bool {
        self.buffer_len == 0
    }
}