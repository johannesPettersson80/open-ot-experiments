use std::collections::BTreeMap;
use std::env;
use std::error::Error;
use std::ffi::OsString;
use std::fs;
use std::io::{self, Write};
use std::path::{Path, PathBuf};
use std::process::{Command, ExitCode, Stdio};
use std::thread;
use std::time::{Duration, Instant};

use open_ot_carriage::concurrent::{ConcurrentProducer, ConcurrentRawConsumer, ConcurrentStore};
use open_ot_carriage::consumer::LossAccountingConsumer;
use open_ot_carriage::registry::{
    EVENT_MESSAGE, EVENT_SOURCE_HIGH_WATER, KEY_SOURCE_HIGH_WATER, TY_ULINT,
};
use open_ot_carriage::wire::{Record, Slot};
use open_ot_conformance::{
    BatchObserver, EmbeddedAbsOracle, ExpectedSource, ObservationMetadata, ObservedReport,
    ReportInputs, expected_abs_slot,
};
use open_ot_shm::{FenceMode, SharedConcurrentStore};

const RUN_ID: u64 = 1;
const DEFAULT_CAP: usize = 4096;
const DEFAULT_SOURCES: u32 = 4;
const DEFAULT_PER_SOURCE: u64 = 5_000;
const DEFAULT_POLL_SLEEP_US: u64 = 100;
const DEFAULT_TIMEOUT_MS: u64 = 30_000;
const LITMUS_CAP: usize = 512;
const LITMUS_SOURCES: u32 = 1;
const LITMUS_PER_SOURCE: u64 = 200_000;

fn main() -> ExitCode {
    match real_main() {
        Ok(()) => ExitCode::SUCCESS,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(1)
        }
    }
}

fn real_main() -> Result<(), Box<dyn Error>> {
    let mut args = env::args_os();
    let _program = args.next();
    let Some(command) = args.next() else {
        return Err(usage().into());
    };
    let rest = args.collect::<Vec<_>>();
    match command.to_string_lossy().as_ref() {
        "producer" => producer_cmd(ProducerArgs::parse(&rest)?)?,
        "consumer" => consumer_cmd(ConsumerArgs::parse(&rest)?)?,
        "run" => run_cmd(RunArgs::parse(&rest)?)?,
        _ => return Err(usage().into()),
    }
    Ok(())
}

fn producer_cmd(args: ProducerArgs) -> Result<(), Box<dyn Error>> {
    let store = if args.shm.exists() {
        let store = SharedConcurrentStore::open_existing_with_mode(&args.shm, args.fence_mode)?;
        if store.capacity() != args.cap {
            return Err(format!(
                "existing shared mapping has cap {}, producer requested {}",
                store.capacity(),
                args.cap
            )
            .into());
        }
        store
    } else {
        SharedConcurrentStore::create_with_mode(&args.shm, args.cap, args.fence_mode)?
    };
    let mut producer = ConcurrentProducer::with_store(store);

    for seq in 0..args.per_source {
        for source_id in source_ids(args.sources) {
            let record = message_record_with_expected_abs(&producer, source_id, seq)?;
            publish_record(&mut producer, args.append_mode, &record, "producer write")?;
        }
    }

    for source_id in source_ids(args.sources) {
        let record =
            source_high_water_record_with_expected_abs(&producer, source_id, args.per_source)?;
        publish_record(
            &mut producer,
            args.append_mode,
            &record,
            "producer high-water write",
        )?;
    }
    write_done_atomic(&done_path(&args.shm))?;
    println!(
        "producer: fence={} append_mode={} sources={} per_source={} data_total={} stream_total={} head_abs={} lost_count={}",
        args.fence_mode.as_str(),
        args.append_mode.as_str(),
        args.sources,
        args.per_source,
        u64::from(args.sources) * args.per_source,
        u64::from(args.sources) * (args.per_source + 1),
        producer.store().load_head_acquire(),
        producer.store().load_lost_acquire()
    );
    Ok(())
}

