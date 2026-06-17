    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_job_result_auto_advances_next_node() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let first_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "first".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("first job should be created: {error}"));
        let second_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "second".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("second job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "auto-advance".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "first".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(first_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                        super::WorkflowNodeSpec {
                            key: "second".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(second_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "first".to_owned(),
                        to: "second".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("node should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued node should exist"));
        assert_eq!(materialized.queue_item.status, "done");
        assert_eq!(materialized.queue_item.lease_owner, None);
        let job_claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("job queue should claim: {error}"))
            .unwrap_or_else(|| panic!("job queue item should exist"));
        assert_eq!(job_claim.item.attempt, 1);
        let job_instance_id = materialized
            .node
            .job_instance_id
            .clone()
            .unwrap_or_else(|| panic!("job node should create job instance"));

        let running_marked = workflows
            .mark_dispatch_queue_running(&job_claim.item.id, "server-a")
            .await
            .unwrap_or_else(|error| panic!("job queue should mark running: {error}"));
        assert!(running_marked);

        let outcome = workflows
            .complete_job_node_from_result(&job_instance_id, InstanceStatus::Succeeded, None)
            .await
            .unwrap_or_else(|error| panic!("workflow should advance from job result: {error}"))
            .unwrap_or_else(|| panic!("job should be linked to workflow node"));

        assert_eq!(outcome.node_key, "first");
        assert_eq!(outcome.status, "succeeded");
        assert_eq!(outcome.queued_nodes, vec!["second".to_owned()]);
        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.status, "running");
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn dispatch_queue_can_close_by_terminal_job_instance() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = super::JobInstanceRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "terminal-close".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should be created: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        instances
            .update_status(&instance.id, InstanceStatus::Succeeded)
            .await
            .unwrap_or_else(|error| panic!("instance should become terminal: {error}"));
        assert!(
            workflows
                .mark_dispatch_queue_done_by_instance(&instance.id)
                .await
                .unwrap_or_else(|error| panic!("queue should close: {error}"))
        );

        let overview = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue overview should load: {error}"));
        assert_eq!(overview.pending, 0);
        assert_eq!(overview.running, 0);
        assert_eq!(overview.done, 1);
        assert_eq!(overview.items[0].status, "done");
        assert!(overview.items[0].lease_owner.is_none());
    }

    #[tokio::test]
    async fn dispatch_queue_claim_sets_and_releases_lease() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db.clone());
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "claimable".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "claim-flow".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "start".to_owned(),
                        name: None,
                        kind: Some("job".to_owned()),
                        job_id: Some(job.id),
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let _instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let claim = workflows
            .claim_next_dispatch_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue should claim: {error}"))
            .unwrap_or_else(|| panic!("queue item should be claimable"));
        assert_eq!(claim.lease_owner, "server-a");
        assert_eq!(claim.item.lease_owner.as_deref(), Some("server-a"));
        assert_eq!(
            claim.item.fencing_token.as_deref(),
            Some(claim.fencing_token.as_str())
        );
        assert!(claim.fencing_token.starts_with("lease:server-a:"));
        assert_eq!(claim.item.attempt, 1);
        assert!(claim.item.workflow_node_instance_id.is_some());

        let cleared = workflows
            .clear_expired_dispatch_queue_leases()
            .await
            .unwrap_or_else(|error| panic!("expired lease cleanup should run: {error}"));
        assert_eq!(cleared, 0);

        let second_claim = workflows
            .claim_dispatch_queue_item(&claim.item.id, "server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("second claim should not error: {error}"));
        assert!(second_claim.is_none());
        assert!(
            workflows
                .release_dispatch_queue_item(&claim.item.id, "server-a")
                .await
                .unwrap_or_else(|error| panic!("release should succeed: {error}"))
        );
        let reclaimed = workflows
            .claim_dispatch_queue_item_with_fencing(
                &claim.item.id,
                "server-b",
                30,
                Some("raft:server-b:term-2"),
            )
            .await
            .unwrap_or_else(|error| panic!("reclaim should succeed: {error}"))
            .unwrap_or_else(|| panic!("released item should be claimable"));
        assert_eq!(reclaimed.lease_owner, "server-b");
        assert_eq!(reclaimed.fencing_token, "raft:server-b:term-2");
        assert_eq!(
            reclaimed.item.fencing_token.as_deref(),
            Some("raft:server-b:term-2")
        );
        assert_eq!(reclaimed.item.attempt, 2);
    }

    #[tokio::test]
    async fn worker_pool_max_concurrency_blocks_additional_job_claims() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        seed_worker_pool_quota(&db, "tenant-a", "billing", "slow-pool", 0, 1).await;
        insert_scoped_job_queue_item(&db, "tenant-a", "billing", "slow-pool", "pending").await;
        insert_scoped_job_queue_item(&db, "tenant-a", "billing", "slow-pool", "pending").await;
        let workflows = super::WorkflowRepository::new(db.clone());

        let first = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("first queue item should claim: {error}"))
            .unwrap_or_else(|| panic!("first item should be claimable"));
        assert!(
            workflows
                .mark_dispatch_queue_running(&first.item.id, "server-a")
                .await
                .unwrap_or_else(|error| panic!("first item should mark running: {error}"))
        );

        let blocked = workflows
            .claim_next_job_queue_item("server-b", 30)
            .await
            .unwrap_or_else(|error| panic!("blocked queue scan should not error: {error}"));
        assert!(
            blocked.is_none(),
            "max_concurrency=1 must prevent a second running item in the same pool"
        );
        let summary = workflows
            .dispatch_queue_slo_summary()
            .await
            .unwrap_or_else(|error| panic!("queue summary should load: {error}"));
        assert_eq!(summary.blocked_by_quota, 1);
    }

    #[tokio::test]
    async fn worker_pool_max_queue_depth_blocks_until_depth_falls_below_limit() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        seed_worker_pool_quota(&db, "tenant-a", "billing", "tiny-pool", 1, 0).await;
        let first_queue_id =
            insert_scoped_job_queue_item(&db, "tenant-a", "billing", "tiny-pool", "pending").await;
        insert_scoped_job_queue_item(&db, "tenant-a", "billing", "tiny-pool", "pending").await;
        let workflows = super::WorkflowRepository::new(db.clone());

        let blocked = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue-depth scan should not error: {error}"));
        assert!(
            blocked.is_none(),
            "max_queue_depth=1 must backpressure when two active items are already queued"
        );
        let summary = workflows
            .dispatch_queue_slo_summary()
            .await
            .unwrap_or_else(|error| panic!("queue summary should load: {error}"));
        assert_eq!(summary.blocked_by_quota, 2);

        assert!(
            dispatch_queue::Entity::update_many()
                .col_expr(
                    dispatch_queue::Column::Status,
                    sea_orm::sea_query::Expr::value("done"),
                )
                .filter(dispatch_queue::Column::Id.eq(&first_queue_id))
                .exec(&db)
                .await
                .unwrap_or_else(|error| panic!("one queue item should close: {error}"))
                .rows_affected
                == 1
        );
        let claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue item should claim after depth falls: {error}"))
            .unwrap_or_else(|| panic!("remaining item should be claimable"));
        assert_ne!(claim.item.id, first_queue_id);
    }

    #[tokio::test]
    async fn worker_pool_quota_scan_skips_congested_pool_without_starving_later_pool() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        seed_worker_pool_quota(&db, "tenant-a", "billing", "congested", 1, 0).await;
        seed_worker_pool_quota(&db, "tenant-a", "billing", "open", 0, 0).await;
        for _ in 0..20 {
            insert_scoped_job_queue_item(&db, "tenant-a", "billing", "congested", "pending").await;
        }
        let open_queue_id =
            insert_scoped_job_queue_item(&db, "tenant-a", "billing", "open", "pending").await;
        let workflows = super::WorkflowRepository::new(db);

        let claim = workflows
            .claim_next_job_queue_item("server-a", 30)
            .await
            .unwrap_or_else(|error| panic!("queue scan should not error: {error}"))
            .unwrap_or_else(|| panic!("open pool item must remain claimable"));
        assert_eq!(claim.item.id, open_queue_id);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_shards_complete_and_advance_successor() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let reduce_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "reduce".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "shards".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "map".to_owned(),
                            name: None,
                            kind: Some("map".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: Some(vec![
                                serde_json::json!({"n": 1}),
                                serde_json::json!({"n": 2}),
                            ]),
                            config: None,
                        },
                        super::WorkflowNodeSpec {
                            key: "reduce".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(reduce_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "map".to_owned(),
                        to: "reduce".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map should materialize: {error}"))
            .unwrap_or_else(|| panic!("map queue should exist"));
        assert_eq!(materialized.shards.len(), 2);
        assert!(
            materialized
                .shards
                .iter()
                .all(|shard| shard.job_instance_id.is_some())
        );

        let first = workflows
            .complete_workflow_shard(
                &materialized.shards[0].id,
                super::CompleteWorkflowShardInput {
                    status: "succeeded".to_owned(),
                    output: Some(serde_json::json!({"ok": 1})),
                    checkpoint: None,
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("first shard should complete: {error}"))
            .unwrap_or_else(|| panic!("first shard should exist"));
        assert!(!first.node_completed);
        assert!(first.advance.is_none());

        let second = workflows
            .complete_workflow_shard(
                &materialized.shards[1].id,
                super::CompleteWorkflowShardInput {
                    status: "succeeded".to_owned(),
                    output: Some(serde_json::json!({"ok": 2})),
                    checkpoint: None,
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("second shard should complete: {error}"))
            .unwrap_or_else(|| panic!("second shard should exist"));
        assert!(second.node_completed);
        assert_eq!(second.node_status.as_deref(), Some("succeeded"));
        assert_eq!(
            second
                .advance
                .as_ref()
                .map(|advance| advance.queued_nodes.as_slice()),
            Some(&["reduce".to_owned()][..])
        );

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn cancel_job_instance_closes_dispatch_queue() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let instances = JobInstanceRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "cancel-me".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should create: {error}"));
        let instance = instances
            .create_pending(CreateJobInstance {
                job_id: job.id,
                trigger_type: TriggerType::Api,
                execution_mode: ExecutionMode::Single,
            })
            .await
            .unwrap_or_else(|error| panic!("instance should create: {error}"))
            .unwrap_or_else(|| panic!("job should exist"));

        assert!(
            workflows
                .cancel_job_instance(&instance.id)
                .await
                .unwrap_or_else(|error| panic!("cancel should persist: {error}"))
        );
        let reloaded = instances
            .get(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("instance should reload: {error}"))
            .unwrap_or_else(|| panic!("instance should exist"));
        assert_eq!(reloaded.status, InstanceStatus::Cancelled);
        let queue = workflows
            .queue_overview(10)
            .await
            .unwrap_or_else(|error| panic!("queue overview should load: {error}"));
        assert_eq!(queue.items[0].status, "cancelled");
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_map_reduce_writes_reduce_chunks_and_manifest() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db.clone());
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "map-reduce-manifest".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "reduce".to_owned(),
                        name: None,
                        kind: Some("map_reduce".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: Some(vec![
                            serde_json::json!({"n": 1}),
                            serde_json::json!({"n": 2}),
                            serde_json::json!({"n": 3}),
                        ]),
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map_reduce should materialize: {error}"))
            .unwrap_or_else(|| panic!("map_reduce queue should exist"));
        for (index, shard) in materialized.shards.iter().enumerate() {
            workflows
                .complete_workflow_shard(
                    &shard.id,
                    super::CompleteWorkflowShardInput {
                        status: "succeeded".to_owned(),
                        output: Some(serde_json::json!({"ok": index})),
                        checkpoint: Some(serde_json::json!({"offset": index})),
                        message: None,
                    },
                )
                .await
                .unwrap_or_else(|error| panic!("shard should complete: {error}"));
        }
        let events = crate::entities::instance_event::Entity::find()
            .filter(crate::entities::instance_event::Column::InstanceId.eq(instance.id))
            .all(&db)
            .await
            .unwrap_or_else(|error| panic!("events should load: {error}"));
        assert!(
            events
                .iter()
                .any(|event| event.event_type == "workflow.map_reduce.chunk")
        );
        let manifest = events
            .iter()
            .find(|event| event.event_type == "workflow.map_reduce.manifest")
            .unwrap_or_else(|| panic!("manifest event should exist"));
        let payload: serde_json::Value =
            serde_json::from_str(manifest.payload.as_deref().unwrap_or("{}"))
                .unwrap_or_else(|error| panic!("manifest payload should parse: {error}"));
        assert_eq!(payload["totalShards"], 3);
        assert_eq!(payload["spilled"], true);
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn workflow_failed_shard_rebalance_preserves_checkpoint_and_requeues() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "rebalance-shards".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "map".to_owned(),
                        name: None,
                        kind: Some("map".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: Some(vec![serde_json::json!({"n": 1})]),
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("map should materialize: {error}"))
            .unwrap_or_else(|| panic!("map queue should exist"));
        let failed = workflows
            .complete_workflow_shard(
                &materialized.shards[0].id,
                super::CompleteWorkflowShardInput {
                    status: "failed".to_owned(),
                    output: Some(serde_json::json!({"error": "boom"})),
                    checkpoint: Some(serde_json::json!({"offset": 42})),
                    message: Some("failed with checkpoint".to_owned()),
                },
            )
            .await
            .unwrap_or_else(|error| panic!("shard should fail: {error}"))
            .unwrap_or_else(|| panic!("shard should exist"));
        assert_eq!(
            failed.shard.checkpoint,
            Some(serde_json::json!({"offset": 42}))
        );

        let rebalanced = workflows
            .rebalance_workflow_shards(
                &instance.id,
                super::RebalanceWorkflowShardsInput {
                    node_key: Some("map".to_owned()),
                    statuses: Some(vec!["failed".to_owned()]),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("shards should rebalance: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));

        assert_eq!(rebalanced.requeued_shards.len(), 1);
        assert_eq!(rebalanced.requeued_shards[0].status, "pending");
        assert_eq!(rebalanced.requeued_shards[0].retry_count, 1);
        assert_eq!(
            rebalanced.requeued_shards[0].checkpoint,
            Some(serde_json::json!({"offset": 42}))
        );
        assert!(rebalanced.requeued_shards[0].job_instance_id.is_some());
    }

    #[tokio::test]
    #[allow(clippy::too_many_lines)]
    async fn child_workflow_completion_advances_parent_node() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let child_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "child-job".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let child = workflows
            .create_workflow(super::CreateWorkflow {
                name: "child".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "child-task".to_owned(),
                        name: None,
                        kind: Some("job".to_owned()),
                        job_id: Some(child_job.id),
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("child workflow should be created: {error}"));
        let parent = workflows
            .create_workflow(super::CreateWorkflow {
                name: "parent".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "child".to_owned(),
                        name: None,
                        kind: Some("sub_workflow".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: Some(child.id),
                        map_items: None,
                        config: None,
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("parent workflow should be created: {error}"));
        let parent_instance = workflows
            .run_workflow(&parent.id, "api")
            .await
            .unwrap_or_else(|error| panic!("parent should run: {error}"))
            .unwrap_or_else(|| panic!("parent should exist"));
        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("sub workflow should materialize: {error}"))
            .unwrap_or_else(|| panic!("sub workflow queue should exist"));
        let child_instance_id = materialized
            .node
            .child_workflow_instance_id
            .clone()
            .unwrap_or_else(|| panic!("child instance id should exist"));

        let advanced = workflows
            .advance_workflow(
                &child_instance_id,
                super::AdvanceWorkflowInput {
                    node_key: "child-task".to_owned(),
                    status: "succeeded".to_owned(),
                    message: None,
                },
            )
            .await
            .unwrap_or_else(|error| panic!("child should advance: {error}"))
            .unwrap_or_else(|| panic!("child should exist"));
        assert!(advanced.completed);

        let refreshed = workflows
            .get_workflow_instance(&parent_instance.id)
            .await
            .unwrap_or_else(|error| panic!("parent should load: {error}"))
            .unwrap_or_else(|| panic!("parent should exist"));
        assert_eq!(refreshed.status, "succeeded");
        assert_eq!(refreshed.nodes[0].status, "succeeded");
    }

    #[tokio::test]
    async fn workflow_condition_node_routes_failure_branch_and_auto_advances() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let jobs = JobRepository::new(db.clone());
        let workflows = super::WorkflowRepository::new(db);
        let false_branch_job = jobs
            .create_job(CreateJob {
                created_by: None,
                namespace: "default".to_owned(),
                app: "billing".to_owned(),
                name: "false-branch".to_owned(),
                schedule_type: "api".to_owned(),
                schedule_expr: None,
                misfire_policy: "fire_once".to_owned(),
                schedule_start_at: None,
                schedule_end_at: None,
                schedule_calendar_json: None,
                processor_name: None,
                processor_type: None,
                script_id: None,
                enabled: true,
                canary_job_id: None,
                canary_percent: 0,
                canary_policy: None,
                retry_policy: None,
            })
            .await
            .unwrap_or_else(|error| panic!("job should be created: {error}"));
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "condition-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "gate".to_owned(),
                            name: None,
                            kind: Some("condition".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({"expression": "false"})),
                        },
                        super::WorkflowNodeSpec {
                            key: "false-task".to_owned(),
                            name: None,
                            kind: Some("job".to_owned()),
                            job_id: Some(false_branch_job.id),
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "gate".to_owned(),
                        to: "false-task".to_owned(),
                        condition: Some("on_failure".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("condition should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued condition should exist"));

        assert_eq!(materialized.node.node_key, "gate");
        assert_eq!(materialized.node.status, "failed");
        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "failed");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn workflow_condition_node_evaluates_safe_typed_expression() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "typed-condition-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "gate".to_owned(),
                            name: None,
                            kind: Some("condition".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({
                                "expression": "vars.env == 'prod' && vars.progress >= 90 && vars.approved == true",
                                "vars": {
                                    "env": "prod",
                                    "progress": 95,
                                    "approved": true
                                }
                            })),
                        },
                        super::WorkflowNodeSpec {
                            key: "end".to_owned(),
                            name: None,
                            kind: Some("end".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "gate".to_owned(),
                        to: "end".to_owned(),
                        condition: Some("on_success".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("condition should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued condition should exist"));
        assert_eq!(materialized.node.node_key, "gate");
        assert_eq!(materialized.node.status, "succeeded");

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "succeeded");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn workflow_approval_node_times_out_and_routes_failure_branch() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db.clone());
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "approval-timeout-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "approve".to_owned(),
                            name: None,
                            kind: Some("approval".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({
                                "approvers": "ops",
                                "timeoutSeconds": 1,
                                "onTimeout": "failed"
                            })),
                        },
                        super::WorkflowNodeSpec {
                            key: "timeout-branch".to_owned(),
                            name: None,
                            kind: Some("end".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![super::WorkflowEdgeSpec {
                        from: "approve".to_owned(),
                        to: "timeout-branch".to_owned(),
                        condition: Some("on_failure".to_owned()),
                    }],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("approval should materialize: {error}"))
            .unwrap_or_else(|| panic!("queued approval should exist"));
        assert_eq!(materialized.node.node_key, "approve");
        assert_eq!(materialized.node.status, "running");

        let row = crate::entities::workflow_node_instance::Entity::find_by_id(materialized.node.id)
            .one(&db)
            .await
            .unwrap_or_else(|error| panic!("approval row should load: {error}"))
            .unwrap_or_else(|| panic!("approval row should exist"));
        let mut active: crate::entities::workflow_node_instance::ActiveModel = row.into();
        active.updated_at = Set("1970-01-01T00:00:00Z".to_owned());
        active
            .update(&db)
            .await
            .unwrap_or_else(|error| panic!("approval row should age out: {error}"));

        let expired = workflows
            .expire_timed_out_approval_nodes()
            .await
            .unwrap_or_else(|error| panic!("approval timeout scan should run: {error}"));
        assert_eq!(expired, 1);
        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "failed");
        assert_eq!(refreshed.nodes[1].status, "queued");
    }

    #[tokio::test]
    async fn workflow_compensation_node_auto_advances_after_failure_branch() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "compensation-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![
                        super::WorkflowNodeSpec {
                            key: "gate".to_owned(),
                            name: None,
                            kind: Some("condition".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(serde_json::json!({"expression": "false"})),
                        },
                        super::WorkflowNodeSpec {
                            key: "rollback".to_owned(),
                            name: None,
                            kind: Some("compensation".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: Some(
                                serde_json::json!({"compensates": "gate", "strategy": "saga"}),
                            ),
                        },
                        super::WorkflowNodeSpec {
                            key: "end".to_owned(),
                            name: None,
                            kind: Some("end".to_owned()),
                            job_id: None,
                            processor_name: None,
                            child_workflow_id: None,
                            map_items: None,
                            config: None,
                        },
                    ],
                    edges: vec![
                        super::WorkflowEdgeSpec {
                            from: "gate".to_owned(),
                            to: "rollback".to_owned(),
                            condition: Some("on_failure".to_owned()),
                        },
                        super::WorkflowEdgeSpec {
                            from: "rollback".to_owned(),
                            to: "end".to_owned(),
                            condition: Some("on_success".to_owned()),
                        },
                    ],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        let instance = workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("condition should materialize: {error}"));
        let compensation = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("compensation should materialize: {error}"))
            .unwrap_or_else(|| panic!("compensation should queue"));
        assert_eq!(compensation.node.node_key, "rollback");
        assert_eq!(compensation.node.status, "succeeded");

        let refreshed = workflows
            .get_workflow_instance(&instance.id)
            .await
            .unwrap_or_else(|error| panic!("workflow instance should load: {error}"))
            .unwrap_or_else(|| panic!("workflow instance should exist"));
        assert_eq!(refreshed.nodes[0].status, "failed");
        assert_eq!(refreshed.nodes[1].status, "succeeded");
        assert_eq!(refreshed.nodes[2].status, "queued");
    }

    #[tokio::test]
    async fn workflow_delay_node_uses_run_after_before_materializing() {
        let db = crate::connect_and_migrate("sqlite::memory:")
            .await
            .unwrap_or_else(|error| panic!("sqlite memory db should connect: {error}"));
        let workflows = super::WorkflowRepository::new(db);
        let workflow = workflows
            .create_workflow(super::CreateWorkflow {
                name: "delay-routing".to_owned(),
                created_by: "test".to_owned(),
                definition: super::WorkflowDefinition {
                    nodes: vec![super::WorkflowNodeSpec {
                        key: "wait".to_owned(),
                        name: None,
                        kind: Some("delay".to_owned()),
                        job_id: None,
                        processor_name: None,
                        child_workflow_id: None,
                        map_items: None,
                        config: Some(serde_json::json!({"seconds": 60})),
                    }],
                    edges: vec![],
                },
            })
            .await
            .unwrap_or_else(|error| panic!("workflow should be created: {error}"));
        workflows
            .run_workflow(&workflow.id, "api")
            .await
            .unwrap_or_else(|error| panic!("workflow should run: {error}"))
            .unwrap_or_else(|| panic!("workflow should exist"));

        let materialized = workflows
            .materialize_next_queued_node()
            .await
            .unwrap_or_else(|error| panic!("delay claim should not fail: {error}"));
        assert!(
            materialized.is_none(),
            "delay node must wait until run_after"
        );
    }
