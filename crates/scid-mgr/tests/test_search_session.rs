use scid_mgr::db::ScidDatabaseWrapper;
use scid_mgr::pgn_db::PgnDatabaseWrapper;
use scid_mgr::server::handlers::db::handle_query_games;
use scid_mgr::server::handlers::search::{handle_cql_search, handle_search};
use scid_mgr::server::search_session::SearchSessionManager;
use scid_mgr::server::{DatabaseBackend, RequestMessage};
use std::path::Path;

#[test]
fn test_search_session_scid_and_pgn_pagination() {
    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let mut session_mgr = SearchSessionManager::new();
    let cancel_flag = std::sync::Arc::new(std::sync::atomic::AtomicBool::new(false));

    // 1. SCID Database Search Session Test
    let scid_path = Path::new("tests/fixtures/sample.si5");
    if scid_path.exists() {
        let scid_db = ScidDatabaseWrapper::open(scid_path).unwrap();
        let db_backend = Some(DatabaseBackend::Scid(scid_db));

        // A. Run initial search
        let search_req = RequestMessage {
            id: Some(1),
            command: "search".to_string(),
            params: serde_json::json!({
                "query": "queens >= 0"
            }),
        };

        let resp1 = handle_cql_search(
            &search_req,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
            &cancel_flag,
        );
        assert_eq!(resp1.status, "ok");
        let data1 = resp1.data.unwrap();
        let search_id = data1["search_id"].as_str().unwrap().to_string();
        let matched_count = data1["matched_count"].as_u64().unwrap() as usize;
        assert_eq!(data1["cached"], false);
        assert!(matched_count > 0);

        // B. Reusing identical search returns cached search_id
        let resp2 = handle_cql_search(
            &search_req,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
            &cancel_flag,
        );
        assert_eq!(resp2.status, "ok");
        let data2 = resp2.data.unwrap();
        assert_eq!(data2["search_id"].as_str().unwrap(), search_id);
        assert_eq!(data2["cached"], true);
        assert_eq!(data2["duration_ms"], 0);

        // C. Paginate through search session via query_games
        let list_req_page0 = RequestMessage {
            id: Some(2),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": search_id,
                "page": 0,
                "page_size": 2
            }),
        };

        let list_resp0 =
            handle_query_games(&list_req_page0, &db_backend, &mut session_mgr, &thread_pool);
        assert_eq!(list_resp0.status, "ok");
        let list_data0 = list_resp0.data.unwrap();
        assert_eq!(list_data0["page"], 0);
        assert_eq!(list_data0["page_size"], 2);
        assert_eq!(list_data0["total"], matched_count);
        assert_eq!(list_data0["search_id"], search_id);

        let games0 = list_data0["games"].as_array().unwrap();
        let page0_len = usize::min(2, matched_count);
        assert_eq!(games0.len(), page0_len);
        if page0_len > 0 {
            assert!(games0[0].get("matching_plies").is_some());
            assert!(games0[0].get("match_count").is_some());
        }

        // D. Sorting across pages by Elo descending
        let list_req_sort_elo = RequestMessage {
            id: Some(3),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": search_id,
                "page": 0,
                "page_size": 2,
                "sort_by": "white_elo",
                "sort_asc": false
            }),
        };
        let list_resp_sort = handle_query_games(
            &list_req_sort_elo,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
        );
        assert_eq!(list_resp_sort.status, "ok");
        let list_data_sort = list_resp_sort.data.unwrap();
        let games_sort = list_data_sort["games"].as_array().unwrap();
        if games_sort.len() >= 2 {
            let elo0 = games_sort[0]["white_elo"].as_u64().unwrap_or(0);
            let elo1 = games_sort[1]["white_elo"].as_u64().unwrap_or(0);
            assert!(elo0 >= elo1, "Expected Elo0 ({}) >= Elo1 ({})", elo0, elo1);
        }

        // D2. Sorting by White player name ascending
        let list_req_sort_white = RequestMessage {
            id: Some(4),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": search_id,
                "page": 0,
                "page_size": 10,
                "sort_by": "white",
                "sort_asc": true
            }),
        };
        let list_resp_white = handle_query_games(
            &list_req_sort_white,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
        );
        assert_eq!(list_resp_white.status, "ok");
        let list_data_white = list_resp_white.data.unwrap();
        let games_white = list_data_white["games"].as_array().unwrap();
        if games_white.len() >= 2 {
            let w0 = games_white[0]["white"].as_str().unwrap_or("");
            let w1 = games_white[1]["white"].as_str().unwrap_or("");
            assert!(w0 <= w1, "Expected White0 ({}) <= White1 ({})", w0, w1);
        }

        // E. Out-of-bounds page
        let list_req_oob = RequestMessage {
            id: Some(5),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": search_id,
                "page": 9999,
                "page_size": 2
            }),
        };
        let list_resp_oob =
            handle_query_games(&list_req_oob, &db_backend, &mut session_mgr, &thread_pool);
        assert_eq!(list_resp_oob.status, "ok");
        let list_data_oob = list_resp_oob.data.unwrap();
        assert_eq!(list_data_oob["games"].as_array().unwrap().len(), 0);
        assert_eq!(list_data_oob["total"], matched_count);

        // F. Invalid search_id
        let list_req_invalid = RequestMessage {
            id: Some(6),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": "non_existent_search",
                "page": 0,
                "page_size": 10
            }),
        };
        let list_resp_invalid = handle_query_games(
            &list_req_invalid,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
        );
        assert_eq!(list_resp_invalid.status, "error");
        assert!(list_resp_invalid.error.unwrap().contains("not found"));
        // G. Position search uniform search_id test
        let mut pos_index = None;
        let pos_search_req = RequestMessage {
            id: Some(7),
            command: "search_position".to_string(),
            params: serde_json::json!({
                "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1",
                "match_mode": "exact"
            }),
        };
        let pos_resp = scid_mgr::server::handlers::position::handle_search_position(
            &pos_search_req,
            &db_backend,
            &mut pos_index,
            &mut session_mgr,
            &thread_pool,
        );
        assert_eq!(pos_resp.status, "ok");
        let pos_data = pos_resp.data.unwrap();
        assert!(pos_data.get("search_id").is_some());
        let pos_search_id = pos_data["search_id"].as_str().unwrap();

        // Paginate and sort position search results
        let list_pos_req = RequestMessage {
            id: Some(8),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": pos_search_id,
                "page": 0,
                "page_size": 5,
                "sort_by": "date",
                "sort_asc": false
            }),
        };
        let list_pos_resp =
            handle_query_games(&list_pos_req, &db_backend, &mut session_mgr, &thread_pool);
        assert_eq!(list_pos_resp.status, "ok");

        // H. Material search uniform search_id test
        let mat_search_req = RequestMessage {
            id: Some(9),
            command: "search_material".to_string(),
            params: serde_json::json!({
                "white_queens": [1, 1],
                "black_queens": [1, 1]
            }),
        };
        let mat_resp = scid_mgr::server::handlers::position::handle_search_material(
            &mat_search_req,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
        );
        assert_eq!(mat_resp.status, "ok");
        let mat_data = mat_resp.data.unwrap();
        assert!(mat_data.get("search_id").is_some());
        let mat_search_id = mat_data["search_id"].as_str().unwrap();

        // Paginate and sort material search results
        let list_mat_req = RequestMessage {
            id: Some(10),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": mat_search_id,
                "page": 0,
                "page_size": 5,
                "sort_by": "white_elo",
                "sort_asc": false
            }),
        };
        let list_mat_resp =
            handle_query_games(&list_mat_req, &db_backend, &mut session_mgr, &thread_pool);
        assert_eq!(list_mat_resp.status, "ok");

        // I. Unified search handler test (Structured filter with headers + position)
        let unified_req = RequestMessage {
            id: Some(11),
            command: "search".to_string(),
            params: serde_json::json!({
                "white": "Kasparov",
                "fen": "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq e3 0 1"
            }),
        };
        let unified_resp = handle_search(
            &unified_req,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
            &cancel_flag,
        );
        assert_eq!(unified_resp.status, "ok");
        let unified_data = unified_resp.data.unwrap();
        assert!(unified_data.get("search_id").is_some());
        let unified_search_id = unified_data["search_id"].as_str().unwrap();

        // Paginate unified search results
        let list_unified_req = RequestMessage {
            id: Some(12),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": unified_search_id,
                "page": 0,
                "page_size": 10
            }),
        };
        let list_unified_resp = handle_query_games(
            &list_unified_req,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
        );
        assert_eq!(list_unified_resp.status, "ok");

        // J. Unified search handler test (Pure CQL string)
        let unified_cql_req = RequestMessage {
            id: Some(13),
            command: "search".to_string(),
            params: serde_json::json!({
                "query": "queens >= 1"
            }),
        };
        let unified_cql_resp = handle_search(
            &unified_cql_req,
            &db_backend,
            &mut session_mgr,
            &thread_pool,
            &cancel_flag,
        );
        assert_eq!(unified_cql_resp.status, "ok");
        let unified_cql_data = unified_cql_resp.data.unwrap();
        assert!(unified_cql_data.get("search_id").is_some());
    }

    // 2. PGN Database Search Session Test
    let pgn_path = Path::new("tests/fixtures/sample.pgn");
    if pgn_path.exists() {
        let pgn_db = PgnDatabaseWrapper::open(pgn_path).unwrap();
        let pgn_backend = Some(DatabaseBackend::Pgn(pgn_db));

        let search_req = RequestMessage {
            id: Some(10),
            command: "search".to_string(),
            params: serde_json::json!({
                "query": "rooks >= 0"
            }),
        };

        let pgn_resp1 = handle_cql_search(
            &search_req,
            &pgn_backend,
            &mut session_mgr,
            &thread_pool,
            &cancel_flag,
        );
        assert_eq!(pgn_resp1.status, "ok");
        let pgn_data1 = pgn_resp1.data.unwrap();
        let pgn_search_id = pgn_data1["search_id"].as_str().unwrap().to_string();
        let pgn_matches = pgn_data1["matched_count"].as_u64().unwrap() as usize;

        let pgn_list_req = RequestMessage {
            id: Some(11),
            command: "query_games".to_string(),
            params: serde_json::json!({
                "search_id": pgn_search_id,
                "page": 0,
                "page_size": 10,
                "sort_by": "white",
                "sort_asc": true
            }),
        };
        let pgn_list_resp =
            handle_query_games(&pgn_list_req, &pgn_backend, &mut session_mgr, &thread_pool);
        assert_eq!(pgn_list_resp.status, "ok");
        let pgn_list_data = pgn_list_resp.data.unwrap();
        assert_eq!(pgn_list_data["total"], pgn_matches);
    }
}

