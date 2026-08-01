---
title: "Regression Gate"
brief: "Regression gate: exit 0"
doc_type: proc
created: 2026-08-01T17:43:44Z
step: regression
---
## Regression Gate

全部 23 个 test case 已通过各自的 Red-Green-Refactor 门槛，本步执行最终全量回归门禁：`cargo test` 全量套件必须 exit 0。

### Command

```
cargo test
```

（首轮以默认 `cargo test` 执行时输出被 rtk 压缩为一行摘要，且 rtk tee 目录中无本次运行的完整输出；按任务指示以 `rtk proxy cargo test` 直连重跑捕获完整未压缩输出。两次运行均 exit 0，均报告 193 passed。以下为 rtk proxy 直连运行的原始完整输出。）

### Exit Code

```
0
```

### Output

```
   Compiling ring v0.17.14
   Compiling rustls v0.23.41
   Compiling rustls-webpki v0.103.13
   Compiling tokio-rustls v0.26.4
   Compiling hyper-rustls v0.27.9
   Compiling reqwest v0.12.28
   Compiling cct v0.5.0 (/Users/zhengjiaye/projects/llm_app/cc_starter/.claude/worktrees/exec-tdd-proxy-deadlock-fix-20260801172308-20260801173605)
    Finished `test` profile [unoptimized + debuginfo] target(s) in 7.98s
     Running unittests src/lib.rs (target/debug/deps/cct-c695630c4374c597)

running 147 tests
test app::tests::field_labels_returns_backend_specific_labels ... ok
test app::tests::form_state_field_navigation ... ok
test app::tests::form_state_six_fields ... ok
test app::tests::codex_form_does_not_swap_api_key_and_model ... ok
test app::tests::app_mode_transitions ... ok
test app::tests::filtered_indices_returns_correct_backend_subset ... ok
test app::tests::codex_form_field_mapping_matches_labels ... ok
test app::tests::claude_form_field_mapping_matches_labels ... ok
test app::tests::kimi_form_empty_context_maps_to_none ... ok
test app::tests::next_prev_navigate_within_filtered_backend ... ok
test app::tests::switch_backend_resets_selected_to_first_matching ... ok
test app::tests::to_new_profile_passes_auth_type ... ok
test app::tests::from_profile_codex_prefills_fields ... ok
test app::tests::from_profile_kimi_prefills_fields ... ok
test app::tests::from_profile_preserves_auth_type_token ... ok
test app::tests::from_profile_claude_prefills_fields ... ok
test app::tests::kimi_form_field_mapping_matches_labels ... ok
test cli::tests::pick_profile_selects_valid ... ok
test cli::tests::pick_profile_shows_kimi_tag ... ok
test cli::tests::pick_profile_rejects_out_of_range ... ok
test cli::tests::pick_profile_empty_list_errors ... ok
test cli::tests::pick_profile_rejects_invalid_input ... ok
test cli::tests::resolve_backend_parses_known_backends ... ok
test cli::tests::cli_add_sets_claude_backend_and_no_full_auto ... ok
test config::tests::append_codex_profile_generates_openai_env ... ok
test cli::tests::cli_add_with_kimi_backend ... ok
test cli::tests::cli_run_add_rejects_duplicate ... ok
test config::tests::append_codex_profile_with_subscription_auth ... ok
test config::tests::append_kimi_profile_minimal_no_env_section ... ok
test config::tests::append_kimi_profile_writes_minimal_env ... ok
test config::tests::backend_enum_deserialization ... ok
test config::tests::default_config_contains_default_kimi ... ok
test config::tests::default_config_is_valid_toml ... ok
test config::tests::default_max_context_size_detects_k3 ... ok
test config::tests::append_minimal_no_env_section ... ok
test config::tests::append_minimal_profile ... ok
test config::tests::append_preserves_existing ... ok
test config::tests::parse_full_profile ... ok
test config::tests::parse_minimal_profile ... ok
test config::tests::append_profile_base_url_only ... ok
test config::tests::profile_with_base_url_roundtrips ... ok
test config::tests::resolve_max_context_size_maps_labels ... ok
test config::tests::append_profile_generates_env_section ... ok
test config::tests::append_profile_roundtrips ... ok
test config::tests::append_profile_with_auth_token ... ok
test config::tests::ensure_kimi_profile_appends_default ... ok
test config::tests::find_profile_by_name_not_found ... ok
test config::tests::find_profile_by_name_returns_profile ... ok
test config::tests::profile_name_exists_case_insensitive ... ok
test config::tests::toggle_auth_type_api_key_to_token ... ok
test config::tests::toggle_auth_type_not_found ... ok
test config::tests::toggle_auth_type_token_to_api_key ... ok
test config::tests::toggle_codex_auth_type_insert ... ok
test config::tests::toggle_codex_auth_type_not_found ... ok
test config::tests::toggle_codex_auth_type_remove ... ok
test config::tests::toggle_full_auto_flip ... ok
test config::tests::toggle_full_auto_insert ... ok
test config::tests::toggle_full_auto_not_found ... ok
test config::tests::toggle_kimi_max_context_size_from_k3_default ... ok
test config::tests::toggle_kimi_max_context_size_from_other_default ... ok
test config::tests::toggle_kimi_max_context_size_not_found ... ok
test config::tests::toggle_skip_permissions_flip ... ok
test config::tests::toggle_skip_permissions_insert ... ok
test config::tests::validate_codex_auth_type_subscription_on_claude_rejected ... ok
test config::tests::validate_profiles_rejects_claude_full_auto ... ok
test config::tests::validate_profiles_rejects_codex_skip_permissions ... ok
test config::tests::validate_profiles_rejects_kimi_auth_type ... ok
test config::tests::validate_profiles_rejects_kimi_full_auto ... ok
test config::tests::toggle_skip_permissions_not_found ... ok
test config::tests::validate_profiles_rejects_kimi_skip_permissions ... ok
test launch::tests::build_args_continue_only ... ok
test launch::tests::build_args_empty ... ok
test launch::tests::build_args_continue_with_flags ... ok
test launch::tests::build_args_full ... ok
test launch::tests::build_args_model_only ... ok
test launch::tests::build_args_with_continue_false ... ok
test launch::tests::build_codex_args_empty ... ok
test launch::tests::build_codex_args_extra_only ... ok
test launch::tests::build_codex_args_full_auto_and_extra ... ok
test launch::tests::build_codex_args_full_auto_only ... ok
test launch::tests::build_codex_proxy_config_args_different_model_and_port ... ok
test launch::tests::build_codex_proxy_config_args_includes_model_and_port ... ok
test launch::tests::build_codex_subscription_args_empty ... ok
test launch::tests::build_codex_subscription_args_with_full_auto ... ok
test launch::tests::build_codex_subscription_args_with_model ... ok
test launch::tests::build_kimi_args_model ... ok
test launch::tests::build_kimi_args_model_and_extra ... ok
test launch::tests::build_kimi_args_model_from_env_fallback ... ok
test launch::tests::build_kimi_args_no_model_extra_only ... ok
test config::tests::update_profile_missing_original_errors ... ok
test launch::tests::build_launch_command_dispatches_kimi ... ok
test config::tests::update_profile_preserves_extra_args ... ok
test config::tests::update_profile_preserves_unknown_env_keys ... ok
test launch::tests::check_claude_installed_found ... ok
test launch::tests::check_claude_installed_not_found ... ok
test config::tests::update_profile_renames_in_place ... ok
test config::tests::update_profile_switches_claude_to_kimi_cleanly ... ok
test config::tests::update_profile_with_auth_token ... ok
test launch::tests::claude_default_env_is_injected ... ok
test launch::tests::normalize_kimi_base_url_cases ... ok
test proxy::tests::check_proxy_running_false_when_socket_absent ... ok
test launch::tests::generate_kimi_config_explicit_max_context_size_wins ... ok
test proxy::tests::check_proxy_running_true_when_daemon_responds ... ok
test proxy::tests::control_command_parse_status ... ok
test proxy::tests::control_command_parse_switch ... ok
test proxy::tests::control_response_serialize_err ... ok
test proxy::tests::control_response_serialize_ok ... ok
test proxy::tests::mask_ctl_line_masks_custom_token_api_key ... ok
test proxy::tests::mask_ctl_line_masks_sk_prefix_api_key ... ok
test launch::tests::generate_kimi_config_k3_writes_effort_and_1m ... ok
test proxy::tests::mask_ctl_line_no_key_passthrough ... ok
test proxy::tests::mask_request_path_masks_key_with_separators ... ok
test proxy::tests::mask_request_path_masks_query_key ... ok
test proxy::tests::mask_request_path_preserves_non_secret ... ok
test proxy::tests::proxy_port_default ... ok
test proxy::tests::proxy_port_from_env ... ok
test proxy::tests::proxy_socket_path_ends_with_proxy_sock ... ok
test proxy::tests::proxy_socket_path_override ... ok
test proxy::tests::shutdown_proxy_ok_when_daemon_responds ... ok
test launch::tests::generate_kimi_config_no_model_skips_models_table ... ok
test launch::tests::generate_kimi_config_preserves_existing_tables ... ok
test ui::tests::codex_confirm_shows_codex_labels_with_correct_values ... ok
test ui::tests::codex_detail_shows_subscription_auth ... ok
test ui::tests::codex_input_form_shows_codex_labels ... ok
test ui::tests::codex_subscription_profile_has_gray_style ... ok
test ui::tests::detail_does_not_show_auth_for_api_key ... ok
test ui::tests::detail_panel_shows_full_auto_for_codex_profile ... ok
test ui::tests::detail_shows_auth_type_token ... ok
test ui::tests::kimi_detail_shows_max_context_size ... ok
test ui::tests::kimi_footer_shows_space_context_hint ... ok
test ui::tests::mask_api_key ... ok
test ui::tests::mask_auth_token ... ok
test ui::tests::mask_secret ... ok
test ui::tests::no_mask_url ... ok
test ui::tests::skip_permissions_profile_has_red_style ... ok
test ui::tests::tab_bar_renders_with_active_highlight ... ok
test ui::tests::ui_confirmation_shows_five_fields ... ok
test ui::tests::ui_footer_shows_add_hint ... ok
test ui::tests::ui_form_title_and_confirmation_reflect_edit_mode ... ok
test ui::tests::ui_renders_add_form ... ok
test launch::tests::generate_kimi_config_regeneration_removes_effort_keys ... ok
test launch::tests::generate_kimi_config_writes_provider_and_model ... ok
test launch::tests::kimi_config_path_honors_override ... ok
test proxy::tests::tcp_port_owner_fallback_when_lsof_missing ... ok
test proxy::tests::tcp_port_owner_reports_pid_when_lsof_available ... ok
test proxy::tests::check_proxy_running_false_when_socket_silent ... ok
test proxy::tests::shutdown_proxy_errs_on_unresponsive_socket ... ok

test result: ok. 147 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.03s

     Running unittests src/main.rs (target/debug/deps/cct-a415d99fd63277d3)

running 21 tests
test tests::clap_routing_add_subcommand ... ok
test tests::clap_routing_edit_subcommand ... ok
test tests::clap_routing_add_with_auth_type ... ok
test tests::clap_routing_add_with_backend ... ok
test tests::clap_routing_env_no_args_lists_profiles ... ok
test tests::clap_routing_env_with_command ... ok
test tests::clap_routing_env_without_double_dash ... ok
test tests::clap_routing_env_empty_command ... ok
test tests::clap_routing_no_subcommand ... ok
test tests::clap_routing_proxy_start ... ok
test tests::clap_routing_proxy_stop ... ok
test tests::clap_routing_run_without_name ... ok
test tests::clap_routing_run_with_name ... ok
test tests::readme_documents_inline_edit_keybinding ... ok
test tests::duplicate_always_appends_copy_suffix ... ok
test tests::edit_mode_validates_duplicate_rename_and_keeps_unchanged_name ... ok
test tests::e_key_enters_prefilled_edit_form ... ok
test tests::d_key_enters_duplicate_form_with_copy_suffix ... ok
test tests::stop_proxy_errs_on_unresponsive_socket ... ok
test tests::stop_proxy_ok_when_socket_absent ... ok
test tests::edit_mode_save_reloads_and_reselects_updated_profile ... ok

test result: ok. 21 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.02s

     Running tests/integration.rs (target/debug/deps/integration-7dbc2c9fd2903748)

running 5 tests
test build_args_empty_profile ... ok
test build_args_ordering ... ok
test config_round_trip ... ok
test exec_full_profile_fake_binary ... ok
test exec_env_injection ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.79s

     Running tests/launch_proxy_contract.rs (target/debug/deps/launch_proxy_contract-11aa5be2c88374b7)

running 5 tests
test port_occupied_bails_with_diagnosis ... ok
test probe_exhaustion_reports_error ... ok
test spawns_fake_when_none_running ... ok
test zombie_socket_triggers_restart ... ok
test reuses_live_proxy ... ok

test result: ok. 5 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 2.73s

     Running tests/live.rs (target/debug/deps/live-080beb1530291df0)

running 4 tests
test arg_passthrough_via_fake ... ok
test release_binary_builds ... ok
test binary_spawns_cleanly ... ok
test real_config_loads ... ok

test result: ok. 4 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

     Running tests/proxy_contract.rs (target/debug/deps/proxy_contract-37538df7d2e1326e)

running 11 tests
test smoke_stub_receives_request ... ok
test concurrent_control_and_http ... ok
test shutdown_removes_socket_file ... ok
test log_masks_api_key_upstream_error ... ok
test port_occupied_reports_error_keeps_occupant ... ok
test log_masks_api_key ... ok
test double_start_race_one_wins ... ok
test launch_path_writes_no_codex_config ... ok
test zombie_recovery_restarts_proxy ... ok
test stub_forwarding_with_bearer ... ok
test stop_times_out_on_unresponsive_socket ... ok

test result: ok. 11 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 6.22s

   Doc-tests cct

running 0 tests

test result: ok. 0 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished in 0.00s

```

汇总：7 个套件共 **193 passed; 0 failed**，exit code **0**。先前已知瞬态（`launch_path_writes_no_codex_config` / `shutdown_proxy_ok_when_daemon_responds` os error 57、doctest "extern location for reqwest does not exist"）本次未出现。**Regression Gate: PASS。**