fn publish_record(
    producer: &mut ConcurrentProducer<SharedConcurrentStore>,
    append_mode: AppendMode,
    record: &Record,
    context: &str,
) -> Result<(), Box<dyn Error>> {
    match append_mode {
        AppendMode::WriteRecord => producer
            .write_record(record)
            .map_err(|error| format!("{context} failed: {error:?}").into()),
        AppendMode::Encoded => {
            let encoded = record
                .encode(true)
                .map_err(|error| format!("{context} encode failed: {error:?}"))?;
            producer
                .append_encoded(&encoded)
                .map_err(|error| format!("{context} encoded append failed: {error:?}").into())
        }
    }
}

fn consumer_cmd(args: ConsumerArgs) -> Result<(), Box<dyn Error>> {
    let store = SharedConcurrentStore::open_existing_with_mode(&args.shm, args.fence_mode)?
        .with_recheck_stall(Duration::from_micros(args.recheck_stall_us));
    let mut raw = ConcurrentRawConsumer::with_store(store.clone());
    let mut accounting = LossAccountingConsumer::new();
    let deadline = Instant::now() + Duration::from_millis(args.timeout_ms);
    let mut observer = BatchObserver::new(EmbeddedAbsOracle);

    loop {
        let batch = raw
            .poll()
            .map_err(|error| format!("consumer poll failed: {error:?}"))?;
        observer.observe_batch(&batch)?;
        accounting.account_batch(&batch);

        if args.done.exists() && raw.cursor_abs() == raw.head_abs() {
            break;
        }

        if Instant::now() > deadline {
            return Err("consumer timed out before producer completion".into());
        }

        if args.poll_sleep_us > 0 {
            thread::sleep(Duration::from_micros(args.poll_sleep_us));
        } else {
            thread::yield_now();
        }
    }

    let observed = ObservedReport::from_consumer(ReportInputs {
        metadata: ObservationMetadata::new(
            args.mode.as_str(),
            args.append_mode.as_str(),
            args.fence_mode,
        ),
        expected_sources: expected_sources(args.sources, args.per_source).collect(),
        raw: &raw,
        accounting: &accounting,
        store: &store,
        stale_violations: observer.into_violations(),
    });
    observed.write(&args.out)?;
    println!("{}", observed.summary_line());
    Ok(())
}

fn run_cmd(args: RunArgs) -> Result<(), Box<dyn Error>> {
    let started = Instant::now();
    let _ = fs::remove_file(&args.shm);
    let done = done_path(&args.shm);
    let observed = observed_path(&done);
    let _ = fs::remove_file(&done);
    let _ = fs::remove_file(&observed);

    let store = SharedConcurrentStore::create_with_mode(&args.shm, args.cap, args.fence_mode)?;
    drop(store);

    let exe = env::current_exe()?;
    let pinning = args.mode == RunMode::Litmus && can_use_taskset();
    let mut consumer = command_for(&exe, pinning.then_some(1));
    consumer
        .arg("consumer")
        .arg("--mode")
        .arg(args.mode.as_str())
        .arg(fence_flag(args.fence_mode))
        .arg("--append-mode")
        .arg(args.append_mode.as_str())
        .arg("--shm")
        .arg(&args.shm)
        .arg("--done")
        .arg(&done)
        .arg("--out")
        .arg(&observed)
        .arg("--sources")
        .arg(args.sources.to_string())
        .arg("--per-source")
        .arg(args.per_source.to_string())
        .arg("--poll-sleep-us")
        .arg(args.poll_sleep_us.to_string())
        .arg("--recheck-stall-us")
        .arg(args.recheck_stall_us.to_string())
        .arg("--timeout-ms")
        .arg(args.timeout_ms.to_string());
    let mut consumer = consumer.spawn()?;

    let mut producer = command_for(&exe, pinning.then_some(0));
    producer
        .arg("producer")
        .arg(fence_flag(args.fence_mode))
        .arg("--append-mode")
        .arg(args.append_mode.as_str())
        .arg("--shm")
        .arg(&args.shm)
        .arg("--cap")
        .arg(args.cap.to_string())
        .arg("--sources")
        .arg(args.sources.to_string())
        .arg("--per-source")
        .arg(args.per_source.to_string());
    let mut producer = producer.spawn()?;

    let producer_status = producer.wait()?;
    if !producer_status.success() {
        let _ = consumer.kill();
        return Err(format!("producer failed with {producer_status}").into());
    }
    let consumer_status = consumer.wait()?;
    if !consumer_status.success() {
        return Err(format!("consumer failed with {consumer_status}").into());
    }

    let observed = ObservedReport::read(&observed)?;
    assert_run_result(
        args.fence_mode,
        args.cap,
        args.sources,
        args.per_source,
        &observed,
    )?;
    let elapsed = started.elapsed();
    println!(
        "run: mode={} fence={} append_mode={} elapsed_ms={} pinned={} stale_violations={}",
        args.mode.as_str(),
        args.fence_mode.as_str(),
        args.append_mode.as_str(),
        elapsed.as_millis(),
        pinning,
        observed.stale_violations.len()
    );
    println!("{}", observed.summary_line());
    for source in &observed.sources {
        println!(
            "source {}: expected_total={} delivered={} lost={} reconciled={}",
            source.source_id,
            source.expected_total,
            source.delivered,
            source.lost,
            source.delivered + source.lost
        );
    }
    if args.fence_mode == FenceMode::Unfenced && observed.stale_violations.is_empty() {
        println!(
            "unfenced: stale oracle did not fire in this run; this is a non-reproduction, not proof of safety"
        );
    }
    for stale in &observed.stale_violations {
        println!(
            "stale: kind={} source={} seq={} expected_abs={} actual_abs={} crc_passed={}",
            stale.kind.as_str(),
            stale.source_id,
            stale.seq,
            stale.expected_abs,
            stale.actual_abs,
            stale.crc_passed
        );
    }
    Ok(())
}