#[test]
fn test_search_session_manager_lru_and_ownership() {
    use scid_mgr::server::search_session::{
        PositionMatchMode, SearchSessionManager, SessionOwner, SessionQuery,
    };

    let mut mgr = SearchSessionManager::with_capacity(3); // Cap reference sessions at 3

    // 1. Create a MainTable session
    let main_id = mgr.create_session_with_metadata(
        "db1",
        "player:Carlsen",
        1000,
        vec![1u32],
        10,
        SessionOwner::Main,
        SessionQuery::HeaderSearch {
            filter: Box::new(scid_mgr::db::GameFilter {
                player: Some("Carlsen".to_string()),
                ..Default::default()
            }),
        },
    );

    assert!(main_id.starts_with("main_"));
    assert!(mgr.main_session.is_some());

    // 2. Create 4 reference sessions (exceeding capacity 3)
    let ref1 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.e4",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "rnbqkbnr/pppppppp/8/8/4P3/8/PPPP1PPP/RNBQKBNR b KQkq - 0 1".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );
    let ref2 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.d4",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "rnbqkbnr/pppppppp/8/8/3P4/8/PPP1PPPP/RNBQKBNR b KQkq - 0 1".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );
    let ref3 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.c4",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "rnbqkbnr/pppppppp/8/8/2P5/8/PP1PPPPP/RNBQKBNR b KQkq - 0 1".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    assert_eq!(mgr.reference_sessions.len(), 3);
    assert!(mgr.get_session(&ref1).is_some());
    assert!(mgr.get_session(&ref2).is_some());
    assert!(mgr.get_session(&ref3).is_some());

    // Creating 4th reference session should evict ref1 (oldest LRU)
    let ref4 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.Nf3",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "rnbqkbnr/pppppppp/8/8/8/5N2/PPPPPPPP/RNBQKB1R b KQkq - 1 1".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    assert_eq!(mgr.reference_sessions.len(), 3);
    assert!(mgr.get_session(&ref1).is_none()); // Evicted!
    assert!(mgr.get_session(&ref2).is_some());
    assert!(mgr.get_session(&ref3).is_some());
    assert!(mgr.get_session(&ref4).is_some());

    // Main session is completely untouched by reference LRU eviction!
    assert!(mgr.get_session(&main_id).is_some());
    assert_eq!(
        mgr.get_session_by_owner(&main_id, Some(SessionOwner::Main))
            .unwrap()
            .owner,
        SessionOwner::Main
    );

    // Mismatched owner lookup returns None
    assert!(mgr
        .get_session_by_owner(&main_id, Some(SessionOwner::Reference))
        .is_none());
    assert!(mgr
        .get_session_by_owner(&ref4, Some(SessionOwner::Main))
        .is_none());
}

