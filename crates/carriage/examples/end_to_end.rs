//! End-to-end walk-through: produce events into an in-controller ring, then read
//! them back downstream with full loss accounting and source high-water checkpoints.
//!
//! Run with: `cargo run -p open-ot-carriage --example end_to_end`

use open_ot_carriage::{EpochProducer, RawByteConsumer};

const EVENT_MESSAGE: u32 = 0x0003;
const SILENT_SOURCE: u32 = 88;
const NOISY_SOURCE: u32 = 99;

fn main() {
    // A deliberately small ring so the example actually overflows and drops records,
    // which is the interesting case for loss accounting.
    let mut producer = EpochProducer::new(512, 1, 1);

    // A "silent" source emits a few records early, then never speaks again.
    for _ in 0..5 {
        producer.emit_data(SILENT_SOURCE, EVENT_MESSAGE).unwrap();
    }

    // A "noisy" source floods the ring, evicting the silent source's records.
    for _ in 0..200 {
        producer.emit_data(NOISY_SOURCE, EVENT_MESSAGE).unwrap();
    }

    // The producer publishes how many records each source has produced so far.
    // This checkpoint is what lets a consumer reconcile a source that was dropped
    // and then went quiet.
    producer.checkpoint_high_water().unwrap();

    // Downstream: walk the raw ring bytes, validating CRC and following wrap markers.
    let mut consumer = RawByteConsumer::new();
    let batch = consumer.poll(producer.ring()).unwrap();
    println!(
        "delivered {} records this batch (lapped = {})",
        batch.records.len(),
        batch.lapped
    );

    println!(
        "source {SILENT_SOURCE}: delivered={}, lost={}",
        consumer.delivered_in_run(1, SILENT_SOURCE),
        consumer.lost_in_run(1, SILENT_SOURCE),
    );
    println!(
        "source {NOISY_SOURCE}:  delivered={}, lost={}",
        consumer.delivered_in_run(1, NOISY_SOURCE),
        consumer.lost_in_run(1, NOISY_SOURCE),
    );

    println!("\nloss events after poll:");
    for event in consumer.loss_events() {
        println!(
            "  source {} seq [{}..={}] count={} ({})",
            event.source_id,
            event.first_seq,
            event.last_seq,
            event.count,
            if event.synthetic {
                "synthetic gap"
            } else {
                "authoritative"
            },
        );
    }
}