fn assert_run_result(
    fence_mode: FenceMode,
    cap: usize,
    sources: u32,
    per_source: u64,
    observed: &ObservedReport,
) -> Result<(), Box<dyn Error>> {
    if observed.delivered_total == 0 {
        return Err("consumer made no progress".into());
    }
    if observed.head_abs <= cap as u64 * 100 {
        return Err(format!(
            "insufficient absolute progress: head_abs={} cap={cap}",
            observed.head_abs
        )
        .into());
    }
    if observed.lost_count == 0 {
        return Err("producer did not report ring evictions".into());
    }
    let expected_total = u64::from(sources) * (per_source + 1);
    if fence_mode == FenceMode::Fenced
        && observed.delivered_total + observed.lost_total < expected_total
    {
        return Err(format!(
            "consumer did not reconcile the full in-band stream: delivered={} lost={} expected_total={expected_total}",
            observed.delivered_total, observed.lost_total
        )
        .into());
    }
    if fence_mode == FenceMode::Fenced && !observed.stale_violations.is_empty() {
        return Err(format!(
            "fenced run accepted stale records: {}",
            observed.stale_violations.len()
        )
        .into());
    }
    let observed_by_source = observed
        .sources
        .iter()
        .map(|source| (source.source_id, source))
        .collect::<BTreeMap<_, _>>();
    for expected in expected_sources(sources, per_source) {
        let Some(source) = observed_by_source.get(&expected.source_id) else {
            return Err(format!("missing observed source {}", expected.source_id).into());
        };
        if source.expected_total != expected.expected_total {
            return Err(format!(
                "source {} expected-total mismatch: run={} observed={}",
                expected.source_id, expected.expected_total, source.expected_total
            )
            .into());
        }
        if fence_mode == FenceMode::Fenced
            && source.delivered + source.lost != expected.expected_total
        {
            return Err(format!(
                "source {} in-band reconciliation failed: delivered={} lost={} expected_total={}",
                expected.source_id, source.delivered, source.lost, expected.expected_total
            )
            .into());
        }
    }
    Ok(())
}

#[derive(Debug, Clone)]
struct ProducerArgs {
    shm: PathBuf,
    cap: usize,
    sources: u32,
    per_source: u64,
    fence_mode: FenceMode,
    append_mode: AppendMode,
}

