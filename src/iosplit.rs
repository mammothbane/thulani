use std::{
    io::{
        Read,
        Write,
        Error,
    },
    sync::{
        Mutex,
        Arc,
    },
};

use log::trace;

pub fn io_split<T>(t: T) -> (impl Read, impl Write)
    where T: Read + Write {
    let lock = Arc::new(Mutex::new(t));

    (Wrap { io: lock.clone() }, Wrap { io: lock })
}

struct Wrap<T> {
    io: Arc<Mutex<T>>,
}

impl <T> Read for Wrap<T> where T: Read {
    #[inline]
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        trace!("read {} bytes: {}", buf.len(), String::from_utf8(buf.to_owned()).unwrap());
        let mut reader = self.io.lock().unwrap();
        reader.read(buf)
    }

    #[inline]
    fn read_vectored(&mut self, bufs: &mut [std::io::IoSliceMut<'_>]) -> std::io::Result<usize> {
        trace!("read {} bytes", bufs.iter().map(|buf| buf.len()).sum::<usize>());
        let mut reader = self.io.lock().unwrap();
        reader.read_vectored(bufs)
    }

    #[inline]
    fn read_exact(&mut self, buf: &mut [u8]) -> std::io::Result<()> {
        trace!("read {} bytes", buf.len());
        let mut reader = self.io.lock().unwrap();
        reader.read_exact(buf)
    }
}

impl <T> Write for Wrap<T> where T: Write {
    #[inline]
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        trace!("write {} bytes", buf.len());
        let mut writer = self.io.lock().unwrap();
        writer.write(buf)
    }

    #[inline]
    fn write_vectored(&mut self, bufs: &[std::io::IoSlice<'_>]) -> std::io::Result<usize> {
        let mut writer = self.io.lock().unwrap();
        writer.write_vectored(bufs)
    }

    #[inline]
    fn flush(&mut self) -> Result<(), Error> {
        let mut writer = self.io.lock().unwrap();
        writer.flush()
    }
}