#[test]
fn test_search_session_lru_touch_mru_promotion() {
    use scid_mgr::server::search_session::{
        PositionMatchMode, SearchSessionManager, SessionOwner, SessionQuery,
    };

    let mut mgr = SearchSessionManager::with_capacity(3);

    let ref1 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.e4",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen_1".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );
    let ref2 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.d4",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen_2".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );
    let ref3 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.c4",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen_3".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    // LRU state is [ref1, ref2, ref3]
    // Accessing ref1 moves it to MRU -> LRU becomes [ref2, ref3, ref1]
    let accessed = mgr.get_session_mut(&ref1);
    assert!(accessed.is_some());

    // Creating ref4 should now evict ref2 (the oldest unaccessed), NOT ref1!
    let ref4 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.Nf3",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen_4".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    assert_eq!(mgr.reference_sessions.len(), 3);
    assert!(
        mgr.get_session(&ref1).is_some(),
        "ref1 should have been kept because it was promoted to MRU"
    );
    assert!(
        mgr.get_session(&ref2).is_none(),
        "ref2 should have been evicted as the oldest LRU session"
    );
    assert!(mgr.get_session(&ref3).is_some());
    assert!(mgr.get_session(&ref4).is_some());

    // Access ref3 via find_cached -> moves ref3 to MRU: LRU becomes [ref1, ref4, ref3]
    let found = mgr.find_cached("db1", "pos:1.c4");
    assert!(found.is_some());

    // Creating ref5 should now evict ref1 (oldest in LRU)
    let ref5 = mgr.create_session_with_metadata(
        "db1",
        "pos:1.g3",
        1000,
        vec![],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen_5".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    assert_eq!(mgr.reference_sessions.len(), 3);
    assert!(
        mgr.get_session(&ref1).is_none(),
        "ref1 should now be evicted"
    );
    assert!(
        mgr.get_session(&ref3).is_some(),
        "ref3 was touched by find_cached so kept"
    );
    assert!(mgr.get_session(&ref4).is_some());
    assert!(mgr.get_session(&ref5).is_some());
}

