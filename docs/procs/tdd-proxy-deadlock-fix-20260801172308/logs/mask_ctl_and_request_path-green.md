---
title: "mask_ctl_and_request_path — Green Phase"
brief: "mask_ctl_and_request_path — Green: exit 0"
doc_type: proc
created: 2026-08-01T18:00:29+0800
case: "mask_ctl_and_request_path"
phase: green
---
Exit code: 0
Full output:
```
    Finished `test` profile [unoptimized + debuginfo] target(s) in 0.08s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 10 tests
test ui::tests::mask_api_key ... ok
test ui::tests::mask_auth_token ... ok
test proxy::tests::mask_ctl_line_no_key_passthrough ... ok
test proxy::tests::mask_request_path_preserves_non_secret ... ok
test proxy::tests::mask_request_path_masks_key_with_separators ... ok
test proxy::tests::mask_request_path_masks_query_key ... ok
test ui::tests::mask_secret ... ok
test proxy::tests::mask_ctl_line_masks_custom_token_api_key ... ok
test proxy::tests::mask_ctl_line_masks_sk_prefix_api_key ... ok
test ui::tests::no_mask_url ... ok

test result: ok. 10 passed; 0 failed; 0 ignored; 0 measured; 135 filtered out; finished in 0.00s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 19 filtered out; finished in 0.00s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 5 filtered out; finished in 0.00s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 4 filtered out; finished in 0.00s
```
