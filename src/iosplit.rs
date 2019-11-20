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

pub fn io_split<T>(t: T) -> (impl Read, impl Write)
    where T: Read + Write {
    let lock = Arc::new(Mutex::new(t));

    (Wrap { io: lock.clone() }, Wrap { io: lock })
}

struct Wrap<T> {
    io: Arc<Mutex<T>>,
}

impl <T> Read for Wrap<T> where T: Read {
    fn read(&mut self, buf: &mut [u8]) -> Result<usize, Error> {
        let mut reader = self.io.lock().unwrap();
        reader.read(buf)
    }
}

impl <T> Write for Wrap<T> where T: Write {
    fn write(&mut self, buf: &[u8]) -> Result<usize, Error> {
        let mut writer = self.io.lock().unwrap();
        writer.write(buf)
    }

    fn flush(&mut self) -> Result<(), Error> {
        let mut writer = self.io.lock().unwrap();
        writer.flush()
    }
}

