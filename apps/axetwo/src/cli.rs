//! Command dispatch for `axetwo`.

use axetwo_stream::{pcm, Duplex, Flow, AUDIO_FUNCTION_ID, INTERFACE_GUID};
use axetwo_usb::layout;
use axetwo_win::{hub, setup, winusb};

type AnyResult = Result<(), Box<dyn std::error::Error>>;

const USAGE: &str = "\
usage: axetwo <command>

  status                  say what is connected and whether it is ready to stream
  probe                   print the unit's USB descriptors (read-only)
  setup                   put Windows' WinUSB driver on the unit's audio function
                          (run from an administrator prompt)
  capture [secs] [file]   record the unit's USB audio; optionally save a WAV file
  tone [secs] [dBFS]      play a 440 Hz sine to the unit (silence when no level
                          is given) while recording, and report on both streams
";

pub fn run(args: &[String]) -> AnyResult {
    let number = |i: usize| args.get(i).and_then(|s| s.parse::<f64>().ok());
    match args.first().map(String::as_str) {
        Some("status") => status(),
        Some("probe") => probe(),
        Some("setup") => setup_audio(),
        Some("capture") => capture(number(1).unwrap_or(2.0), args.get(2).map(String::as_str)),
        Some("tone") => tone(number(1).unwrap_or(1.0), number(2)),
        _ => {
            print!("{USAGE}");
            Ok(())
        }
    }
}

fn status() -> AnyResult {
    let attached = hub::attached_devices()?;
    let units: Vec<_> = attached.iter().filter(|d| d.vendor_id() == layout::VENDOR_ID).collect();
    if units.is_empty() {
        println!("No Axe-Fx II is connected and switched on.");
        return Ok(());
    }
    for d in units {
        let state = match d.product_id() {
            layout::PRODUCT_ID_RUNNING => "USB firmware running",
            layout::PRODUCT_ID_BOOTLOADER => "waiting for its USB firmware",
            _ => "unknown product",
        };
        println!("Found {:04x}:{:04x} on port {}: {state}.", d.vendor_id(), d.product_id(), d.port);
    }
    if winusb::interface_paths(INTERFACE_GUID)?.is_empty() {
        println!("Audio: not set up. Run `axetwo setup` from an administrator prompt.");
        return Ok(());
    }
    let duplex = Duplex::open()?;
    let l = duplex.layout();
    println!(
        "Audio: ready. {} Hz, {} channels to the computer, {} channels to the unit, {} bit.",
        duplex.sample_rate()?,
        l.capture.channels,
        l.playback.channels,
        l.capture.bits
    );
    Ok(())
}

fn setup_audio() -> AnyResult {
    let report = setup::bind_winusb(AUDIO_FUNCTION_ID, INTERFACE_GUID)?;
    println!("device: {}", report.instance_id);
    println!("driver: {} (from Windows' own winusb.inf)", report.driver);
    if report.reboot_required {
        println!("Windows asks for a restart to finish.");
    }
    Ok(())
}

fn report(stats: &axetwo_stream::Stats, peaks: &[u32], seconds: f64) {
    println!("frames: {} ({:.1} per second)", stats.frames, stats.frames as f64 / seconds);
    if let Some((lo, hi)) = stats.frames_per_ms {
        println!("frames per millisecond: {lo} to {hi}");
    }
    println!(
        "errors: {} capture packets, {} playback transfers",
        stats.capture_packet_errors, stats.playback_transfer_errors
    );
    for (ch, peak) in peaks.iter().enumerate() {
        println!("capture channel {} peak {:.1} dBFS", ch + 1, pcm::dbfs(*peak));
    }
}

fn track_peaks(peaks: &mut [u32], captured: &[u8]) {
    let frame = peaks.len() * pcm::SAMPLE_BYTES;
    for f in captured.chunks_exact(frame) {
        for (peak, s) in peaks.iter_mut().zip(f.as_chunks::<{ pcm::SAMPLE_BYTES }>().0) {
            *peak = (*peak).max(pcm::decode(s).unsigned_abs());
        }
    }
}

