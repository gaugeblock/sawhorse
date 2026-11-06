//! Flight-recorder spans around provider completion (`GB-243`).
//!
//! This is the third of the three choke points, and the only asynchronous one. That difference matters:
//! [`gaugeblock_workspace::SpanGuard`] writes on `Drop`, so holding one across an `.await` makes the
//! surrounding future non-`Send` and pushes that bound into the provider traits and every caller. So
//! this module records *after* the call returns instead - capture the clock, await, append the span.
//!
//! Nothing here can fail a completion. Telemetry that can fail a caller is worse than no telemetry: a
//! read-only checkout, a full disk or a missing suite root must all mean "no span", never "the
//! inference failed".

use gaugeblock_contracts::{SpanStatus, TelemetrySpan};
use gaugeblock_workspace::{GaugeblockPaths, append_span, now_ms};
use uuid::Uuid;

/// Trace id of the run this process belongs to, set by the parent so child spans join it.
pub const TRACE_ENV: &str = "GAUGEBLOCK_TRACE_ID";

/// Records one completed provider call against the suite's telemetry log.
///
/// `started_at_ms` must be captured *before* the await. `outcome` carries the first diagnostic line on
/// failure and nothing on success, which is what a repair pair is keyed on.
///
/// Deliberately records no prompt or completion text. There is no field for it in [`TelemetrySpan`],
/// and that is the stronger guarantee than a redaction pass: the log is plaintext on disk and stays
/// safe by never carrying the payload in the first place.
pub fn record_completion(started_at_ms: u64, outcome: Result<(), &str>) {
    let Ok(cwd) = std::env::current_dir() else {
        return;
    };
    let Ok(paths) = GaugeblockPaths::discover(&cwd) else {
        return;
    };

    let span = completion_span(trace_id_from_env(), started_at_ms, now_ms(), outcome);
    let _ = append_span(&paths.telemetry_events(), &span);
}

/// The trace id this process shares with its parent, or a fresh one when it is running alone.
#[must_use]
pub fn trace_id_from_env() -> Uuid {
    std::env::var(TRACE_ENV)
        .ok()
        .and_then(|raw| Uuid::parse_str(raw.trim()).ok())
        .unwrap_or_else(Uuid::new_v4)
}

/// Builds the span for a completion that ran from `started_at_ms` to `closed_at_ms`.
///
/// Split out from [`record_completion`] so the shape is testable without a filesystem or a clock.
#[must_use]
pub fn completion_span(
    trace_id: Uuid,
    started_at_ms: u64,
    closed_at_ms: u64,
    outcome: Result<(), &str>,
) -> TelemetrySpan {
    let mut span = TelemetrySpan::open(Uuid::new_v4(), "sawhorse", trace_id, started_at_ms);
    span.command = Some("complete".to_string());
    span.duration_ms = Some(closed_at_ms.saturating_sub(started_at_ms));
    match outcome {
        Ok(()) => span.status = SpanStatus::Ok,
        Err(diagnostic) => {
            span.status = SpanStatus::Error;
            span.first_diagnostic = Some(diagnostic.to_string());
        }
    }
    span
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn a_successful_completion_records_a_duration_and_no_diagnostic() {
        let span = completion_span(Uuid::new_v4(), 1_000, 1_250, Ok(()));

        assert_eq!(span.status, SpanStatus::Ok);
        assert_eq!(span.duration_ms, Some(250));
        assert_eq!(span.first_diagnostic, None);
        assert_eq!(span.binary, "sawhorse");
        assert_eq!(span.command.as_deref(), Some("complete"));
    }

    #[test]
    fn a_failed_completion_keeps_the_first_diagnostic() {
        let span = completion_span(Uuid::new_v4(), 5_000, 5_010, Err("provider timed out"));

        assert_eq!(span.status, SpanStatus::Error);
        assert_eq!(span.first_diagnostic.as_deref(), Some("provider timed out"));
        assert_eq!(span.duration_ms, Some(10));
    }

    #[test]
    fn a_clock_that_goes_backwards_yields_zero_rather_than_underflow() {
        // Two processes sharing one trace can disagree on wall clock, and a panic in an emitter would
        // fail the call it was only supposed to observe.
        let span = completion_span(Uuid::new_v4(), 9_000, 8_000, Ok(()));

        assert_eq!(span.duration_ms, Some(0));
    }

    #[test]
    fn an_absent_ambient_trace_id_still_yields_a_usable_one() {
        // The variable cannot be set here: this workspace denies `unsafe_code` and `set_var` is unsafe in
        // edition 2024. The unset path is the common one regardless - a call made by hand outside an
        // instrumented `outfeed gate` step.
        let trace_id = trace_id_from_env();

        assert_eq!(trace_id.get_version_num(), 4, "minted locally as v4");
    }
}
