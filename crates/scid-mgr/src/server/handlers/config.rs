use crate::server::{RequestMessage, ResponseMessage};

pub fn handle_set_threads(
    req: &RequestMessage,
    thread_pool: &mut rayon::ThreadPool,
    current_thread_count: &mut usize,
    max_system_threads: usize,
) -> ResponseMessage {
    let id = req.id;
    let threads = req
        .params
        .get("threads")
        .or_else(|| req.params.get("params").and_then(|p| p.get("threads")))
        .and_then(|v| v.as_u64())
        .unwrap_or(max_system_threads as u64) as usize;
    let clamped = threads.clamp(1, max_system_threads * 2);
    match rayon::ThreadPoolBuilder::new().num_threads(clamped).build() {
        Ok(new_pool) => {
            *thread_pool = new_pool;
            *current_thread_count = clamped;
            ResponseMessage {
                id,
                status: "ok".to_string(),
                data: Some(serde_json::json!({
                    "threads": *current_thread_count,
                    "max_threads": max_system_threads,
                })),
                error: None,
            }
        }
        Err(e) => ResponseMessage {
            id,
            status: "error".to_string(),
            data: None,
            error: Some(format!("Failed to configure thread pool: {}", e)),
        },
    }
}

pub fn handle_get_threads(
    req: &RequestMessage,
    current_thread_count: usize,
    max_system_threads: usize,
) -> ResponseMessage {
    ResponseMessage {
        id: req.id,
        status: "ok".to_string(),
        data: Some(serde_json::json!({
            "threads": current_thread_count,
            "max_threads": max_system_threads,
        })),
        error: None,
    }
}
