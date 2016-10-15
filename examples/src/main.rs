extern crate vorbis;

use std::io::Write;

struct MyFile {
	file: std::fs::File,
}

impl MyFile {
	fn new() -> Self {
		MyFile {
			file: std::fs::File::open("/home/thany/Dropbox/Projects/Start/Music/back-1.ogg").unwrap(),
		}
	}
}

impl std::io::Read for MyFile {
	fn read(&mut self, buf: &mut [u8]) -> std::io::Result<usize> {
		self.file.read(buf)
	}
}

impl std::io::Seek for MyFile {
	fn seek(&mut self, pos: std::io::SeekFrom) -> std::io::Result<u64> {
		self.file.seek(pos)
	}
}

fn main() {
    let f = MyFile::new();
    let mut decoder = vorbis::Decoder::new(f).unwrap();
    let packets = decoder.packets();
    let mut data = Vec::new();
	let mut channels = 0;
	let mut rate = 0;
	let mut bitrate_upper = 0;
	let mut bitrate_nominal = 0;
	let mut bitrate_lower = 0;
	let mut bitrate_window = 0;
	let mut file = std::fs::File::create("/home/thany/pcm").unwrap();
    for p in packets {
        match p {
            Ok(packet) => {
				channels = packet.channels;
				rate = packet.rate;
				bitrate_upper = packet.bitrate_upper;
				bitrate_nominal = packet.bitrate_nominal;
				bitrate_lower = packet.bitrate_lower;
				bitrate_window = packet.bitrate_window;
				let mut file_data = vec![0u8; packet.data.len() << 1];
				let mut index = 0;
				for sample in packet.data {
					file_data[index] = (sample as u32 & 255) as u8;
					index+=1;
					file_data[index] = ((sample as u32 >> 8) & 255) as u8;
					index+=1;
					data.push(sample);
				}
				file.write(&file_data[..]).unwrap();
            },
            _ => {}
        }

    }
    println!("PCM data size: {}", data.len());
	println!("channels: {:?}", channels);
	println!("rate: {:?}", rate);
	println!("bitrate_upper: {:?}", bitrate_upper);
	println!("bitrate_nominal: {:?}", bitrate_nominal);
	println!("bitrate_lower: {:?}", bitrate_lower);
	println!("bitrate_window: {:?}", bitrate_window);
	let mut file = std::fs::File::create("/home/thany/1.ogg").expect("Can not open the file.");
	let mut encoder = vorbis::Encoder::new(channels as u8, rate, vorbis::VorbisQuality::Midium).expect("Error in creating encoder");
	file.write(encoder.encode(&data).expect("Error in encoding.").as_slice()).expect("Error in writing");
	file.write(encoder.flush().expect("Error in flushing.").as_slice()).expect("Error in writing");
}
