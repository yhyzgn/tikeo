-- Local tikeo-migrate demo/CI fixture for XXL-JOB auto-export.
-- Production migrations should point tikeo-migrate at the real read-only MySQL/PostgreSQL scheduler DB.
create table if not exists xxl_job_info (
  id integer primary key,
  job_desc text not null,
  executor_app_name text,
  schedule_type text,
  schedule_conf text,
  executor_handler text,
  executor_fail_retry_count integer default 0,
  trigger_status integer default 1,
  executor_route_strategy text,
  executor_block_strategy text,
  glue_type text
);

delete from xxl_job_info;
insert into xxl_job_info (
  id,
  job_desc,
  executor_app_name,
  schedule_type,
  schedule_conf,
  executor_handler,
  executor_fail_retry_count,
  trigger_status,
  executor_route_strategy,
  executor_block_strategy,
  glue_type
) values
  (1001, 'nightly billing', 'billing', 'CRON', '0 0 2 * * ?', 'billingProcessor', 2, 1, null, null, 'BEAN'),
  (1002, 'disabled report rebuild', 'reporting', 'NONE', null, 'reportRebuildProcessor', 0, 0, 'FIRST', 'SERIAL_EXECUTION', 'BEAN');