#[test]
fn test_search_session_main_session_retention_under_flood() {
    use scid_mgr::server::search_session::{
        PositionMatchMode, SearchSessionManager, SessionOwner, SessionQuery,
    };

    let mut mgr = SearchSessionManager::with_capacity(5);

    // Create persistent main session
    let main_id = mgr.create_session_with_metadata(
        "db_main",
        "player:Kasparov",
        50000,
        vec![42u32],
        15,
        SessionOwner::Main,
        SessionQuery::HeaderSearch {
            filter: Box::new(scid_mgr::db::GameFilter {
                player: Some("Kasparov".to_string()),
                ..Default::default()
            }),
        },
    );

    // Generate 100 reference sessions simulating rapid board moves in Reference Explorer
    for i in 0..100 {
        let fen = format!("fen_pos_{}", i);
        let query = format!("pos:{}", i);
        mgr.create_session_with_metadata(
            "db_main",
            &query,
            50000,
            vec![],
            1,
            SessionOwner::Reference,
            SessionQuery::PurePosition {
                fen,
                match_mode: PositionMatchMode::Exact,
                max_ply: None,
            },
        );
    }

    // Capacity is strictly enforced at 5
    assert_eq!(mgr.reference_sessions.len(), 5);
    assert_eq!(mgr.reference_lru.len(), 5);

    // Main session is completely preserved and unaffected!
    let main_session = mgr
        .get_session(&main_id)
        .expect("Main session must survive reference flood");
    assert_eq!(main_session.owner, SessionOwner::Main);
    assert_eq!(main_session.matches.len(), 1);
    assert_eq!(main_session.matches[0], 42);
}

