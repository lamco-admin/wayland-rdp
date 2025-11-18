//! Buffer Management
//!
//! Manages PipeWire buffers including DMA-BUF and memory-mapped buffers.

use std::collections::{HashMap, HashSet, VecDeque};
use std::os::fd::RawFd;
use std::sync::Arc;
use std::time::SystemTime;
use tokio::sync::Mutex;
use crate::pipewire::error::{PipeWireError, Result};
use crate::pipewire::ffi::SpaDataType;

/// Safe wrapper for raw pointer that implements Send+Sync
/// Safety: The buffer manager ensures proper synchronization
struct SendPtr(*mut u8);
unsafe impl Send for SendPtr {}
unsafe impl Sync for SendPtr {}

impl SendPtr {
    fn new(ptr: *mut u8) -> Self {
        Self(ptr)
    }

    fn as_ptr(&self) -> *mut u8 {
        self.0
    }
}

/// Buffer type
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BufferType {
    /// DMA-BUF (zero-copy)
    DmaBuf,
    /// Memory file descriptor
    MemFd,
    /// Memory pointer
    MemPtr,
}

impl BufferType {
    pub fn from_spa_type(spa_type: SpaDataType) -> Option<Self> {
        match spa_type {
            SpaDataType::DmaBuf => Some(Self::DmaBuf),
            SpaDataType::MemFd => Some(Self::MemFd),
            SpaDataType::MemPtr => Some(Self::MemPtr),
            _ => None,
        }
    }

    pub fn is_dmabuf(&self) -> bool {
        matches!(self, Self::DmaBuf)
    }
}

/// Managed buffer
pub struct ManagedBuffer {
    /// Buffer ID
    pub id: u32,

    /// Buffer type
    pub buffer_type: BufferType,

    /// File descriptor (for DMA-BUF and MemFd)
    pub fd: Option<RawFd>,

    /// Buffer size
    pub size: usize,

    /// Memory mapping (for MemFd and MemPtr)
    /// Safety: This pointer is only valid while the buffer exists
    pub mapped: Option<SendPtr>,

    /// Mapped size
    pub mapped_size: usize,

    /// In use flag
    pub in_use: bool,

    /// Use count
    pub use_count: u64,

    /// Last used time
    pub last_used: SystemTime,

    /// DMA-BUF modifier (for DMA-BUF only)
    pub modifier: u64,
}

impl ManagedBuffer {
    /// Create new managed buffer
    pub fn new(id: u32, buffer_type: BufferType, size: usize) -> Self {
        Self {
            id,
            buffer_type,
            fd: None,
            size,
            mapped: None,
            mapped_size: 0,
            in_use: false,
            use_count: 0,
            last_used: SystemTime::now(),
            modifier: 0,
        }
    }

    /// Mark buffer as in use
    pub fn acquire(&mut self) {
        self.in_use = true;
        self.use_count += 1;
        self.last_used = SystemTime::now();
    }

    /// Mark buffer as free
    pub fn release(&mut self) {
        self.in_use = false;
        self.last_used = SystemTime::now();
    }

    /// Get mapped data as slice
    pub unsafe fn as_slice(&self) -> Option<&[u8]> {
        self.mapped.as_ref().map(|send_ptr| {
            std::slice::from_raw_parts(send_ptr.as_ptr(), self.mapped_size)
        })
    }

    /// Get mapped data as mutable slice
    pub unsafe fn as_mut_slice(&mut self) -> Option<&mut [u8]> {
        self.mapped.as_mut().map(|send_ptr| {
            std::slice::from_raw_parts_mut(send_ptr.as_ptr(), self.mapped_size)
        })
    }
}

impl Drop for ManagedBuffer {
    fn drop(&mut self) {
        // Unmap memory if mapped
        if let Some(send_ptr) = self.mapped.take() {
            if self.mapped_size > 0 {
                unsafe {
                    libc::munmap(send_ptr.as_ptr() as *mut libc::c_void, self.mapped_size);
                }
            }
        }

        // Note: We don't close the FD here as it's managed by PipeWire
    }
}

/// Buffer manager
pub struct BufferManager {
    /// Buffers indexed by ID
    buffers: HashMap<u32, ManagedBuffer>,

    /// Free buffer queue
    free_buffers: VecDeque<u32>,

    /// In-use buffer set
    in_use_buffers: HashSet<u32>,

    /// Maximum buffers
    max_buffers: usize,

    /// Next buffer ID
    next_id: u32,

    /// Statistics
    stats: BufferStats,
}

impl BufferManager {
    /// Create new buffer manager
    pub fn new(max_buffers: usize) -> Self {
        Self {
            buffers: HashMap::new(),
            free_buffers: VecDeque::new(),
            in_use_buffers: HashSet::new(),
            max_buffers,
            next_id: 0,
            stats: BufferStats::default(),
        }
    }