impl ProducerArgs {
    fn parse(args: &[OsString]) -> Result<Self, String> {
        let mut parser = ArgParser::new(args);
        let parsed = Self {
            shm: parser.path("--shm")?,
            cap: parser.usize("--cap")?,
            sources: parser.u32("--sources")?,
            per_source: parser.u64("--per-source")?,
            fence_mode: parser.fence_mode()?,
            append_mode: parser.append_mode()?.unwrap_or(AppendMode::WriteRecord),
        };
        parser.finish()?;
        Ok(parsed)
    }
}

#[derive(Debug, Clone)]
struct ConsumerArgs {
    mode: RunMode,
    shm: PathBuf,
    done: PathBuf,
    out: PathBuf,
    sources: u32,
    per_source: u64,
    poll_sleep_us: u64,
    recheck_stall_us: u64,
    timeout_ms: u64,
    fence_mode: FenceMode,
    append_mode: AppendMode,
}

impl ConsumerArgs {
    fn parse(args: &[OsString]) -> Result<Self, String> {
        let mut parser = ArgParser::new(args);
        let mode = parser.run_mode()?.unwrap_or(RunMode::Smoke);
        let done = parser.path("--done")?;
        let out = parser
            .optional_path("--out")?
            .unwrap_or_else(|| observed_path(&done));
        let parsed = Self {
            mode,
            shm: parser.path("--shm")?,
            done,
            out,
            sources: parser.u32("--sources")?,
            per_source: parser.u64("--per-source")?,
            poll_sleep_us: parser
                .optional_u64("--poll-sleep-us")?
                .unwrap_or(DEFAULT_POLL_SLEEP_US),
            recheck_stall_us: parser.optional_u64("--recheck-stall-us")?.unwrap_or(0),
            timeout_ms: parser
                .optional_u64("--timeout-ms")?
                .unwrap_or(DEFAULT_TIMEOUT_MS),
            fence_mode: parser.fence_mode()?,
            append_mode: parser.append_mode()?.unwrap_or(AppendMode::WriteRecord),
        };
        parser.finish()?;
        Ok(parsed)
    }
}

#[derive(Debug, Clone)]
struct RunArgs {
    mode: RunMode,
    shm: PathBuf,
    cap: usize,
    sources: u32,
    per_source: u64,
    poll_sleep_us: u64,
    recheck_stall_us: u64,
    timeout_ms: u64,
    fence_mode: FenceMode,
    append_mode: AppendMode,
}

impl RunArgs {
    fn parse(args: &[OsString]) -> Result<Self, String> {
        let mut parser = ArgParser::new(args);
        let mode = parser.run_mode()?.unwrap_or(RunMode::Smoke);
        let defaults = mode.defaults();
        let parsed = Self {
            mode,
            shm: parser
                .optional_path("--shm")?
                .unwrap_or_else(default_shm_path),
            cap: parser.optional_usize("--cap")?.unwrap_or(defaults.cap),
            sources: parser
                .optional_u32("--sources")?
                .unwrap_or(defaults.sources),
            per_source: parser
                .optional_u64("--per-source")?
                .unwrap_or(defaults.per_source),
            poll_sleep_us: parser
                .optional_u64("--poll-sleep-us")?
                .unwrap_or(DEFAULT_POLL_SLEEP_US),
            recheck_stall_us: parser.optional_u64("--recheck-stall-us")?.unwrap_or(0),
            timeout_ms: parser
                .optional_u64("--timeout-ms")?
                .unwrap_or(DEFAULT_TIMEOUT_MS),
            fence_mode: parser.fence_mode()?,
            append_mode: parser.append_mode()?.unwrap_or(AppendMode::WriteRecord),
        };
        parser.finish()?;
        Ok(parsed)
    }
}

struct ArgParser {
    args: Vec<OsString>,
}

impl ArgParser {
    fn new(args: &[OsString]) -> Self {
        Self {
            args: args.to_vec(),
        }
    }

    fn path(&mut self, name: &str) -> Result<PathBuf, String> {
        self.optional_path(name)?
            .ok_or_else(|| format!("missing required argument {name}"))
    }

    fn optional_path(&mut self, name: &str) -> Result<Option<PathBuf>, String> {
        self.take(name).map(|value| value.map(PathBuf::from))
    }

