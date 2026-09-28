# Generated lifecycle transitions

Source: `schemas/lifecycle-transitions.json`. This is an allow-list; guards are not executable in the scaffold.

Total transitions: 72.

| Entity | From | To | Owner | Guard | Durable event | Retry | Reconciliation | Crash recovery |
|---|---|---|---|---|---|---|---|---|
| Run | admitted | running | coordinator | graph_and_policy_revalidated | run.state.changed | same_run | none |  |
| Run | running | awaiting_approval | coordinator | material_effect_requires_user_decision_before_invocation | run.state.changed | resume_only_after_exact_approval | none |  |
| Run | running | paused | coordinator | pause_stops_new_admissions_and_tracks_inflight_effects | run.state.changed | resume_revalidates_current_authority | required_for_ambiguous_effects |  |
| Run | running | succeeded | coordinator | all_required_outputs_have_accepted_receipts_and_no_unknown_effects | run.state.changed | terminal | none |  |
| Run | running | failed | coordinator | failure_policy_exhausted_and_no_ambiguous_effects | run.state.changed | new_run_or_explicit_policy_retry | none |  |
| Run | running | cancelled | coordinator | cancel_prevents_new_admissions_and_no_effect_is_ambiguous | run.state.changed | terminal | required_before_cancel_if_effect_is_ambiguous |  |
| Run | running | unknown_effect | coordinator | external_effect_outcome_cannot_be_established | run.state.changed | never_automatically | required |  |
| Run | awaiting_approval | running | user_and_coordinator | exact_action_approved_and_current_deny_rules_rechecked | run.state.changed | resume_same_run | none |  |
| Run | awaiting_approval | cancelled | user_and_coordinator | user_denies_or_cancels_before_effect_invocation | run.state.changed | terminal | none |  |
| Run | paused | running | coordinator | resume_revalidates_inputs_grants_and_leases | run.state.changed | resume_same_run | required_for_ambiguous_effects |  |
| Run | paused | cancelled | coordinator | cancel_prevents_new_admissions_and_ambiguous_effects_are_reconciled | run.state.changed | terminal | required_for_ambiguous_effects |  |
| Run | unknown_effect | running | coordinator | every_unknown_effect_reconciled_and_remaining_work_revalidated | run.state.changed | new_attempt_only_if_reconciliation_proves_no_effect | required |  |
| Run | unknown_effect | succeeded | coordinator | reconciled_effects_and_all_required_outputs_have_accepted_receipts | run.state.changed | terminal | required |  |
| Run | unknown_effect | failed | coordinator | reconciled_outcome_satisfies_failure_policy | run.state.changed | new_run_or_explicit_policy_retry | required |  |
| Run | unknown_effect | cancelled | coordinator | all_ambiguous_effects_reconciled_before_cancellation | run.state.changed | terminal | required |  |
| Run | admitted | cancelled | coordinator | user_cancel_before_any_attempt_or_effect_exists | run.state.changed | terminal | none |  |
| GraphRevision | draft | validated | coordinator | deterministic_compiler_accepts_graph_and_declared_contracts | graph.revision.state.changed | same_revision_until_admission | none |  |
| GraphRevision | draft | rejected | coordinator | deterministic_compiler_rejects_graph | graph.revision.state.changed | new_revision_required_for_changes | none |  |
| GraphRevision | validated | admitted | coordinator | coordinator_commits_scope_grants_budget_and_revision | graph.revision.state.changed | same_revision_only_if_not_admitted | none |  |
| GraphRevision | validated | rejected | coordinator | admission_precondition_no_longer_holds | graph.revision.state.changed | new_revision_required_for_changes | none |  |
| GraphRevision | admitted | superseded | coordinator | replacement_revision_is_admitted_without_rewriting_history | graph.revision.state.changed | new_revision | none |  |
| Task | queued | ready | coordinator | dependencies_accepted_and_required_resources_and_grants_are_available | task.state.changed | same_task | none |  |
| Task | queued | skipped | coordinator | completion_policy_allows_optional_skip_and_required_outputs_remain_explicit | task.state.changed | terminal_for_revision | none |  |
| Task | ready | running | coordinator | attempt_created_and_current_fenced_lease_admitted | task.state.changed | new_attempt_on_retry | none |  |
| Task | ready | cancelled | coordinator | cancel_prevents_dispatch | task.state.changed | terminal_for_revision | none |  |
| Task | running | verifying | coordinator | immutable_result_and_artifact_digest_recorded | task.state.changed | new_attempt_on_retry | none |  |
| Task | running | awaiting_approval | coordinator | material_effect_blocked_before_invocation | task.state.changed | resume_after_exact_approval | none |  |
| Task | running | paused | coordinator | pause_stops_new_dispatch_and_tracks_inflight_effects | task.state.changed | revalidate_before_resume | required_for_ambiguous_effects |  |
| Task | running | failed | coordinator | attempt_failed_and_no_effect_is_ambiguous | task.state.changed | new_attempt_only_under_bounded_retry_policy | none |  |
| Task | running | cancelled | coordinator | cancel_stops_work_and_no_effect_is_ambiguous | task.state.changed | terminal_for_revision | required_for_ambiguous_effects |  |
| Task | running | unknown_effect | coordinator | external_effect_outcome_is_ambiguous | task.state.changed | never_automatically | required |  |
| Task | verifying | succeeded | coordinator | unique_attempt_verification_receipt_accepts_exact_artifact_digest_and_task_event_links_same_attempt_and_receipt | task.state.changed | terminal_for_revision | none |  |
| Task | verifying | failed | coordinator | independent_verification_rejects_artifact_or_policy_exhausted | task.state.changed | new_attempt_only_under_bounded_retry_policy | none |  |
| Task | verifying | unknown_effect | coordinator | verification_cannot_establish_external_effect_outcome | task.state.changed | never_automatically | required |  |
| Task | awaiting_approval | running | user_and_coordinator | exact_action_approved_and_current_authority_rechecked | task.state.changed | resume_same_attempt_only_if_still_live | none |  |
| Task | awaiting_approval | cancelled | user_and_coordinator | user_denies_or_cancels_before_effect_invocation | task.state.changed | terminal_for_revision | none |  |
| Task | paused | ready | coordinator | dependencies_grants_and_inputs_revalidated_before_new_attempt | task.state.changed | new_attempt_if_prior_attempt_ended | required_for_ambiguous_effects |  |
| Task | paused | cancelled | coordinator | cancel_prevents_new_dispatch_and_ambiguous_effects_are_reconciled | task.state.changed | terminal_for_revision | required_for_ambiguous_effects |  |
| Task | failed | ready | coordinator | bounded_retry_policy_allows_new_attempt_and_inputs_remain_valid | task.state.changed | creates_new_attempt | none |  |
| Task | unknown_effect | succeeded | coordinator | reconciliation_proves_effect_and_exact_output_is_accepted | task.state.changed | terminal_for_revision | required |  |
| Task | unknown_effect | failed | coordinator | reconciliation_establishes_outcome_and_failure_policy | task.state.changed | new_attempt_only_if_no_effect_and_policy_allows | required |  |
| Task | unknown_effect | ready | coordinator | reconciliation_proves_no_effect_and_retry_policy_allows_new_attempt | task.state.changed | creates_new_attempt | required |  |
| Task | queued | cancelled | coordinator | run_cancelled_before_task_dispatch | task.state.changed | terminal_for_revision | none |  |
| Attempt | created | dispatched | coordinator | coordinator_records_current_fenced_lease_before_dispatch | attempt.state.changed | new_attempt_after_terminal_failure | none |  |
| Attempt | created | cancelled | coordinator | cancel_prevents_dispatch_before_worker_or_effect_invocation | attempt.state.changed | new_attempt_only_if_task_policy_allows | none |  |
| Attempt | dispatched | running | coordinator | worker_acknowledges_matching_live_lease | attempt.state.changed | new_attempt_if_lease_expires_before_invocation | none |  |
| Attempt | dispatched | failed | coordinator | dispatch_proven_not_invoked_and_lease_ended | attempt.state.changed | new_attempt_only_under_bounded_retry_policy | none |  |
| Attempt | dispatched | cancelled | coordinator | cancel_prevents_invocation | attempt.state.changed | terminal_for_attempt | none |  |
| Attempt | dispatched | unknown_effect | coordinator | invocation_may_have_occurred_but_result_is_missing | attempt.state.changed | never_automatically | required |  |
| Attempt | running | awaiting_approval | coordinator | material_effect_is_blocked_before_invocation | attempt.state.changed | resume_after_exact_approval | none |  |
| Attempt | running | paused | coordinator | pause_tracks_worker_and_any_inflight_effect | attempt.state.changed | resume_only_after_authority_recheck | required_for_ambiguous_effects |  |
| Attempt | running | verifying | coordinator | result_and_exact_artifact_digest_are_recorded | attempt.state.changed | new_attempt_on_retry | none |  |
| Attempt | running | failed | coordinator | attempt_failed_and_external_effect_is_known | attempt.state.changed | new_attempt_only_under_bounded_retry_policy | none |  |
| Attempt | running | cancelled | coordinator | cancelled_and_no_external_effect_is_ambiguous | attempt.state.changed | terminal_for_attempt | required_for_ambiguous_effects |  |
| Attempt | running | unknown_effect | coordinator | external_effect_outcome_cannot_be_established | attempt.state.changed | never_automatically | required |  |
| Attempt | awaiting_approval | running | user_and_coordinator | exact_action_approved_and_current_authority_rechecked | attempt.state.changed | resume_same_attempt_only_if_still_live | none |  |
| Attempt | awaiting_approval | cancelled | user_and_coordinator | user_denies_or_cancels_before_effect_invocation | attempt.state.changed | terminal_for_attempt | none |  |
| Attempt | paused | running | coordinator | same_attempt_is_live_and_current_authority_revalidated | attempt.state.changed | resume_same_attempt | required_for_ambiguous_effects |  |
| Attempt | paused | cancelled | coordinator | cancel_prevents_further_work_and_ambiguous_effects_are_reconciled | attempt.state.changed | terminal_for_attempt | required_for_ambiguous_effects |  |
| Attempt | verifying | succeeded | coordinator | independent_receipt_accepts_exact_artifact_digest | attempt.state.changed | terminal_for_attempt | none |  |
| Attempt | verifying | failed | coordinator | independent_verification_rejects_or_failure_policy_exhausted | attempt.state.changed | new_attempt_only_under_bounded_retry_policy | none |  |
| Attempt | verifying | unknown_effect | coordinator | verification_cannot_establish_external_effect_outcome | attempt.state.changed | never_automatically | required |  |
| Attempt | unknown_effect | reconciled | coordinator | independent_evidence_establishes_effect_outcome | attempt.state.changed | terminal_for_attempt | required |  |
| Effect | recorded | authorized | coordinator | exact_action_target_artifact_grant_and_policy_approval_validated_against_current_deny_rules | effect.state.changed | same_intent_until_invocation | none |  |
| Effect | recorded | cancelled | coordinator | cancel_or_denial_prevents_invocation | effect.state.changed | terminal_for_intent | none |  |
| Effect | authorized | invoking | coordinator | current_fenced_lease_rechecked_and_invoking_event_durably_committed_before_effect_call | effect.state.changed | no_replay_after_invocation_may_begin | none | committed_invoking_without_terminal_receipt_becomes_unknown_effect_without_replay |
| Effect | authorized | cancelled | coordinator | cancel_prevents_invocation_before_effect_boundary | effect.state.changed | terminal_for_intent | none |  |
| Effect | invoking | applied | coordinator | matching_effect_receipt_confirms_exact_intent_and_action_digest | effect.state.changed | terminal_for_intent | none |  |
| Effect | invoking | failed | coordinator | independent_evidence_proves_no_effect_was_applied | effect.state.changed | new_intent_only_after_current_authority_and_policy_retry_check | evidence_required |  |
| Effect | invoking | unknown_effect | coordinator | invocation_may_have_occurred_but_outcome_is_not_established | effect.state.changed | never_replay_this_intent | required |  |
| Effect | unknown_effect | reconciled_applied | coordinator | independent_evidence_source_proves_exact_effect_was_applied | effect.reconciled | terminal_for_intent | required |  |
| Effect | unknown_effect | reconciled_not_applied | coordinator | independent_evidence_source_proves_effect_was_not_applied | effect.reconciled | new_intent_only_after_explicit_current_authorization | required |  |
