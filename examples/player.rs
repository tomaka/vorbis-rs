extern crate vorbis;
extern crate openal;
extern crate time;

use std::collections::RingBuf;
use std::io::timer::sleep;
use std::io::BufReader;
use std::time::duration::Duration;
use openal::al;
use openal::alc;

fn main() {
    let device = alc::Device::open(None).expect("Could not open device");
    let ctx = device.create_context([]).expect("Could not create context");
    ctx.make_current();

    let source = al::Source::gen();

    let mut decoder = vorbis::Decoder::new(BufReader::new(include_bin!("mozart_symfony_40.ogg")))
        .unwrap();

    let mut buffers = RingBuf::new();
    let mut packet_num = 0u;
    let mut previous_packet_duration = Duration::milliseconds(0);

    for packet in decoder.packets() {
        let packet = packet.unwrap();

        packet_num += 1;
        if packet_num >= 2 {
            buffers.pop_front();
        }

        buffers.push(al::Buffer::gen());

        let packet_duration = packet.data.len() as f32 / (packet.channels as u64 * packet.rate) as f32;
        let packet_duration = Duration::milliseconds((packet_duration * 1000.0) as i64);

        let format = match packet.channels {
            1 => al::FormatMono16,
            2 => al::FormatStereo16,
            _ => unimplemented!()
        };

        unsafe { buffers.back_mut().unwrap()
            .buffer_data(format, packet.data.as_slice(), packet.rate as al::ALsizei) };
        source.queue_buffer(buffers.back().unwrap());

        if !source.is_playing() {
            source.play();
        }

        if packet_num >= 2 {
            sleep(previous_packet_duration);
        }

        previous_packet_duration = packet_duration;
    }

    ctx.destroy();
    device.close().ok().expect("Unable to close device");
}
