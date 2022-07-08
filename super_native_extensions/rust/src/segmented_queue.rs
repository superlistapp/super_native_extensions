use std::{
    cell::{Cell, RefCell},
    sync::{Arc, Condvar, Mutex},
};

trait Segment {
    fn write(&self, data: &[u8]) -> Result<(), ()>;
    fn complete(&self);

    fn read(&self, max_len: usize) -> Vec<u8>;

    fn memory_used(&self) -> usize;
}

struct MemorySegmentInner {
    data: Vec<u8>,
    read_position: usize,
    completed: bool,
}

struct MemorySegment {
    inner: Mutex<MemorySegmentInner>,
    max_size: usize,
    condition: Condvar,
}

impl MemorySegment {
    fn new(max_size: usize) -> Self {
        Self {
            inner: Mutex::new(MemorySegmentInner {
                data: Vec::new(),
                read_position: 0,
                completed: false,
            }),
            condition: Condvar::new(),
            max_size,
        }
    }
}

impl Segment for MemorySegment {
    fn write(&self, data: &[u8]) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        if inner.completed || inner.data.len() >= self.max_size {
            return Err(());
        }
        inner.data.extend_from_slice(data);
        inner.completed |= inner.data.len() >= self.max_size;
        self.condition.notify_all();
        Ok(())
    }

    fn complete(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.completed = true;
        self.condition.notify_all();
    }

    fn read(&self, max_len: usize) -> Vec<u8> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if inner.read_position < inner.data.len() {
                let to_read = (inner.data.len() - inner.read_position).min(max_len);
                let res = &inner.data[inner.read_position..inner.read_position + to_read];
                let res = res.to_owned();
                inner.read_position += to_read;
                return res;
            } else if inner.completed {
                inner.data.clear();
                return Vec::new();
            } else {
                inner = self.condition.wait(inner).unwrap();
            }
        }
    }

    fn memory_used(&self) -> usize {
        self.inner.lock().unwrap().data.len()
    }
}

type BoxedSegment = Box<dyn Segment + Send + Sync>;

struct QueueStateInner {
    segments: Vec<Arc<BoxedSegment>>,
    completed: bool,
}

struct QueueState {
    inner: Mutex<QueueStateInner>,
    condition: Condvar,
}

impl QueueState {
    fn new() -> Self {
        Self {
            inner: Mutex::new(QueueStateInner {
                segments: Vec::new(),
                completed: false,
            }),
            condition: Condvar::new(),
        }
    }

    /// Blocks until segment at given index is available; Returns None if segment
    /// is not available and queue is completed.
    fn get_segment_at_index(&self, index: usize) -> Option<Arc<BoxedSegment>> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if index < inner.segments.len() {
                return Some(inner.segments[index].clone());
            } else if inner.completed {
                return None;
            } else {
                inner = self.condition.wait(inner).unwrap();
            }
        }
    }

    fn insert_segment(&self, segment: Arc<BoxedSegment>) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.completed {
            inner.segments.push(segment);
        }
        self.condition.notify_all();
    }

    fn complete(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.completed = true;
        self.condition.notify_all();
    }
}

pub struct SegmentedQueueReader {
    state: Arc<QueueState>,
    current_segment: Cell<usize>,
}

impl SegmentedQueueReader {
    fn new(state: Arc<QueueState>) -> Self {
        Self {
            state: state,
            current_segment: Cell::new(0),
        }
    }

    pub fn read_some(&self, max_len: usize) -> Vec<u8> {
        loop {
            let segment = self.state.get_segment_at_index(self.current_segment.get());
            match segment {
                Some(segment) => {
                    let data = segment.read(max_len);
                    if data.len() == 0 {
                        self.current_segment.replace(self.current_segment.get() + 1);
                    } else {
                        return data;
                    }
                }
                None => return Vec::new(),
            }
        }
    }

    pub fn read(&self, len: usize) -> Vec<u8> {
        let mut res = Vec::new();
        while res.len() < len {
            let to_read = len - res.len();
            let data = self.read_some(to_read);
            res.extend_from_slice(&data);
            if data.is_empty() {
                break;
            }
        }
        res
    }
}

pub struct SegmentedQueueWriter {
    memory_segment_max_size: usize,
    state: Arc<QueueState>,
    current_segment: RefCell<Arc<BoxedSegment>>,
}

impl SegmentedQueueWriter {
    fn new(state: Arc<QueueState>, memory_segment_max_size: usize) -> Self {
        let segment: BoxedSegment = Box::new(MemorySegment::new(memory_segment_max_size));
        let segment = Arc::new(segment);
        state.insert_segment(segment.clone());
        Self {
            memory_segment_max_size,
            state,
            current_segment: RefCell::new(segment),
        }
    }

    fn next_segment(&self) {
        let segment: BoxedSegment = Box::new(MemorySegment::new(self.memory_segment_max_size));
        let segment = Arc::new(segment);
        self.state.insert_segment(segment.clone());
        self.current_segment.replace(segment);
    }

    pub fn write(&self, data: &[u8]) {
        let segment = self.current_segment.borrow().clone();
        if segment.write(data).is_err() {
            self.next_segment();
            let segment = self.current_segment.borrow().clone();
            segment.write(data).expect("Fresh segment refused data");
        }
    }

    pub fn close(&self) {
        self.state.complete();
        self.current_segment.borrow().complete();
    }
}

pub fn new_segmented_queue(
    memory_segment_max_size: usize,
) -> (SegmentedQueueWriter, SegmentedQueueReader) {
    let state = Arc::new(QueueState::new());
    let writer = SegmentedQueueWriter::new(state.clone(), memory_segment_max_size);
    let reader = SegmentedQueueReader::new(state);
    (writer, reader)
}

#[cfg(test)]
mod test {
    #[test]
    fn test1() {
        println!("Hello");
    }
}