    /// Register a new buffer
    pub fn register_buffer(
        &mut self,
        buffer_type: BufferType,
        size: usize,
        fd: Option<RawFd>,
        modifier: u64,
    ) -> Result<u32> {
        if self.buffers.len() >= self.max_buffers {
            return Err(PipeWireError::BufferAllocationFailed(
                "Maximum buffer count reached".to_string()
            ));
        }

        let id = self.next_id;
        self.next_id += 1;

        let mut buffer = ManagedBuffer::new(id, buffer_type, size);
        buffer.fd = fd;
        buffer.modifier = modifier;

        // Try to map memory if needed
        if let Some(fd) = fd {
            if buffer_type != BufferType::DmaBuf {
                buffer.mapped = Some(unsafe {
                    let ptr = libc::mmap(
                        std::ptr::null_mut(),
                        size,
                        libc::PROT_READ | libc::PROT_WRITE,
                        libc::MAP_SHARED,
                        fd,
                        0,
                    );

                    if ptr == libc::MAP_FAILED {
                        return Err(PipeWireError::BufferAllocationFailed(
                            format!("Failed to mmap buffer: {}", std::io::Error::last_os_error())
                        ));
                    }

                    SendPtr::new(ptr as *mut u8)
                });
                buffer.mapped_size = size;
            }
        }

        self.buffers.insert(id, buffer);
        self.free_buffers.push_back(id);
        self.stats.total_allocated += 1;

        Ok(id)
    }

    /// Acquire a buffer for use
    pub fn acquire_buffer(&mut self) -> Option<u32> {
        if let Some(id) = self.free_buffers.pop_front() {
            if let Some(buffer) = self.buffers.get_mut(&id) {
                buffer.acquire();
                self.in_use_buffers.insert(id);
                self.stats.acquisitions += 1;
                return Some(id);
            }
        }
        self.stats.acquisition_failures += 1;
        None
    }

    /// Release a buffer
    pub fn release_buffer(&mut self, id: u32) -> Result<()> {
        if let Some(buffer) = self.buffers.get_mut(&id) {
            buffer.release();
            self.in_use_buffers.remove(&id);
            self.free_buffers.push_back(id);
            self.stats.releases += 1;
            Ok(())
        } else {
            Err(PipeWireError::InvalidParameter(
                format!("Buffer {} not found", id)
            ))
        }
    }

    /// Get buffer by ID
    pub fn get_buffer(&self, id: u32) -> Option<&ManagedBuffer> {
        self.buffers.get(&id)
    }

    /// Get mutable buffer by ID
    pub fn get_buffer_mut(&mut self, id: u32) -> Option<&mut ManagedBuffer> {
        self.buffers.get_mut(&id)
    }

    /// Unregister a buffer
    pub fn unregister_buffer(&mut self, id: u32) -> Result<()> {
        if let Some(_buffer) = self.buffers.remove(&id) {
            self.free_buffers.retain(|&bid| bid != id);
            self.in_use_buffers.remove(&id);
            self.stats.total_freed += 1;
            Ok(())
        } else {
            Err(PipeWireError::InvalidParameter(
                format!("Buffer {} not found", id)
            ))
        }
    }

    /// Get number of free buffers
    pub fn free_count(&self) -> usize {
        self.free_buffers.len()
    }

    /// Get number of in-use buffers
    pub fn in_use_count(&self) -> usize {
        self.in_use_buffers.len()
    }

    /// Get total buffer count
    pub fn total_count(&self) -> usize {
        self.buffers.len()
    }

    /// Get statistics
    pub fn stats(&self) -> &BufferStats {
        &self.stats
    }

    /// Clear all buffers
    pub fn clear(&mut self) {
        self.buffers.clear();
        self.free_buffers.clear();
        self.in_use_buffers.clear();
        self.next_id = 0;
    }
}

/// Buffer statistics
#[derive(Debug, Clone, Default)]
pub struct BufferStats {
    /// Total buffers allocated
    pub total_allocated: u64,

    /// Total buffers freed
    pub total_freed: u64,

    /// Total acquisitions
    pub acquisitions: u64,

    /// Total releases
    pub releases: u64,

    /// Acquisition failures
    pub acquisition_failures: u64,
}

impl BufferStats {
    /// Get allocation rate
    pub fn allocation_rate(&self) -> f64 {
        if self.total_freed == 0 {
            self.total_allocated as f64
        } else {
            self.total_allocated as f64 / self.total_freed as f64
        }
    }

    /// Get failure rate
    pub fn failure_rate(&self) -> f64 {
        if self.acquisitions == 0 {
            0.0
        } else {
            self.acquisition_failures as f64 / (self.acquisitions + self.acquisition_failures) as f64
        }
    }
}

