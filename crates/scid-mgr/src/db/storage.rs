use memmap2::Mmap;
use std::ops::Deref;
use std::sync::Arc;

/// Zero-copy index storage abstraction supporting both memory-mapped slices and heap-allocated vectors
#[derive(Clone)]
pub enum IndexStorage<T: 'static> {
    Owned(Arc<Vec<T>>),
    Mmap {
        _mmap: Arc<Mmap>,
        ptr: *const T,
        len: usize,
    },
}

unsafe impl<T: Send + Sync> Send for IndexStorage<T> {}
unsafe impl<T: Send + Sync> Sync for IndexStorage<T> {}

impl<T> IndexStorage<T> {
    pub fn from_vec(vec: Vec<T>) -> Self {
        Self::Owned(Arc::new(vec))
    }

    /// Creates a zero-copy memory-mapped slice of records
    ///
    /// # Safety
    /// The caller must ensure that `offset + count * size_of::<T>() <= mmap.len()`
    /// and that the alignment of `T` is respected.
    pub unsafe fn from_mmap(mmap: Arc<Mmap>, offset: usize, count: usize) -> Self {
        let ptr = mmap.as_ptr().add(offset) as *const T;
        Self::Mmap {
            _mmap: mmap,
            ptr,
            len: count,
        }
    }

    #[inline(always)]
    pub fn as_slice(&self) -> &[T] {
        match self {
            Self::Owned(vec) => vec.as_slice(),
            Self::Mmap { ptr, len, .. } => unsafe { std::slice::from_raw_parts(*ptr, *len) },
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        match self {
            Self::Owned(vec) => vec.len(),
            Self::Mmap { len, .. } => *len,
        }
    }

    #[inline(always)]
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    #[inline(always)]
    pub fn is_mmap(&self) -> bool {
        matches!(self, Self::Mmap { .. })
    }
}

impl<T> Deref for IndexStorage<T> {
    type Target = [T];

    #[inline(always)]
    fn deref(&self) -> &Self::Target {
        self.as_slice()
    }
}

impl<T> AsRef<[T]> for IndexStorage<T> {
    #[inline(always)]
    fn as_ref(&self) -> &[T] {
        self.as_slice()
    }
}

impl<T: std::fmt::Debug> std::fmt::Debug for IndexStorage<T> {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "IndexStorage(mode={}, len={})",
            if self.is_mmap() { "mmap" } else { "owned" },
            self.len()
        )
    }
}

impl<T: Default> Default for IndexStorage<T> {
    fn default() -> Self {
        Self::Owned(Arc::new(Vec::new()))
    }
}