#[test]
fn test_search_session_main_session_replacement_and_query_cache() {
    use scid_mgr::server::search_session::{SearchSessionManager, SessionOwner, SessionQuery};

    let mut mgr = SearchSessionManager::new();

    let id1 = mgr.create_session_with_metadata(
        "db1",
        "eco:C50",
        1000,
        vec![1u32],
        10,
        SessionOwner::Main,
        SessionQuery::General {
            description: "eco:C50".to_string(),
        },
    );

    assert_eq!(
        mgr.find_cached("db1", "eco:C50")
            .map(|s| s.search_id.as_str()),
        Some(id1.as_str())
    );

    // Replacing main session with a new query
    let id2 = mgr.create_session_with_metadata(
        "db1",
        "eco:B90",
        1000,
        vec![2u32],
        12,
        SessionOwner::Main,
        SessionQuery::General {
            description: "eco:B90".to_string(),
        },
    );

    assert_ne!(id1, id2);
    // Old query is no longer cached
    assert!(mgr.find_cached("db1", "eco:C50").is_none());
    // New query is cached
    assert_eq!(
        mgr.find_cached("db1", "eco:B90")
            .map(|s| s.search_id.as_str()),
        Some(id2.as_str())
    );
    assert_eq!(mgr.main_session.as_ref().unwrap().search_id, id2);
}