/// Thread-safe buffer manager wrapper
pub struct SharedBufferManager {
    inner: Arc<Mutex<BufferManager>>,
}

impl SharedBufferManager {
    /// Create new shared buffer manager
    pub fn new(max_buffers: usize) -> Self {
        Self {
            inner: Arc::new(Mutex::new(BufferManager::new(max_buffers))),
        }
    }

    /// Register a buffer
    pub async fn register_buffer(
        &self,
        buffer_type: BufferType,
        size: usize,
        fd: Option<RawFd>,
        modifier: u64,
    ) -> Result<u32> {
        self.inner.lock().await.register_buffer(buffer_type, size, fd, modifier)
    }

    /// Acquire a buffer
    pub async fn acquire_buffer(&self) -> Option<u32> {
        self.inner.lock().await.acquire_buffer()
    }

    /// Release a buffer
    pub async fn release_buffer(&self, id: u32) -> Result<()> {
        self.inner.lock().await.release_buffer(id)
    }

    /// Get buffer (requires holding lock)
    pub async fn with_buffer<F, R>(&self, id: u32, f: F) -> Option<R>
    where
        F: FnOnce(&ManagedBuffer) -> R,
    {
        let mgr = self.inner.lock().await;
        mgr.get_buffer(id).map(f)
    }

    /// Get mutable buffer (requires holding lock)
    pub async fn with_buffer_mut<F, R>(&self, id: u32, f: F) -> Option<R>
    where
        F: FnOnce(&mut ManagedBuffer) -> R,
    {
        let mut mgr = self.inner.lock().await;
        mgr.get_buffer_mut(id).map(f)
    }

    /// Unregister a buffer
    pub async fn unregister_buffer(&self, id: u32) -> Result<()> {
        self.inner.lock().await.unregister_buffer(id)
    }

    /// Get statistics
    pub async fn stats(&self) -> BufferStats {
        self.inner.lock().await.stats().clone()
    }

    /// Get free count
    pub async fn free_count(&self) -> usize {
        self.inner.lock().await.free_count()
    }

    /// Get in-use count
    pub async fn in_use_count(&self) -> usize {
        self.inner.lock().await.in_use_count()
    }

    /// Clone the Arc
    pub fn clone(&self) -> Self {
        Self {
            inner: Arc::clone(&self.inner),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_buffer_creation() {
        let buffer = ManagedBuffer::new(0, BufferType::MemPtr, 1024);
        assert_eq!(buffer.id, 0);
        assert_eq!(buffer.buffer_type, BufferType::MemPtr);
        assert_eq!(buffer.size, 1024);
        assert!(!buffer.in_use);
    }

    #[test]
    fn test_buffer_acquire_release() {
        let mut buffer = ManagedBuffer::new(0, BufferType::MemPtr, 1024);

        buffer.acquire();
        assert!(buffer.in_use);
        assert_eq!(buffer.use_count, 1);

        buffer.release();
        assert!(!buffer.in_use);
        assert_eq!(buffer.use_count, 1);
    }

    #[test]
    fn test_buffer_manager() {
        let mut mgr = BufferManager::new(5);

        // Register buffers
        let id1 = mgr.register_buffer(BufferType::MemPtr, 1024, None, 0).unwrap();
        let id2 = mgr.register_buffer(BufferType::MemPtr, 2048, None, 0).unwrap();

        assert_eq!(mgr.total_count(), 2);
        assert_eq!(mgr.free_count(), 2);

        // Acquire buffer
        let acquired = mgr.acquire_buffer().unwrap();
        assert_eq!(mgr.free_count(), 1);
        assert_eq!(mgr.in_use_count(), 1);

        // Release buffer
        mgr.release_buffer(acquired).unwrap();
        assert_eq!(mgr.free_count(), 2);
        assert_eq!(mgr.in_use_count(), 0);
    }

    #[test]
    fn test_buffer_limit() {
        let mut mgr = BufferManager::new(2);

        mgr.register_buffer(BufferType::MemPtr, 1024, None, 0).unwrap();
        mgr.register_buffer(BufferType::MemPtr, 1024, None, 0).unwrap();

        // Should fail due to limit
        let result = mgr.register_buffer(BufferType::MemPtr, 1024, None, 0);
        assert!(result.is_err());
    }

    #[tokio::test]
    async fn test_shared_buffer_manager() {
        let mgr = SharedBufferManager::new(5);

        let id = mgr.register_buffer(BufferType::MemPtr, 1024, None, 0).await.unwrap();
        assert_eq!(mgr.free_count().await, 1);

        let acquired = mgr.acquire_buffer().await.unwrap();
        assert_eq!(acquired, id);
        assert_eq!(mgr.in_use_count().await, 1);

        mgr.release_buffer(acquired).await.unwrap();
        assert_eq!(mgr.free_count().await, 1);
    }
}
