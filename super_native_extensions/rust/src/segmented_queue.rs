use std::{
    cell::{Cell, RefCell},
    env,
    fs::{File, OpenOptions},
    io,
    ops::Deref,
    path::PathBuf,
    sync::{Arc, Condvar, Mutex},
};

use rand::{distributions::Alphanumeric, Rng};

use crate::log::OkLog;

trait Segment {
    /// Writes data to segment. Error is returned if segment already reached
    /// or exceeded its capacity.
    fn write(&self, data: &[u8]) -> Result<(), ()>;

    /// Marks segment as completed. Unblock reader.
    fn complete(&self);

    /// Blocking read from segment. Blocks until any data is available
    /// or segment is completed, in which case returns empty data.
    fn read(&self, max_len: usize) -> Vec<u8>;

    /// Returns the amount of memory used.
    fn memory_used(&self) -> usize;
}

struct MemorySegmentInner {
    data: Vec<u8>,
    read_position: usize,
    completed: bool,
}

/// In memory implementation of [Segment].
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

struct FileHolder {
    file: File,
    path: PathBuf,
}

impl FileHolder {
    fn new_temporary() -> Self {
        let path = Self::temp_path();
        Self {
            file: OpenOptions::new()
                .create(true)
                .read(true)
                .write(true)
                .truncate(true)
                .open(&path)
                .unwrap(),

            path,
        }
    }

    fn temp_path() -> PathBuf {
        let temp_dir = env::temp_dir();
        let file_name: String = rand::thread_rng()
            .sample_iter(&Alphanumeric)
            .take(30)
            .map(char::from)
            .collect();
        temp_dir.join(file_name)
    }
}

impl Drop for FileHolder {
    fn drop(&mut self) {
        std::fs::remove_file(&self.path).ok_log();
    }
}

impl Deref for FileHolder {
    type Target = File;

    fn deref(&self) -> &Self::Target {
        &self.file
    }
}

struct FileSegmentInner {
    file: Option<FileHolder>,
    write_position: u64,
    read_position: u64,
    completed: bool,
}

struct FileSegment {
    max_file_length: u64,
    inner: Mutex<FileSegmentInner>,
    condition: Condvar,
}

impl FileSegment {
    fn new(max_file_length: u64) -> FileSegment {
        FileSegment {
            max_file_length,
            inner: Mutex::new(FileSegmentInner {
                file: Some(FileHolder::new_temporary()),
                read_position: 0,
                write_position: 0,
                completed: false,
            }),
            condition: Condvar::new(),
        }
    }

    #[cfg(target_family = "windows")]
    fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::os::windows::prelude::FileExt;
        file.seek_read(buf, offset)
    }

    #[cfg(target_family = "unix")]
    fn read_at(file: &File, buf: &mut [u8], offset: u64) -> io::Result<usize> {
        use std::os::unix::prelude::FileExt;
        file.read_at(buf, offset)
    }
}

impl Segment for FileSegment {
    fn write(&self, data: &[u8]) -> Result<(), ()> {
        let mut inner = self.inner.lock().unwrap();
        match &inner.file {
            Some(file) => {
                if inner.write_position >= self.max_file_length {
                    Err(())
                } else {
                    #[cfg(target_family = "windows")]
                    {
                        use std::os::windows::prelude::FileExt;
                        file.seek_write(data, inner.write_position).ok_log();
                    }
                    #[cfg(target_family = "unix")]
                    {
                        use std::os::unix::prelude::FileExt;
                        file.write_all_at(data, inner.write_position).ok();
                    }
                    inner.write_position += data.len() as u64;
                    inner.completed |= inner.write_position >= self.max_file_length;
                    self.condition.notify_all();
                    Ok(())
                }
            }
            None => Err(()),
        }
    }

    fn complete(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.completed = true;
        if inner.read_position >= inner.write_position {
            inner.file.take();
        }
        self.condition.notify_all();
    }

    fn read(&self, max_len: usize) -> Vec<u8> {
        let mut inner = self.inner.lock().unwrap();
        loop {
            if inner.read_position < inner.write_position {
                match &inner.file {
                    Some(file) => {
                        let mut buf = vec![0u8; max_len];
                        let res = FileSegment::read_at(file, &mut buf, inner.read_position)
                            .ok_log()
                            .unwrap_or(0);
                        inner.read_position += res as u64;
                        buf.resize(res, 0);
                        return buf;
                    }
                    None => return Vec::new(),
                }
            } else if inner.completed {
                inner.file.take();
                return Vec::new();
            } else {
                inner = self.condition.wait(inner).unwrap();
            }
        }
    }

    fn memory_used(&self) -> usize {
        0
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

    /// Inserts segment. Potentially unblock readers waiting for
    /// get_segment_at_index.
    fn insert_segment(&self, segment: Arc<BoxedSegment>) {
        let mut inner = self.inner.lock().unwrap();
        if !inner.completed {
            inner.segments.push(segment);
        }
        self.condition.notify_all();
    }

    /// Marks writing as completed. If there is reader waiting for
    /// get_segment_at_index unblocks it.
    fn complete(&self) {
        let mut inner = self.inner.lock().unwrap();
        inner.completed = true;
        self.condition.notify_all();
    }

    fn total_memory_usage(&self) -> usize {
        let inner = self.inner.lock().unwrap();
        let mut size = 0;
        for segment in &inner.segments {
            size += segment.memory_used();
        }
        size
    }
}

pub struct SegmentedQueueReader {
    state: Arc<QueueState>,
    current_segment: Cell<usize>,
}

impl SegmentedQueueReader {
    fn new(state: Arc<QueueState>) -> Self {
        Self {
            state,
            current_segment: Cell::new(0),
        }
    }

