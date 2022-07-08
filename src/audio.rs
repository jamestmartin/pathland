use cpal::*;
use cpal::traits::*;
use claxon::*;

pub struct Audio {
    device: Device,
    stream: Stream,
}

impl Audio {
    pub fn setup() -> Self {
        let device = default_host().default_output_device()
                                   .expect("No audio output device available.");
        if let Ok(name) = device.name() {
            log::info!("Using audio output device: {}", name);
        }
        let config = device.default_output_config()
                           .expect("Failed to get audio output device default configuration.");
        log::info!("Using audio output config: {:?}", config);
        let stream = match config.sample_format() {
            SampleFormat::F32 => create_output_stream::<f32>(&device, &config.config()),
            SampleFormat::I16 => create_output_stream::<i16>(&device, &config.config()),
            SampleFormat::U16 => create_output_stream::<u16>(&device, &config.config())
        };
        Self { device, stream }
    }
}

fn create_output_stream<T: Sample>(device: &Device, config: &StreamConfig) -> Stream {
    let sample_rate = config.sample_rate.0;
    let channels = config.channels as usize;
    let mut clock = 0;

    let music = read_music();

    device.build_output_stream(
        &config,
        move |output: &mut [T], _| {
            for frame in output.chunks_mut(channels) {
                for sample in frame.iter_mut() {
                    clock += 1;
                    if clock >= music.len() {
                        *sample = Sample::from(&0.0);
                        return;
                    }
                    *sample = Sample::from(&music[clock]);
                }
            }
        },
        move |err| {
            log::error!("Audio stream error: {}", err);
        }
    ).expect("Failed to create audio output stream.")
}

fn read_music() -> Box<[f32]> {
    let mut reader = FlacReader::open("continue.flac").unwrap();
    if reader.streaminfo().channels != 2 {
        panic!("Incorrect number of channels in FLAC (must be stereo).");
    }
    let mut music = Vec::new();
    let mut frames = reader.blocks();
    let mut buffer = Some(Vec::new());
    while let Some(block) = frames.read_next_or_eof(buffer.take().unwrap()).expect("Error reading FLAC stream.") {
        for sample in block.stereo_samples() {
            music.push(convert_sample(sample.0));
            music.push(convert_sample(sample.1));
        }
        buffer = Some(block.into_buffer());
    }
    music.into_boxed_slice()
}

fn convert_sample(sample: i32) -> f32 {
    sample as f32 / i16::MAX as f32
}
