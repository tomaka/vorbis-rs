#![feature(unsafe_destructor)]

extern crate "ogg-sys" as ogg_sys;
extern crate "vorbis-sys" as vorbis_sys;
extern crate "vorbisfile-sys" as vorbisfile_sys;
extern crate libc;

/// Allows you to decode a sound file stream into packets.
pub struct Decoder<R> where R: Reader + Seek {
    // further informations are boxed so that a pointer can be passed to callbacks
    data: Box<DecoderData<R>>,
    // the OggVorbis_File struct has no internal thread-safety
    nosync: std::kinds::marker::NoSync,
}

/// 
pub struct PacketsIter<'a, R: 'a + Reader + Seek>(&'a mut DecoderData<R>);

/// Errors that can happen while decoding
#[deriving(Show, PartialEq, Eq)]
pub enum VorbisError {
    ReadError(std::io::IoError),
    NotVorbis,
    VersionMismatch,
    BadHeader,
    InitialFileHeadersCorrupt,
}

impl std::error::Error for VorbisError {
    fn description(&self) -> &str {
        match self {
            &VorbisError::ReadError(_) => "A read from media returned an error",
            &VorbisError::NotVorbis => "Bitstream does not contain any Vorbis data",
            &VorbisError::VersionMismatch => "Vorbis version mismatch",
            &VorbisError::BadHeader => "Invalid Vorbis bitstream header",
            &VorbisError::InitialFileHeadersCorrupt => "Initial file headers are corrupt",
        }
    }

    fn cause(&self) -> Option<&std::error::Error> {
        match self {
            &VorbisError::ReadError(ref err) => Some(err as &std::error::Error),
            _ => None
        }
    }
}

impl std::error::FromError<std::io::IoError> for VorbisError {
    fn from_error(err: std::io::IoError) -> VorbisError {
        VorbisError::ReadError(err)
    }
}

struct DecoderData<R> where R: Reader + Seek {
    vorbis: vorbisfile_sys::OggVorbis_File,
    reader: R,
    current_logical_bitstream: libc::c_int,
    read_error: Option<std::io::IoError>,
}

/// Packet of data.
///
/// Each sample is an `i16` ranging from I16_MIN to I16_MAX.
///
/// The channels are interleaved in the data. For example if you have two channels, you will
/// get a sample from channel 1, then a sample from channel 2, than a sample from channel 1, etc.
#[deriving(Clone, Show)]
pub struct Packet {
    pub data: Vec<i16>,
    pub channels: u16,
    pub rate: u64,
    pub bitrate_upper: u64,
    pub bitrate_nominal: u64,
    pub bitrate_lower: u64,
    pub bitrate_window: u64,
}

impl<R> Decoder<R> where R: Reader + Seek {
    pub fn new(input: R) -> Result<Decoder<R>, VorbisError> {
        extern fn read_func<R>(ptr: *mut libc::c_void, size: libc::size_t, nmemb: libc::size_t,
            datasource: *mut libc::c_void) -> libc::size_t where R: Reader + Seek
        {
            use std::c_vec::CVec;

            let mut ptr = ptr as *mut u8;

            let data: &mut DecoderData<R> = unsafe { std::mem::transmute(datasource) };

            loop {
                let mut buffer: CVec<u8> = unsafe { CVec::new(ptr, size as uint * nmemb as uint) };
                let buffer = buffer.as_mut_slice();

                match data.reader.read(buffer) {
                    Ok(0) => continue,
                    Ok(nb) => {
                        if buffer.len() == nb {
                            return nmemb;
                        } else {
                            unsafe { ptr = ptr.offset(nb as int) };
                        }
                    },
                    Err(ref e) if e.kind == std::io::EndOfFile => {
                        return 0
                    },
                    Err(e) => {
                        data.read_error = Some(e);
                        return 0;
                    }
                };
            }
        }

        extern fn seek_func<R>(datasource: *mut libc::c_void, offset: ogg_sys::ogg_int64_t,
            whence: libc::c_int) -> libc::c_int where R: Reader + Seek
        {
            let data: &mut DecoderData<R> = unsafe { std::mem::transmute(datasource) };

            let result = match whence {
                libc::SEEK_SET => data.reader.seek(offset, std::io::SeekSet),
                libc::SEEK_CUR => data.reader.seek(offset, std::io::SeekCur),
                libc::SEEK_END => data.reader.seek(offset, std::io::SeekEnd),
                _ => unreachable!()
            };

            match result {
                Ok(_) => 0,
                Err(_) => -1
            }
        }

        extern fn tell_func<R>(datasource: *mut libc::c_void) -> libc::c_long
            where R: Reader + Seek
        {
            let data: &mut DecoderData<R> = unsafe { std::mem::transmute(datasource) };
            data.reader.tell().unwrap_or(-1) as libc::c_long
        }

        let callbacks = {
            let mut callbacks: vorbisfile_sys::ov_callbacks = unsafe { std::mem::zeroed() };
            callbacks.read_func = read_func::<R>;
            callbacks.seek_func = seek_func::<R>;
            callbacks.tell_func = tell_func::<R>;
            callbacks
        };

        let mut data = box DecoderData {
            vorbis: unsafe { std::mem::uninitialized() },
            reader: input,
            current_logical_bitstream: 0,
            read_error: None,
        };

        // initializing
        unsafe {
            let data_ptr = &mut *data as *mut DecoderData<R>;
            let data_ptr = data_ptr as *mut libc::c_void;
            try!(check_errors(vorbisfile_sys::ov_open_callbacks(data_ptr, &mut data.vorbis,
                std::ptr::null(), 0, callbacks)));
        }

        Ok(Decoder {
            data: data,
            nosync: std::kinds::marker::NoSync,
        })
    }

    pub fn packets(&mut self) -> PacketsIter<R> {
        PacketsIter(&mut *self.data)
    }
}

impl<'a, R> Iterator<Result<Packet, VorbisError>> for PacketsIter<'a, R> where R: 'a + Reader + Seek {
    fn next(&mut self) -> Option<Result<Packet, VorbisError>> {
        let mut buffer = Vec::from_elem(2048, 0i16);
        let buffer_len = buffer.len() * 2;

        match unsafe {
            vorbisfile_sys::ov_read(&mut self.0.vorbis, buffer.as_mut_ptr() as *mut i8,
                buffer_len as libc::c_int, 0, 2, 1, &mut self.0.current_logical_bitstream)
        } {
            0 => {
                match self.0.read_error.take() {
                    Some(err) => Some(Err(VorbisError::ReadError(err))),
                    None => None,
                }
            },

            err if err < 0 => {
                match check_errors(err as libc::c_int) {
                    Err(e) => Some(Err(e)),
                    Ok(_) => unreachable!()
                }
            },

            len => {
                buffer.truncate(len as uint);

                let infos = unsafe { vorbisfile_sys::ov_info(&mut self.0.vorbis,
                    self.0.current_logical_bitstream) };

                let infos: &vorbis_sys::vorbis_info = unsafe { std::mem::transmute(infos) };

                Some(Ok(Packet {
                    data: buffer,
                    channels: infos.channels as u16,
                    rate: infos.rate as u64,
                    bitrate_upper: infos.bitrate_upper as u64,
                    bitrate_nominal: infos.bitrate_nominal as u64,
                    bitrate_lower: infos.bitrate_lower as u64,
                    bitrate_window: infos.bitrate_window as u64,
                }))
            }
        }
    }
}

#[unsafe_destructor]
impl<R> Drop for Decoder<R> where R: Reader + Seek {
    fn drop(&mut self) {
        unsafe {
            vorbisfile_sys::ov_clear(&mut self.data.vorbis);
        }
    }
}

fn check_errors(code: libc::c_int) -> Result<(), VorbisError> {
    match code {
        0 => Ok(()),

        vorbis_sys::OV_ENOTVORBIS => Err(VorbisError::NotVorbis),
        vorbis_sys::OV_EVERSION => Err(VorbisError::VersionMismatch),
        vorbis_sys::OV_EBADHEADER => Err(VorbisError::BadHeader),
        vorbis_sys::OV_EINVAL => Err(VorbisError::InitialFileHeadersCorrupt),

        vorbis_sys::OV_EREAD => unimplemented!(),

        // indicates a bug or heap/stack corruption
        vorbis_sys::OV_EFAULT => panic!("Internal libvorbis error"),
        _ => panic!("Unknown vorbis error")
    }
}