    fn usize(&mut self, name: &str) -> Result<usize, String> {
        self.optional_usize(name)?
            .ok_or_else(|| format!("missing required argument {name}"))
    }

    fn optional_usize(&mut self, name: &str) -> Result<Option<usize>, String> {
        self.take(name)
            .and_then(|value| parse_optional(value, name))
    }

    fn u32(&mut self, name: &str) -> Result<u32, String> {
        self.optional_u32(name)?
            .ok_or_else(|| format!("missing required argument {name}"))
    }

    fn optional_u32(&mut self, name: &str) -> Result<Option<u32>, String> {
        self.take(name)
            .and_then(|value| parse_optional(value, name))
    }

    fn u64(&mut self, name: &str) -> Result<u64, String> {
        self.optional_u64(name)?
            .ok_or_else(|| format!("missing required argument {name}"))
    }

    fn optional_u64(&mut self, name: &str) -> Result<Option<u64>, String> {
        self.take(name)
            .and_then(|value| parse_optional(value, name))
    }

    fn run_mode(&mut self) -> Result<Option<RunMode>, String> {
        self.take("--mode")?
            .map(|value| RunMode::parse(&value.to_string_lossy()))
            .transpose()
    }

    fn fence_mode(&mut self) -> Result<FenceMode, String> {
        let fenced = self.take_flag("--fenced")?;
        let unfenced = self.take_flag("--unfenced")?;
        match (fenced, unfenced) {
            (true, true) => Err("--fenced and --unfenced are mutually exclusive".to_string()),
            (_, true) => Ok(FenceMode::Unfenced),
            _ => Ok(FenceMode::Fenced),
        }
    }

    fn append_mode(&mut self) -> Result<Option<AppendMode>, String> {
        self.take("--append-mode")?
            .map(|value| AppendMode::parse(&value.to_string_lossy()))
            .transpose()
    }

    fn take_flag(&mut self, name: &str) -> Result<bool, String> {
        let Some(index) = self.args.iter().position(|arg| arg == name) else {
            return Ok(false);
        };
        self.args.remove(index);
        Ok(true)
    }

    fn take(&mut self, name: &str) -> Result<Option<OsString>, String> {
        let Some(index) = self.args.iter().position(|arg| arg == name) else {
            return Ok(None);
        };
        self.args.remove(index);
        if index >= self.args.len() {
            return Err(format!("missing value for {name}"));
        }
        Ok(Some(self.args.remove(index)))
    }

    fn finish(self) -> Result<(), String> {
        if self.args.is_empty() {
            Ok(())
        } else {
            Err(format!("unexpected arguments: {:?}", self.args))
        }
    }
}

fn parse_optional<T: std::str::FromStr>(
    value: Option<OsString>,
    name: &str,
) -> Result<Option<T>, String> {
    value
        .map(|value| {
            value
                .to_string_lossy()
                .parse::<T>()
                .map_err(|_| format!("invalid value for {name}"))
        })
        .transpose()
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum RunMode {
    Smoke,
    Litmus,
}

impl RunMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "smoke" => Ok(Self::Smoke),
            "litmus" => Ok(Self::Litmus),
            _ => Err(format!("invalid --mode {value}; expected smoke or litmus")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Smoke => "smoke",
            Self::Litmus => "litmus",
        }
    }

    fn defaults(self) -> RunDefaults {
        match self {
            Self::Smoke => RunDefaults {
                cap: DEFAULT_CAP,
                sources: DEFAULT_SOURCES,
                per_source: DEFAULT_PER_SOURCE,
            },
            Self::Litmus => RunDefaults {
                cap: LITMUS_CAP,
                sources: LITMUS_SOURCES,
                per_source: LITMUS_PER_SOURCE,
            },
        }
    }
}

