use std::sync::Arc;

use ringbuf::{SharedRb, storage::Heap, traits::Consumer, wrap::caching::Caching};
use sdl3::audio::AudioCallback;

pub struct AudioBuffer {
    pub buffer: Caching<Arc<SharedRb<Heap<f32>>>, false, true>,
}

impl AudioCallback<f32> for AudioBuffer {
    fn callback(&mut self, stream: &mut sdl3::audio::AudioStream, requested: i32) {
        let mut audio_slice = Vec::<f32>::with_capacity(requested as usize);
        for _ in 0..requested {
            let sample_opt = self.buffer.try_pop();
            match sample_opt {
                Some(sample) => audio_slice.push(sample),
                None => audio_slice.push(0.0),
            }
        }
        stream.put_data_f32(&audio_slice).unwrap();
    }
}
