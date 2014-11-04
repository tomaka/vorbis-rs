#![feature(tuple_indexing)]

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

    // creating the decoder
    let mut decoder = vorbis::Decoder::new(BufReader::new(include_bin!("mozart_symfony_40.ogg")))
        .unwrap();

    // storing a list of tuples
    // - the first element is the time when the buffer will finish playing
    // - the second element is the buffer
    let mut buffers = RingBuf::new();

    // simple counter that we println!, doesn't have any usage
    let mut packet_num = 0u;

    // looping through the packets
    for packet in decoder.packets() {
        packet_num += 1;

        // the iterator produces Result<Packet, VorbisError> objects, so we need to unwrap
        let packet = packet.unwrap();

        // removing from the list of buffers all the buffers that should have stopped playing
        let now = time::precise_time_ns();
        while buffers.front().map(|&(stop, _)| stop).unwrap_or(now) < now {
            buffers.pop_front();
        }

        // calculating when the current packet will finish playing
        let packet_duration = packet.data.len() as f32 /
                              (packet.channels as u64 * packet.rate) as f32;
        let packet_play_finish = buffers.back().map(|&(stop, _)| stop).unwrap_or(now) +
                                 (packet_duration * 1000000000.0) as u64;

        // adding to the queue
        buffers.push((packet_play_finish, al::Buffer::gen()));

        // feeding OpenAL with our buffer
        let format = match packet.channels {
            1 => al::FormatMono16,
            2 => al::FormatStereo16,
            _ => unimplemented!()
        };

        unsafe { buffers.back_mut().unwrap().1
            .buffer_data(format, packet.data.as_slice(), packet.rate as al::ALsizei) };
        source.queue_buffer(&buffers.back().unwrap().1);

        if !source.is_playing() {
            source.play();
        }

        // if we have more than 4 buffers in queue, we can start sleeping until the first one
        // has finished playing
        if buffers.len() >= 4 {
            match buffers.front().map(|&(stop, _)| stop) {
                Some(wakeup) => {
                    let now = time::precise_time_ns();
                    if wakeup > now {
                        sleep(Duration::nanoseconds((wakeup - now) as i64));
                    }
                },
                _ => ()
            }
        }

        println!("packet {}", packet_num);
    }

    // sleeping until the last buffer has finished playing
    match buffers.back().map(|&(stop, _)| stop) {
        Some(wakeup) => {
            let now = time::precise_time_ns();
            sleep(Duration::nanoseconds((wakeup - now) as i64));
        },
        _ => ()
    }


    ctx.destroy();
    device.close().ok().expect("Unable to close device");
}