    /// Reads up to `max_len` bytes from queue. Blocks until data is available.
    /// Returns empty vector if queue is completed and no more data is available.
    pub fn read_some(&self, max_len: usize) -> Vec<u8> {
        loop {
            let segment = self.state.get_segment_at_index(self.current_segment.get());
            match segment {
                Some(segment) => {
                    let data = segment.read(max_len);
                    if data.is_empty() {
                        self.current_segment.replace(self.current_segment.get() + 1);
                    } else {
                        return data;
                    }
                }
                None => return Vec::new(),
            }
        }
    }

    /// Reads len amount of bytes from queue. Blocks until data is available.
    /// If returned amount of bytes is less than len, the queue is completed.
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

pub struct QueueConfiguration {
    /// Maximum size for single memory segment
    pub memory_segment_max_size: usize,

    /// Maximum length for file of single file segment
    pub file_segment_max_length: u64,

    /// Total maximum memory used. When queue memory usage gets over this
    /// threshold next created segment will be file segment. Otherwise (or when
    /// not set) the next created segment will be memory segment.
    /// If None all segments in queue will be memory segments.
    pub max_memory_usage: Option<usize>,
}

pub struct SegmentedQueueWriter {
    configuration: QueueConfiguration,
    state: Arc<QueueState>,
    current_segment: RefCell<Arc<BoxedSegment>>,
}

impl SegmentedQueueWriter {
    fn new(state: Arc<QueueState>, configuration: QueueConfiguration) -> Self {
        // Always start with a memory segment
        let segment: BoxedSegment =
            Box::new(MemorySegment::new(configuration.memory_segment_max_size));
        let segment = Arc::new(segment);
        state.insert_segment(segment.clone());
        Self {
            configuration,
            state,
            current_segment: RefCell::new(segment),
        }
    }

    fn next_segment(&self) {
        let segment: BoxedSegment = match self.configuration.max_memory_usage {
            Some(max_memory_usage) => {
                if self.state.total_memory_usage() >= max_memory_usage {
                    Box::new(FileSegment::new(self.configuration.file_segment_max_length))
                } else {
                    Box::new(MemorySegment::new(
                        self.configuration.memory_segment_max_size,
                    ))
                }
            }
            None => Box::new(MemorySegment::new(
                self.configuration.memory_segment_max_size,
            )),
        };
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
    configuration: QueueConfiguration,
) -> (SegmentedQueueWriter, SegmentedQueueReader) {
    let state = Arc::new(QueueState::new());
    let writer = SegmentedQueueWriter::new(state.clone(), configuration);
    let reader = SegmentedQueueReader::new(state);
    (writer, reader)
}

#[cfg(test)]
mod test {
    use std::{sync::Arc, thread, time::Duration};

    use crate::{
        segmented_queue::{FileSegment, MemorySegment},
        value_promise::Promise,
    };

    use super::BoxedSegment;

    fn read_from_segment(size: usize, segment: &Arc<BoxedSegment>) -> Arc<Promise<Vec<u8>>> {
        let promise = Arc::new(Promise::<Vec<u8>>::new());
        let segment_clone = segment.clone();
        let promise_clone = promise.clone();
        thread::spawn(move || {
            let r = segment_clone.read(size);
            promise_clone.set(r);
        });
        promise
    }

    fn test_segment(segment: Arc<BoxedSegment>) {
        let r = read_from_segment(5, &segment);
        thread::sleep(Duration::from_millis(50));
        assert!(segment.write(&[1, 2]).is_ok());
        assert_eq!(r.wait().as_slice(), &[1, 2]);

        let r = read_from_segment(3, &segment);
        thread::sleep(Duration::from_millis(50));
        assert!(segment.write(&[1, 2, 3, 4, 5]).is_ok());
        assert_eq!(r.wait().as_slice(), &[1, 2, 3]);

        let r = read_from_segment(3, &segment);
        assert_eq!(r.wait().as_slice(), &[4, 5]);

        // last one
        assert!(segment.write(&[1, 2, 3]).is_ok());

        let r = read_from_segment(3, &segment);
        assert_eq!(r.wait().as_slice(), &[1, 2, 3]);

        // Not blocking any more, segment is completed (reached size)
        let r = read_from_segment(3, &segment);
        assert_eq!(r.wait().as_slice(), &[]);

        // Can't write to completed segment
        assert!(segment.write(&[1, 2, 3]).is_err());
    }

    fn test_segment_complete(segment: Arc<BoxedSegment>) {
        let r = read_from_segment(5, &segment);
        thread::sleep(Duration::from_millis(50));
        assert!(segment.write(&[1, 2]).is_ok());
        assert_eq!(r.wait().as_slice(), &[1, 2]);

        let r = read_from_segment(5, &segment);
        thread::sleep(Duration::from_millis(50));
        assert!(r.try_take().is_none());
        segment.complete();
        assert_eq!(r.wait().as_slice(), &[]);
    }

    #[test]
    fn test1() {
        test_segment(Arc::new(Box::new(MemorySegment::new(8))));
        test_segment_complete(Arc::new(Box::new(MemorySegment::new(8))));

        test_segment(Arc::new(Box::new(FileSegment::new(8))));
        test_segment_complete(Arc::new(Box::new(FileSegment::new(8))));
    }
}
