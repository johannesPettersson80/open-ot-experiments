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
use open_ot_carriage::ring::ReadRecord;
use open_ot_carriage::wire::{Record, Slot};
use open_ot_shm::{FenceMode, SharedConcurrentStore};

const RUN_ID: u64 = 1;
const KEY_EXPECTED_RECORD_START_ABS: u16 = 0x8001;
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
    let mut stale_violations = Vec::new();

    loop {
        let batch = raw
            .poll()
            .map_err(|error| format!("consumer poll failed: {error:?}"))?;
        stale_violations.extend(stale_violations_from_batch(&batch.records)?);
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
        mode: args.mode,
        fence_mode: args.fence_mode,
        append_mode: args.append_mode,
        source_count: args.sources,
        per_source: args.per_source,
        raw: &raw,
        accounting: &accounting,
        store: &store,
        stale_violations,
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
            "stale: source={} seq={} expected_abs={} actual_abs={} crc_passed={}",
            stale.source_id, stale.seq, stale.expected_abs, stale.actual_abs, stale.crc_passed
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

#[derive(Debug, Clone, Copy)]
struct SourceExpected {
    source_id: u32,
    expected_total: u64,
}

#[derive(Debug, Clone)]
struct SourceObserved {
    source_id: u32,
    expected_total: u64,
    delivered: u64,
    lost: u64,
}

#[derive(Debug, Clone)]
struct StaleViolation {
    source_id: u32,
    seq: u64,
    expected_abs: u64,
    actual_abs: u64,
    crc_passed: bool,
}

#[derive(Debug, Clone)]
struct ObservedReport {
    mode: RunMode,
    fence_mode: FenceMode,
    append_mode: AppendMode,
    cap: usize,
    head_abs: u64,
    lost_count: u64,
    lapped_batches: u64,
    overwritten_retries: u64,
    rejected_records: u64,
    delivered_total: u64,
    lost_total: u64,
    stale_violations: Vec<StaleViolation>,
    sources: Vec<SourceObserved>,
}

struct ReportInputs<'a> {
    mode: RunMode,
    fence_mode: FenceMode,
    append_mode: AppendMode,
    source_count: u32,
    per_source: u64,
    raw: &'a ConcurrentRawConsumer<SharedConcurrentStore>,
    accounting: &'a LossAccountingConsumer,
    store: &'a SharedConcurrentStore,
    stale_violations: Vec<StaleViolation>,
}

impl ObservedReport {
    fn from_consumer(inputs: ReportInputs<'_>) -> Self {
        let mut observed_sources = Vec::new();
        for expected in expected_sources(inputs.source_count, inputs.per_source) {
            let delivered = inputs
                .accounting
                .delivered_in_run(RUN_ID, expected.source_id);
            let lost = inputs.accounting.lost_in_run(RUN_ID, expected.source_id);
            observed_sources.push(SourceObserved {
                source_id: expected.source_id,
                expected_total: expected.expected_total,
                delivered,
                lost,
            });
        }
        let delivered_total = observed_sources.iter().map(|source| source.delivered).sum();
        let lost_total = observed_sources.iter().map(|source| source.lost).sum();
        Self {
            mode: inputs.mode,
            fence_mode: inputs.fence_mode,
            append_mode: inputs.append_mode,
            cap: inputs.store.capacity(),
            head_abs: inputs.store.load_head_acquire(),
            lost_count: inputs.store.load_lost_acquire(),
            lapped_batches: inputs.raw.lapped_batches(),
            overwritten_retries: inputs.raw.overwritten_retries(),
            rejected_records: inputs.raw.rejected_records(),
            delivered_total,
            lost_total,
            stale_violations: inputs.stale_violations,
            sources: observed_sources,
        }
    }

    fn write(&self, path: &Path) -> io::Result<()> {
        let mut out = String::new();
        out.push_str(&format!("mode {}\n", self.mode.as_str()));
        out.push_str(&format!("fence {}\n", self.fence_mode.as_str()));
        out.push_str(&format!("append_mode {}\n", self.append_mode.as_str()));
        out.push_str(&format!("cap {}\n", self.cap));
        out.push_str(&format!("head_abs {}\n", self.head_abs));
        out.push_str(&format!("lost_count {}\n", self.lost_count));
        out.push_str(&format!("lapped_batches {}\n", self.lapped_batches));
        out.push_str(&format!(
            "overwritten_retries {}\n",
            self.overwritten_retries
        ));
        out.push_str(&format!("rejected_records {}\n", self.rejected_records));
        out.push_str(&format!("delivered_total {}\n", self.delivered_total));
        out.push_str(&format!("lost_total {}\n", self.lost_total));
        out.push_str(&format!(
            "stale_violations {}\n",
            self.stale_violations.len()
        ));
        for stale in &self.stale_violations {
            out.push_str(&format!(
                "stale {} {} {} {} {}\n",
                stale.source_id, stale.seq, stale.expected_abs, stale.actual_abs, stale.crc_passed
            ));
        }
        for source in &self.sources {
            out.push_str(&format!(
                "source {} {} {} {}\n",
                source.source_id, source.expected_total, source.delivered, source.lost
            ));
        }
        fs::write(path, out)
    }