#[test]
fn test_handle_query_games_ownership_and_error_handling() {
    use scid_mgr::server::search_session::{
        PositionMatchMode, SearchSessionManager, SessionOwner, SessionQuery,
    };

    let thread_pool = rayon::ThreadPoolBuilder::new()
        .num_threads(2)
        .build()
        .unwrap();
    let scid_path = Path::new("tests/fixtures/sample.si5");
    if !scid_path.exists() {
        return;
    }
    let scid_db = ScidDatabaseWrapper::open(scid_path).unwrap();
    let db_backend = Some(DatabaseBackend::Scid(scid_db));
    let mut mgr = SearchSessionManager::with_capacity(2);

    let main_id = mgr.create_session_with_metadata(
        "sample",
        "main_query",
        100,
        vec![0u32],
        5,
        SessionOwner::Main,
        SessionQuery::General {
            description: "main".to_string(),
        },
    );

    let ref_id1 = mgr.create_session_with_metadata(
        "sample",
        "ref_query_1",
        100,
        vec![0u32],
        5,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen1".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    // 1. Query main session with NO owner -> should succeed
    let req_no_owner = RequestMessage {
        id: Some(101),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": main_id,
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_no_owner, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "ok");
    assert_eq!(resp.data.unwrap()["total"], 1);

    // 2. Query main session with owner="main" -> should succeed
    let req_main_owner = RequestMessage {
        id: Some(102),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": main_id,
            "owner": "main",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_main_owner, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "ok");

    // 3. Query main session with target="main_table" -> should succeed
    let req_target_main = RequestMessage {
        id: Some(103),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": main_id,
            "target": "main_table",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_target_main, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "ok");

    // 4. Query main session with owner="reference" -> should fail with specific error
    let req_mismatched_owner = RequestMessage {
        id: Some(104),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": main_id,
            "owner": "reference",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_mismatched_owner, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "error");
    let err = resp.error.unwrap();
    assert!(
        err.contains("not found in reference explorer"),
        "Error was: {}",
        err
    );

    // 5. Query reference session with NO owner -> should succeed
    let req_ref_no_owner = RequestMessage {
        id: Some(105),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": ref_id1,
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_ref_no_owner, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "ok");

    // 6. Query reference session with owner="reference" -> should succeed
    let req_ref_owner = RequestMessage {
        id: Some(106),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": ref_id1,
            "owner": "reference",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_ref_owner, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "ok");

    // 7. Query reference session with owner="main" -> should fail with specific error
    let req_ref_as_main = RequestMessage {
        id: Some(107),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": ref_id1,
            "owner": "main",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_ref_as_main, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "error");
    let err = resp.error.unwrap();
    assert!(
        err.contains("not found for owner 'main'"),
        "Error was: {}",
        err
    );

    // 8. Evict ref_id1 by pushing 2 more reference sessions
    let _ref_id2 = mgr.create_session_with_metadata(
        "sample",
        "ref_query_2",
        100,
        vec![],
        1,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen2".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );
    let _ref_id3 = mgr.create_session_with_metadata(
        "sample",
        "ref_query_3",
        100,
        vec![],
        1,
        SessionOwner::Reference,
        SessionQuery::PurePosition {
            fen: "fen3".to_string(),
            match_mode: PositionMatchMode::Exact,
            max_ply: None,
        },
    );

    // Querying evicted ref_id1 with owner="reference" -> fails
    let req_evicted = RequestMessage {
        id: Some(108),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": ref_id1,
            "owner": "reference",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_evicted, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "error");
    let err = resp.error.unwrap();
    assert!(
        err.contains("not found in reference explorer"),
        "Error was: {}",
        err
    );

    // Querying completely invalid ID without owner -> fails with general expired message
    let req_non_existent = RequestMessage {
        id: Some(109),
        command: "query_games".to_string(),
        params: serde_json::json!({
            "search_id": "random_xyz_999",
            "page": 0,
            "page_size": 10
        }),
    };
    let resp = handle_query_games(&req_non_existent, &db_backend, &mut mgr, &thread_pool);
    assert_eq!(resp.status, "error");
    let err = resp.error.unwrap();
    assert!(err.contains("not found or expired"), "Error was: {}", err);
}

#[test]
fn test_session_query_and_metadata_serialization() {
    use scid_mgr::server::search_session::{PositionMatchMode, SessionOwner, SessionQuery};

    // SessionOwner serialization
    let owner_main = SessionOwner::Main;
    let json_main = serde_json::to_string(&owner_main).unwrap();
    assert_eq!(json_main, "\"main\"");
    let de_main: SessionOwner = serde_json::from_str(&json_main).unwrap();
    assert_eq!(de_main, SessionOwner::Main);

    let owner_ref = SessionOwner::Reference;
    let json_ref = serde_json::to_string(&owner_ref).unwrap();
    assert_eq!(json_ref, "\"reference\"");
    let de_ref: SessionOwner = serde_json::from_str(&json_ref).unwrap();
    assert_eq!(de_ref, SessionOwner::Reference);

    // PositionMatchMode serialization
    let mode = PositionMatchMode::Placement;
    let json_mode = serde_json::to_string(&mode).unwrap();
    assert_eq!(json_mode, "\"placement\"");
    let de_mode: PositionMatchMode = serde_json::from_str(&json_mode).unwrap();
    assert_eq!(de_mode, PositionMatchMode::Placement);

    // SessionQuery variants serialization
    let q_pure = SessionQuery::PurePosition {
        fen: "8/8/8/8/8/8/8/8 w - - 0 1".to_string(),
        match_mode: PositionMatchMode::Exact,
        max_ply: Some(20),
    };
    let json_pure = serde_json::to_string(&q_pure).unwrap();
    assert!(json_pure.contains("\"type\":\"pure_position\""));
    let de_pure: SessionQuery = serde_json::from_str(&json_pure).unwrap();
    assert_eq!(de_pure, q_pure);

    let q_cql = SessionQuery::CqlSearch {
        query: "cql(input) { [Q] == 2 }".to_string(),
    };
    let json_cql = serde_json::to_string(&q_cql).unwrap();
    assert!(json_cql.contains("\"type\":\"cql_search\""));
    let de_cql: SessionQuery = serde_json::from_str(&json_cql).unwrap();
    assert_eq!(de_cql, q_cql);
}
