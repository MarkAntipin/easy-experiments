use actix_web::web;
use actix_web::HttpResponse;
use chrono::Utc;

use crate::errors::CustomError;
use crate::models::{
    AuthenticatedApiKey, MetricEvent, TrackRequest, TrackResponse,
};
use crate::services::metric_sink::{MetricSink, RecordOutcome};
use crate::validation::ValidatedJson;

pub async fn track(
    api_key: web::ReqData<AuthenticatedApiKey>,
    sink: web::Data<dyn MetricSink>,
    payload: ValidatedJson<TrackRequest>,
) -> Result<HttpResponse, CustomError> {
    let now_ms = Utc::now().timestamp_millis();
    let request = payload.into_inner();

    let mut accepted: usize = 0;
    let mut deduped: usize = 0;

    for event in request.events {
        let ts_ms = event.ts.unwrap_or(now_ms);
        let ev = MetricEvent::new(
            ts_ms,
            api_key.company_id.clone(),
            event.entity_id,
            event.metric_name,
            event.value,
        );
        match sink.record(ev, event.idempotency_key.as_deref()) {
            RecordOutcome::Accepted => accepted += 1,
            RecordOutcome::Deduped => deduped += 1,
            // Dropped events are counted on the sink for ops visibility but
            // not surfaced per-request — clients shouldn't retry on a 200.
            RecordOutcome::Dropped => {}
        }
    }

    Ok(HttpResponse::Ok().json(TrackResponse { accepted, deduped }))
}