fn capture(seconds: f64, file: Option<&str>) -> AnyResult {
    let duplex = Duplex::open()?;
    let channels = usize::from(duplex.layout().capture.channels);
    let rate = duplex.sample_rate()?;
    let mut peaks = vec![0u32; channels];
    let mut audio = Vec::new();
    let started = std::time::Instant::now();
    let stats = duplex.run(|captured, _| {
        track_peaks(&mut peaks, captured);
        if file.is_some() {
            audio.extend_from_slice(captured);
        }
        if started.elapsed().as_secs_f64() < seconds {
            Flow::Continue
        } else {
            Flow::Stop
        }
    })?;
    report(&stats, &peaks, started.elapsed().as_secs_f64());
    if let Some(file) = file {
        crate::wav::write(file, rate, channels as u16, &audio)?;
        println!("wrote {file}");
    }
    Ok(())
}

fn tone(seconds: f64, level_db: Option<f64>) -> AnyResult {
    let duplex = Duplex::open()?;
    let channels = usize::from(duplex.layout().capture.channels);
    let out_frame = duplex.layout().playback.frame_bytes();
    let rate = f64::from(duplex.sample_rate()?);
    let amplitude = level_db.map_or(0.0, |db| (pcm::FULL_SCALE - 1.0) * 10f64.powf(db.min(0.0) / 20.0));
    let step = std::f64::consts::TAU * 440.0 / rate;
    let mut phase = 0.0f64;
    let mut peaks = vec![0u32; channels];
    let started = std::time::Instant::now();
    let stats = duplex.run(|captured, playback| {
        track_peaks(&mut peaks, captured);
        for frame in playback.chunks_exact_mut(out_frame) {
            let value = (phase.sin() * amplitude) as i32;
            phase = (phase + step) % std::f64::consts::TAU;
            for s in frame.as_chunks_mut::<{ pcm::SAMPLE_BYTES }>().0 {
                pcm::encode(value, s);
            }
        }
        if started.elapsed().as_secs_f64() < seconds {
            Flow::Continue
        } else {
            Flow::Stop
        }
    })?;
    report(&stats, &peaks, started.elapsed().as_secs_f64());
    Ok(())
}

fn probe() -> AnyResult {
    let attached = hub::attached_devices()?;
    let mut seen = false;
    for d in attached.iter().filter(|d| d.vendor_id() == layout::VENDOR_ID) {
        seen = true;
        println!(
            "device {:04x}:{:04x} port {} speed {} configuration {}",
            d.vendor_id(),
            d.product_id(),
            d.port,
            d.speed,
            d.current_configuration
        );
        println!("device descriptor: {}", hex(&d.device_descriptor));
        for index in [d.device_descriptor[14], d.device_descriptor[15], d.device_descriptor[16]] {
            if index != 0 {
                match hub::string_descriptor(&d.hub_path, d.port, index) {
                    Ok(s) => println!("string {index}: {s}"),
                    Err(e) => println!("string {index}: {e}"),
                }
            }
        }
        let cfg = hub::configuration_descriptor(&d.hub_path, d.port, 0)?;
        println!("configuration: {} bytes", cfg.len());
        let mut rest = cfg.as_slice();
        while let Some(&len) = rest.first() {
            let len = usize::from(len).max(2).min(rest.len());
            let (one, tail) = rest.split_at(len);
            println!("  {}", hex(one));
            rest = tail;
        }
        match axetwo_usb::parse_configuration(&cfg).map(|c| layout::stream_layout(&c)) {
            Ok(Ok(l)) => {
                for (name, s) in [("playback", l.playback), ("capture", l.capture)] {
                    println!(
                        "{name}: interface {} endpoint {:02x}, {} channels, {} bit, up to {} bytes per microframe",
                        s.interface, s.endpoint.address, s.channels, s.bits, s.endpoint.max_packet
                    );
                }
            }
            Ok(Err(e)) => println!("streams: {e}"),
            Err(e) => println!("streams: {e}"),
        }
    }
    if !seen {
        println!("No Fractal Audio device (vendor 2466) is attached.");
    }
    Ok(())
}

fn hex(bytes: &[u8]) -> String {
    bytes.iter().map(|b| format!("{b:02x}")).collect::<Vec<_>>().join(" ")
}
