use scid_mgr::db::ScidDatabaseWrapper;
use scid_mgr::pgn_db::PgnDatabaseWrapper;
use scid_mgr::server::handlers::db::handle_query_games;
use scid_mgr::server::handlers::reference::handle_reference;
use scid_mgr::server::search_session::SearchSessionManager;
use scid_mgr::server::{DatabaseBackend, RequestMessage};
use std::path::Path;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;

#[test]
fn test_unified_reference_command_all_four_components() {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let mut session_mgr = SearchSessionManager::new();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let mut pos_index = None;
    let mut tree_index = None;

    let scid_path = Path::new("tests/fixtures/sample.si5");
    if !scid_path.exists() {
        return;
    }
    let scid_db = ScidDatabaseWrapper::open(scid_path).unwrap();
    let db_backend = Some(DatabaseBackend::Scid(scid_db));

    // 1. All 4 components enabled
    let req_all = RequestMessage {
        id: Some(1),
        command: "reference".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "include_tree": true,
            "include_continuations": true,
            "include_endgames": true,
            "include_games": true,
            "page": 0,
            "page_size": 5
        }),
    };

    let resp = handle_reference(
        &req_all,
        &db_backend,
        &mut pos_index,
        &mut tree_index,
        &mut session_mgr,
        &thread_pool,
        &cancel_flag,
    );

    assert_eq!(resp.status, "ok");
    let data = resp.data.expect("Response data should be present");

    assert!(data.get("search_id").is_some());
    let search_id = data["search_id"].as_str().unwrap().to_string();
    assert!(search_id.starts_with("ref_"));

    // Check Tree
    assert!(data.get("tree").is_some() && !data["tree"].is_null());
    let tree = &data["tree"];
    assert!(tree.get("moves").is_some());

    // Check Continuations
    assert!(data.get("continuations").is_some() && !data["continuations"].is_null());

    // Check Endgames
    assert!(data.get("endgames").is_some() && !data["endgames"].is_null());

    // Check Games
    assert!(data.get("games").is_some() && !data["games"].is_null());
    let games = data["games"].as_array().unwrap();
    assert!(!games.is_empty());
    assert!(games[0].get("matching_plies").is_some());

    // 2. Verify we can paginate subsequent pages using query_games with returned search_id
    let req_page1 = RequestMessage {
        id: Some(2),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": search_id,
            "owner": "reference",
            "page": 1,
            "page_size": 2
        }),
    };
    let resp_page1 = handle_query_games(&req_page1, &db_backend, &mut session_mgr, &thread_pool);
    assert_eq!(resp_page1.status, "ok");
    let data_page1 = resp_page1.data.unwrap();
    assert_eq!(data_page1["page"], 1);
    assert_eq!(data_page1["page_size"], 2);

    // 3. Selective flags: only Tree requested
    let req_tree_only = RequestMessage {
        id: Some(3),
        command: "reference".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "include_tree": true,
            "include_continuations": false,
            "include_endgames": false,
            "include_games": false
        }),
    };
    let resp_tree_only = handle_reference(
        &req_tree_only,
        &db_backend,
        &mut pos_index,
        &mut tree_index,
        &mut session_mgr,
        &thread_pool,
        &cancel_flag,
    );
    assert_eq!(resp_tree_only.status, "ok");
    let data_tree_only = resp_tree_only.data.unwrap();
    assert!(data_tree_only["tree"].is_object());
    assert!(data_tree_only["continuations"].is_null());
    assert!(data_tree_only["endgames"].is_null());
    assert!(data_tree_only["games"].is_null());

    // 4. Selective flags: only Endgames requested
    let req_endgames_only = RequestMessage {
        id: Some(4),
        command: "reference".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "include_tree": false,
            "include_continuations": false,
            "include_endgames": true,
            "include_games": false
        }),
    };
    let resp_endgames_only = handle_reference(
        &req_endgames_only,
        &db_backend,
        &mut pos_index,
        &mut tree_index,
        &mut session_mgr,
        &thread_pool,
        &cancel_flag,
    );
    assert_eq!(resp_endgames_only.status, "ok");
    let data_endgames_only = resp_endgames_only.data.unwrap();
    assert!(data_endgames_only["tree"].is_null());
    assert!(data_endgames_only["continuations"].is_null());
    assert!(data_endgames_only["endgames"].is_object());
    assert!(data_endgames_only["games"].is_null());
}

