use std::collections::BTreeMap;
use std::borrow::Cow;
use std::io::{Read, Write, Seek};

pub trait ReadSeek: Read + Seek {}
impl<T: Read + Seek + ?Sized> ReadSeek for T {}

pub trait ReadWriteSeek: Read + Write + Seek {}
impl<T: Read + Write + Seek + ?Sized> ReadWriteSeek for T {}

pub trait WriteSeek: Write + Seek {}
impl<T: Write + Seek + ?Sized> WriteSeek for T {}

pub enum IoType<'a> {
    // file path or URL (owned or borrowed)
    FileOrUrl(Cow<'a, str>),

    // in-memory bytes (owned or borrowed)
    Bytes(Cow<'a, [u8]>),

    // Streams
    ReadStream          { stream: Box<dyn Read + Send + 'a>,          size_hint: Option<u64> },
    WriteStream         { stream: Box<dyn Write + Send + 'a>,         size_hint: Option<u64> },
    ReadSeekStream      { stream: Box<dyn ReadSeek + Send + 'a>,      size_hint: Option<u64> },
    ReadWriteSeekStream { stream: Box<dyn ReadWriteSeek + Send + 'a>, size_hint: Option<u64> },
    WriteSeekStream     { stream: Box<dyn WriteSeek + Send + 'a>,     size_hint: Option<u64> },

    FileList(BTreeMap<String, IoType<'a>>),
}

impl<'a> IoType<'a> {
    pub fn from_read           <T: Read                + Send + 'a>(s: T, size_hint: Option<u64>) -> Self { IoType::ReadStream          { stream: Box::new(s), size_hint } }
    pub fn from_write          <T: Write               + Send + 'a>(s: T, size_hint: Option<u64>) -> Self { IoType::WriteStream         { stream: Box::new(s), size_hint } }
    pub fn from_read_seek      <T: Read + Seek         + Send + 'a>(s: T, size_hint: Option<u64>) -> Self { IoType::ReadSeekStream      { stream: Box::new(s), size_hint } }
    pub fn from_read_write_seek<T: Read + Write + Seek + Send + 'a>(s: T, size_hint: Option<u64>) -> Self { IoType::ReadWriteSeekStream { stream: Box::new(s), size_hint } }
    pub fn from_write_seek     <T: Write + Seek        + Send + 'a>(s: T, size_hint: Option<u64>) -> Self { IoType::WriteSeekStream     { stream: Box::new(s), size_hint } }
}

impl<'a> From<&'a str> for IoType<'a> {
    fn from(s: &'a str) -> Self { IoType::FileOrUrl(Cow::Borrowed(s)) }
}
impl From<String> for IoType<'_> {
    fn from(s: String) -> Self { IoType::FileOrUrl(Cow::Owned(s)) }
}
impl<'a> From<&'a [u8]> for IoType<'a> {
    fn from(b: &'a [u8]) -> Self { IoType::Bytes(Cow::Borrowed(b)) }
}
impl From<Vec<u8>> for IoType<'_> {
    fn from(b: Vec<u8>) -> Self { IoType::Bytes(Cow::Owned(b)) }
}
impl<'a> From<BTreeMap<String, IoType<'a>>> for IoType<'a> {
    fn from(m: BTreeMap<String, IoType<'a>>) -> Self { IoType::FileList(m) }
}