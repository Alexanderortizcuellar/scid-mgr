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

        let resp1 = handle_cql_search(&search_req, &db_backend, &mut session_mgr, &thread_pool);
        assert_eq!(resp1.status, "ok");
        let data1 = resp1.data.unwrap();
        let search_id = data1["search_id"].as_str().unwrap().to_string();
        let matched_count = data1["matched_count"].as_u64().unwrap() as usize;
        assert_eq!(data1["cached"], false);
        assert!(matched_count > 0);

        // B. Reusing identical search returns cached search_id
        let resp2 = handle_cql_search(&search_req, &db_backend, &mut session_mgr, &thread_pool);
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
        let unified_resp = handle_search(&unified_req, &db_backend, &mut session_mgr, &thread_pool);
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
        let list_unified_resp =
            handle_query_games(&list_unified_req, &db_backend, &mut session_mgr, &thread_pool);
        assert_eq!(list_unified_resp.status, "ok");

        // J. Unified search handler test (Pure CQL string)
        let unified_cql_req = RequestMessage {
            id: Some(13),
            command: "search".to_string(),
            params: serde_json::json!({
                "query": "queens >= 1"
            }),
        };
        let unified_cql_resp = handle_search(&unified_cql_req, &db_backend, &mut session_mgr, &thread_pool);
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

        let pgn_resp1 =
            handle_cql_search(&search_req, &pgn_backend, &mut session_mgr, &thread_pool);
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