#[test]
fn test_unified_reference_pgn_database() {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let mut session_mgr = SearchSessionManager::new();
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let mut pos_index = None;
    let mut tree_index = None;

    let pgn_path = Path::new("tests/fixtures/sample.pgn");
    if !pgn_path.exists() {
        return;
    }
    let pgn_db = PgnDatabaseWrapper::open(pgn_path).unwrap();
    let db_backend = Some(DatabaseBackend::Pgn(pgn_db));

    let req = RequestMessage {
        id: Some(10),
        command: "reference".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1",
            "include_tree": true,
            "include_continuations": true,
            "include_games": true,
            "page": 0,
            "page_size": 10
        }),
    };

    let resp = handle_reference(
        &req,
        &db_backend,
        &mut pos_index,
        &mut tree_index,
        &mut session_mgr,
        &thread_pool,
        &cancel_flag,
    );

    assert_eq!(resp.status, "ok");
    let data = resp.data.unwrap();
    assert!(data["search_id"].as_str().unwrap().starts_with("ref_"));
    assert!(data["tree"].is_object());
    let moves = data["tree"]["moves"].as_array().unwrap();
    if !moves.is_empty() {
        let first_move = &moves[0];
        if let Some(fy) = first_move.get("first_year").and_then(|v| v.as_u64()) {
            assert!(
                (1800..=2030).contains(&fy),
                "first_year should be 4-digit calendar year, got {}",
                fy
            );
        }
        if let Some(ly) = first_move.get("last_year").and_then(|v| v.as_u64()) {
            assert!(
                (1800..=2030).contains(&ly),
                "last_year should be 4-digit calendar year, got {}",
                ly
            );
        }
    }
}

#[test]
fn test_unified_reference_with_filter_and_lru_eviction() {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let mut session_mgr = SearchSessionManager::with_capacity(3); // Capacity 3
    let cancel_flag = Arc::new(AtomicBool::new(false));
    let mut pos_index = None;
    let mut tree_index = None;

    let scid_path = Path::new("tests/fixtures/sample.si5");
    if !scid_path.exists() {
        return;
    }
    let scid_db = ScidDatabaseWrapper::open(scid_path).unwrap();
    let db_backend = Some(DatabaseBackend::Scid(scid_db));

    // 1. Filtered Reference Query
    let req_filtered = RequestMessage {
        id: Some(20),
        command: "reference".to_string(),
        params: serde_json::json!({
            "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
            "filter": {
                "player": "Kasparov"
            },
            "include_tree": true,
            "include_games": true,
            "page": 0,
            "page_size": 10
        }),
    };

    let resp_filt = handle_reference(
        &req_filtered,
        &db_backend,
        &mut pos_index,
        &mut tree_index,
        &mut session_mgr,
        &thread_pool,
        &cancel_flag,
    );

    assert_eq!(resp_filt.status, "ok");
    let data_filt = resp_filt.data.unwrap();
    let filt_search_id = data_filt["search_id"].as_str().unwrap().to_string();

    // 2. Execute 3 more reference position requests to trigger LRU eviction of the 1st
    let fens = [
        "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq d3 0 1",
        "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq c3 0 1",
        "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1",
    ];

    for fen in &fens {
        let req_pos = RequestMessage {
            id: Some(30),
            command: "reference".to_string(),
            params: serde_json::json!({
                "fen": fen,
                "include_tree": true,
                "include_games": true
            }),
        };
        let r = handle_reference(
            &req_pos,
            &db_backend,
            &mut pos_index,
            &mut tree_index,
            &mut session_mgr,
            &thread_pool,
            &cancel_flag,
        );
        assert_eq!(r.status, "ok");
    }

    // Reference pool capacity is 3 -> the 1st session (`filt_search_id`) was evicted!
    assert_eq!(session_mgr.reference_sessions.len(), 3);
    assert!(session_mgr.get_session(&filt_search_id).is_none());
}
