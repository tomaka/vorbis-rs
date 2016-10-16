extern crate vorbis;

use std::io::Write;

fn main() {
    let args = std::env::args();
    args.next();
    // It needs 3 file address as arguments:
    //      first for input vorbis,
    //      second for pcm output file,
    //      third for
    let error = "Error: Usage is <in-ogg-vorbis-file> <out-pcm-file> <out-vorbis-file>";
    let in_file = args.next().expect(error);
    let in_file = std::fs::File::open(in_file).unwrap();
    let pcm_file = args.next().expect(error);
    let mut pcm_file = std::fs::File::create(pcm_file).unwrap();
    let out_file = args.next().expect(error);
    let mut out_file = std::fs::File::create(out_file).unwrap();
    let mut decoder = vorbis::Decoder::new(in_file).unwrap();
    let packets = decoder.packets();
    let mut data = Vec::new();
    let mut channels = 0;
    let mut rate = 0;
    let mut bitrate_upper = 0;
    let mut bitrate_nominal = 0;
    let mut bitrate_lower = 0;
    let mut bitrate_window = 0;
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
                pcm_file.write(&file_data[..]).unwrap();
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
    let mut encoder = vorbis::Encoder::new(channels as u8, rate, vorbis::VorbisQuality::Midium).expect("Error in creating encoder");
    out_file.write(encoder.encode(&data).expect("Error in encoding.").as_slice()).expect("Error in writing");
    out_file.write(encoder.flush().expect("Error in flushing.").as_slice()).expect("Error in writing");
}
