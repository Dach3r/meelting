use crate::domain::speech::{normalize_level, to_recognition_rate};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use std::sync::mpsc;
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;

pub const MAX_SECONDS: u32 = 60;

pub struct Capture {
    stop: mpsc::Sender<()>,
    worker: JoinHandle<Result<Vec<f32>, &'static str>>,
}

impl Capture {
    pub fn start() -> Result<Capture, &'static str> {
        let (ready_tx, ready_rx) = mpsc::channel();
        let (stop, stop_rx) = mpsc::channel::<()>();
        let worker = std::thread::spawn(move || {
            let opened = open();
            let (stream, samples, channels, rate) = match opened {
                Ok(opened) => {
                    let _ = ready_tx.send(Ok(()));
                    opened
                }
                Err(code) => {
                    let _ = ready_tx.send(Err(code));
                    return Err(code);
                }
            };
            let _ = stop_rx.recv();
            drop(stream);
            let samples = std::mem::take(&mut *samples.lock().map_err(|_| "microphone_failed")?);
            let mut samples = to_recognition_rate(&samples, channels, rate)?;
            normalize_level(&mut samples);
            Ok(samples)
        });
        ready_rx.recv().map_err(|_| "microphone_unavailable")??;
        Ok(Capture { stop, worker })
    }

    pub fn finish(self) -> Result<Vec<f32>, &'static str> {
        let _ = self.stop.send(());
        self.worker.join().map_err(|_| "microphone_failed")?
    }
}

type Opened = (cpal::Stream, Arc<Mutex<Vec<f32>>>, u16, u32);

fn open() -> Result<Opened, &'static str> {
    let device = cpal::default_host()
        .default_input_device()
        .ok_or("microphone_unavailable")?;
    let config = device
        .default_input_config()
        .map_err(|_| "microphone_unavailable")?;
    let channels = config.channels();
    let rate = config.sample_rate().0;
    let limit = (rate * MAX_SECONDS) as usize * usize::from(channels);
    let samples = Arc::new(Mutex::new(Vec::new()));
    let sink = samples.clone();
    let push = move |data: &[f32]| {
        if let Ok(mut buffer) = sink.lock() {
            let room = limit.saturating_sub(buffer.len());
            buffer.extend_from_slice(&data[..data.len().min(room)]);
        }
    };
    let stream = match config.sample_format() {
        cpal::SampleFormat::F32 => device.build_input_stream(
            &config.into(),
            move |data: &[f32], _: &_| push(data),
            |_| {},
            None,
        ),
        cpal::SampleFormat::I16 => device.build_input_stream(
            &config.into(),
            move |data: &[i16], _: &_| {
                let converted: Vec<f32> = data.iter().map(|&s| f32::from(s) / 32_768.0).collect();
                push(&converted)
            },
            |_| {},
            None,
        ),
        _ => return Err("microphone_unavailable"),
    }
    .map_err(|_| "microphone_unavailable")?;
    stream.play().map_err(|_| "microphone_unavailable")?;
    Ok((stream, samples, channels, rate))
}