struct RunDefaults {
    cap: usize,
    sources: u32,
    per_source: u64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum AppendMode {
    WriteRecord,
    Encoded,
}

impl AppendMode {
    fn parse(value: &str) -> Result<Self, String> {
        match value {
            "write-record" => Ok(Self::WriteRecord),
            "encoded" => Ok(Self::Encoded),
            _ => Err(format!(
                "invalid --append-mode {value}; expected write-record or encoded"
            )),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::WriteRecord => "write-record",
            Self::Encoded => "encoded",
        }
    }
}

fn write_done_atomic(path: &Path) -> io::Result<()> {
    let tmp = path.with_extension("done.tmp");
    let mut file = fs::File::create(&tmp)?;
    writeln!(file, "done")?;
    file.sync_all()?;
    fs::rename(tmp, path)
}

fn expected_sources(count: u32, per_source: u64) -> impl Iterator<Item = ExpectedSource> {
    source_ids(count).map(move |source_id| ExpectedSource::new(RUN_ID, source_id, per_source + 1))
}

fn source_ids(count: u32) -> impl Iterator<Item = u32> {
    1..=count
}

fn message_record_with_expected_abs(
    producer: &ConcurrentProducer<SharedConcurrentStore>,
    source_id: u32,
    seq: u64,
) -> Result<Record, Box<dyn Error>> {
    record_with_expected_abs(producer, message_record(source_id, seq))
}

fn source_high_water_record_with_expected_abs(
    producer: &ConcurrentProducer<SharedConcurrentStore>,
    source_id: u32,
    produced_count: u64,
) -> Result<Record, Box<dyn Error>> {
    let mut record = Record::new(
        1_780_000_000_000_000_000 + produced_count,
        RUN_ID,
        produced_count,
        source_id,
        EVENT_SOURCE_HIGH_WATER,
    );
    record.slots.push(Slot::new(
        KEY_SOURCE_HIGH_WATER,
        TY_ULINT,
        produced_count.to_le_bytes(),
    ));
    record_with_expected_abs(producer, record)
}

fn record_with_expected_abs(
    producer: &ConcurrentProducer<SharedConcurrentStore>,
    mut record: Record,
) -> Result<Record, Box<dyn Error>> {
    record.slots.push(expected_abs_slot(0));
    let encoded_len = record
        .encode(true)
        .map_err(|error| format!("placeholder encode failed: {error:?}"))?
        .len();
    let expected_abs = producer.next_start_abs(encoded_len);
    let slot = record
        .slots
        .last_mut()
        .expect("message_record inserts oracle slot");
    slot.payload = expected_abs.to_le_bytes().to_vec();
    Ok(record)
}

fn message_record(source_id: u32, seq: u64) -> Record {
    Record::new(
        1_780_000_000_000_000_000 + seq,
        RUN_ID,
        seq,
        source_id,
        EVENT_MESSAGE,
    )
}

fn fence_flag(fence_mode: FenceMode) -> &'static str {
    match fence_mode {
        FenceMode::Fenced => "--fenced",
        FenceMode::Unfenced => "--unfenced",
    }
}

fn command_for(exe: &Path, core: Option<usize>) -> Command {
    if let Some(core) = core {
        let mut command = Command::new("taskset");
        command.arg("-c").arg(core.to_string()).arg(exe);
        command
    } else {
        Command::new(exe)
    }
}

fn can_use_taskset() -> bool {
    if thread::available_parallelism().map_or(1, usize::from) < 2 {
        return false;
    }
    Command::new("taskset")
        .arg("--version")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok_and(|status| status.success())
}

fn done_path(shm: &Path) -> PathBuf {
    PathBuf::from(format!("{}.done", shm.display()))
}

fn observed_path(status: &Path) -> PathBuf {
    PathBuf::from(format!("{}.observed", status.display()))
}

fn default_shm_path() -> PathBuf {
    PathBuf::from(format!("/dev/shm/open-ot-live-{}", std::process::id()))
}

fn usage() -> &'static str {
    "usage: open-ot-live-harness run [--mode smoke|litmus] [--fenced|--unfenced] [--shm PATH] [--cap BYTES] [--sources N] [--per-source N]\n\
     or: open-ot-live-harness producer [--fenced|--unfenced] --shm PATH --cap BYTES --sources N --per-source N\n\
     or: open-ot-live-harness consumer [--mode smoke|litmus] [--fenced|--unfenced] --shm PATH --done FILE --sources N --per-source N [--out FILE]"
}
