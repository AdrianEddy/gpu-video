use std::collections::BTreeMap;
use std::borrow::Cow;
use std::io::{Read, Write, Seek};

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

pub trait WriteSeek: Write + Seek {}
impl<T: Write + Seek + ?Sized> WriteSeek for T {}

pub enum IoType<'a> {
    // file path or URL (owned or borrowed)
    FileOrUrl(Cow<'a, str>),

    // Owned in-memory bytes — moving a `Vec<u8>` in is free. FFmpeg/r3d keep the
    // buffer alive past any borrow (potentially on another thread), so an input
    // must own its data or guarantee `'static`. Pick the zero-copy path for what
    // you hold:
    //   • own a `Vec`            → `IoType::Bytes(vec)` (this variant)
    //   • shared / mmap / static → `from_read_seek(Cursor::new(owner), hint)`
    //     (e.g. `Arc<[u8]>`, `bytes::Bytes`, `memmap2::Mmap` — all are
    //      `Read + Seek + Send + 'static` and never copy)
    //   • a borrow you can't own  → `unsafe IoType::borrowed_bytes(&buf)`
    Bytes(Vec<u8>),

    // Custom callback
    // Don't use with FileList
    Callback { filename: String, callback: Box<dyn Fn(&str) -> Result<IoType<'static>, std::io::Error>> },

    // Streams
    // FFmpeg's `StreamIo` (and r3d's global custom-IO registry) own the stream
    // and may drive its callbacks from any thread, outliving any borrow, so the
    // stream must be `Send + 'static`.
    ReadStream          { stream: Box<dyn Read + Send + 'static>,          size_hint: Option<u64> },
    WriteStream         { stream: Box<dyn Write + Send + 'static>,         size_hint: Option<u64> },
    ReadSeekStream      { stream: Box<dyn ReadSeek + Send + 'static>,      size_hint: Option<u64> },
    WriteSeekStream     { stream: Box<dyn WriteSeek + Send + 'static>,     size_hint: Option<u64> },

    FileList(BTreeMap<String, IoType<'a>>),
}

impl<'a> IoType<'a> {
    pub fn from_read           <T: Read         + Send + 'static>(s: T, size_hint: Option<u64>) -> Self { IoType::ReadStream          { stream: Box::new(s), size_hint } }
    pub fn from_write          <T: Write        + Send + 'static>(s: T, size_hint: Option<u64>) -> Self { IoType::WriteStream         { stream: Box::new(s), size_hint } }
    pub fn from_read_seek      <T: Read + Seek  + Send + 'static>(s: T, size_hint: Option<u64>) -> Self { IoType::ReadSeekStream      { stream: Box::new(s), size_hint } }
    pub fn from_write_seek     <T: Write + Seek + Send + 'static>(s: T, size_hint: Option<u64>) -> Self { IoType::WriteSeekStream     { stream: Box::new(s), size_hint } }

    /// Wrap a borrowed buffer as a seekable input **without copying it**.
    ///
    /// This is the zero-copy escape hatch for the common "I have a `&[u8]` I
    /// can't take ownership of" case. The slice's lifetime is erased to
    /// `'static`, so the borrow checker can no longer protect you — hence
    /// `unsafe`.
    ///
    /// # Safety
    /// The referenced buffer must stay allocated and unmodified until the
    /// `Decoder` built from this input is dropped. FFmpeg reads through the
    /// pointer lazily and may do so from another thread for the decoder's whole
    /// lifetime; only the caller knows the buffer lives that long.
    pub unsafe fn borrowed_bytes(b: &[u8]) -> IoType<'static> {
        let size = b.len() as u64;
        // SAFETY: the caller's contract (above) guarantees `b` outlives every
        // read. `Cursor<&[u8]>` only reads through the pointer — it never frees
        // or mutates the buffer — so erasing the lifetime is sound given that.
        let s: &'static [u8] = unsafe { std::mem::transmute::<&[u8], &'static [u8]>(b) };
        IoType::ReadSeekStream { stream: Box::new(std::io::Cursor::new(s)), size_hint: Some(size) }
    }
}

impl<'a> From<&'a str> for IoType<'a> {
    fn from(s: &'a str) -> Self { IoType::FileOrUrl(Cow::Borrowed(s)) }
}
impl From<String> for IoType<'_> {
    fn from(s: String) -> Self { IoType::FileOrUrl(Cow::Owned(s)) }
}
impl From<Vec<u8>> for IoType<'_> {
    fn from(b: Vec<u8>) -> Self { IoType::Bytes(b) }
}
impl<'a> From<BTreeMap<String, IoType<'a>>> for IoType<'a> {
    fn from(m: BTreeMap<String, IoType<'a>>) -> Self { IoType::FileList(m) }
}