use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use tower_lsp::Client;
use tower_lsp::lsp_types::notification::Progress;
use tower_lsp::lsp_types::request::WorkDoneProgressCreate;
use tower_lsp::lsp_types::{
    ProgressParams, ProgressParamsValue, ProgressToken, WorkDoneProgress, WorkDoneProgressBegin, WorkDoneProgressCreateParams,
    WorkDoneProgressEnd,
};

pub struct ProgressReporter {
    enabled: AtomicBool,
    token_counter: AtomicU64,
}

impl ProgressReporter {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            enabled: AtomicBool::new(false),
            token_counter: AtomicU64::new(0),
        }
    }

    pub fn set_enabled(&self, enabled: bool) {
        self.enabled.store(enabled, Ordering::Relaxed);
    }

    pub async fn run<R, F>(&self, client: &Client, title: &str, message: Option<String>, future: F) -> R
    where
        F: core::future::Future<Output = R>,
    {
        if !self.enabled.load(Ordering::Relaxed) {
            return future.await;
        }

        let token = ProgressToken::String(format!("mcshader-{}", self.token_counter.fetch_add(1, Ordering::Relaxed)));

        let _ = client
            .send_request::<WorkDoneProgressCreate>(WorkDoneProgressCreateParams { token: token.clone() })
            .await;

        client
            .send_notification::<Progress>(ProgressParams {
                token: token.clone(),
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::Begin(WorkDoneProgressBegin {
                    title: title.to_owned(),
                    cancellable: Some(false),
                    message,
                    percentage: None,
                })),
            })
            .await;

        let result = future.await;

        client
            .send_notification::<Progress>(ProgressParams {
                token,
                value: ProgressParamsValue::WorkDone(WorkDoneProgress::End(WorkDoneProgressEnd {
                    message: Some("Done".to_owned()),
                })),
            })
            .await;

        result
    }
}