    fn read(path: &Path) -> Result<Self, Box<dyn Error>> {
        let content = fs::read_to_string(path)?;
        let mut mode = None;
        let mut fence_mode = None;
        let mut append_mode = None;
        let mut cap = None;
        let mut head_abs = None;
        let mut lost_count = None;
        let mut lapped_batches = None;
        let mut overwritten_retries = None;
        let mut rejected_records = None;
        let mut delivered_total = None;
        let mut lost_total = None;
        let mut stale_count = None;
        let mut stale_violations = Vec::new();
        let mut sources = Vec::new();

        for line in content.lines() {
            let parts = line.split_whitespace().collect::<Vec<_>>();
            match parts.as_slice() {
                ["mode", value] => mode = Some(RunMode::parse(value)?),
                ["fence", value] => fence_mode = Some(FenceMode::parse(value)?),
                ["append_mode", value] => append_mode = Some(AppendMode::parse(value)?),
                ["cap", value] => cap = Some(value.parse()?),
                ["head_abs", value] => head_abs = Some(value.parse()?),
                ["lost_count", value] => lost_count = Some(value.parse()?),
                ["lapped_batches", value] => lapped_batches = Some(value.parse()?),
                ["overwritten_retries", value] => overwritten_retries = Some(value.parse()?),
                ["rejected_records", value] => rejected_records = Some(value.parse()?),
                ["delivered_total", value] => delivered_total = Some(value.parse()?),
                ["lost_total", value] => lost_total = Some(value.parse()?),
                ["stale_violations", value] => stale_count = Some(value.parse()?),
                [
                    "stale",
                    source_id,
                    seq,
                    expected_abs,
                    actual_abs,
                    crc_passed,
                ] => {
                    stale_violations.push(StaleViolation {
                        source_id: source_id.parse()?,
                        seq: seq.parse()?,
                        expected_abs: expected_abs.parse()?,
                        actual_abs: actual_abs.parse()?,
                        crc_passed: crc_passed.parse()?,
                    });
                }
                ["source", source_id, expected_total, delivered, lost] => {
                    sources.push(SourceObserved {
                        source_id: source_id.parse()?,
                        expected_total: expected_total.parse()?,
                        delivered: delivered.parse()?,
                        lost: lost.parse()?,
                    })
                }
                _ => return Err(format!("invalid observed line: {line}").into()),
            }
        }

        if stale_count != Some(stale_violations.len()) {
            return Err("stale_violations count mismatch".into());
        }
        Ok(Self {
            mode: mode.ok_or("missing mode")?,
            fence_mode: fence_mode.ok_or("missing fence")?,
            append_mode: append_mode.ok_or("missing append_mode")?,
            cap: cap.ok_or("missing cap")?,
            head_abs: head_abs.ok_or("missing head_abs")?,
            lost_count: lost_count.ok_or("missing lost_count")?,
            lapped_batches: lapped_batches.ok_or("missing lapped_batches")?,
            overwritten_retries: overwritten_retries.ok_or("missing overwritten_retries")?,
            rejected_records: rejected_records.ok_or("missing rejected_records")?,
            delivered_total: delivered_total.ok_or("missing delivered_total")?,
            lost_total: lost_total.ok_or("missing lost_total")?,
            stale_violations,
            sources,
        })
    }

    fn summary_line(&self) -> String {
        format!(
            "summary: mode={} fence={} append_mode={} cap={} head_abs={} lost_count={} delivered={} lost={} lapped={} retries={} rejected={} stale={}",
            self.mode.as_str(),
            self.fence_mode.as_str(),
            self.append_mode.as_str(),
            self.cap,
            self.head_abs,
            self.lost_count,
            self.delivered_total,
            self.lost_total,
            self.lapped_batches,
            self.overwritten_retries,
            self.rejected_records,
            self.stale_violations.len()
        )
    }
}

fn write_done_atomic(path: &Path) -> io::Result<()> {
    let tmp = path.with_extension("done.tmp");
    let mut file = fs::File::create(&tmp)?;
    writeln!(file, "done")?;
    file.sync_all()?;
    fs::rename(tmp, path)
}

fn expected_sources(count: u32, per_source: u64) -> impl Iterator<Item = SourceExpected> {
    source_ids(count).map(move |source_id| SourceExpected {
        source_id,
        expected_total: per_source + 1,
    })
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
    record.slots.push(Slot::new(
        KEY_EXPECTED_RECORD_START_ABS,
        TY_ULINT,
        0u64.to_le_bytes(),
    ));
    let encoded_len = record
        .encode(true)
        .map_err(|error| format!("placeholder encode failed: {error:?}"))?
        .len();
    let expected_abs = producer.next_start_abs(encoded_len);
    let slot = record
        .slots
        .iter_mut()
        .find(|slot| slot.key == KEY_EXPECTED_RECORD_START_ABS)
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

fn stale_violations_from_batch(records: &[ReadRecord]) -> Result<Vec<StaleViolation>, String> {
    let mut stale = Vec::new();
    for read in records {
        let expected_abs = expected_record_start_abs(&read.record)?;
        if read.start_abs != expected_abs {
            stale.push(StaleViolation {
                source_id: read.record.source_id,
                seq: read.record.seq,
                expected_abs,
                actual_abs: read.start_abs,
                crc_passed: true,
            });
        }
    }
    Ok(stale)
}

fn expected_record_start_abs(record: &Record) -> Result<u64, String> {
    record
        .slots
        .iter()
        .find(|slot| slot.key == KEY_EXPECTED_RECORD_START_ABS)
        .ok_or_else(|| {
            format!(
                "record source={} seq={} has no expectedRecordStartAbs slot",
                record.source_id, record.seq
            )
        })
        .and_then(|slot| {
            if slot.ty != TY_ULINT || slot.payload.len() != 8 {
                return Err(format!(
                    "record source={} seq={} has invalid expectedRecordStartAbs slot",
                    record.source_id, record.seq
                ));
            }
            Ok(u64::from_le_bytes(
                slot.payload
                    .as_slice()
                    .try_into()
                    .expect("length checked above"),
            ))
        })
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
