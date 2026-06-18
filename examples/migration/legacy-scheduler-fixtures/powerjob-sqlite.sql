-- Local tikeo-migrate demo/CI fixture for PowerJob auto-export.
-- Production migrations should point tikeo-migrate at the real read-only MySQL/PostgreSQL scheduler DB.
create table if not exists pj_job_info (
  id integer primary key,
  job_name text not null,
  app_name text,
  time_expression_type integer,
  time_expression text,
  processor_info text,
  instance_retry_num integer default 0,
  execute_type text,
  max_instance_num integer,
  designated_workers text,
  status integer default 1
);

delete from pj_job_info;
insert into pj_job_info (
  id,
  job_name,
  app_name,
  time_expression_type,
  time_expression,
  processor_info,
  instance_retry_num,
  execute_type,
  max_instance_num,
  designated_workers,
  status
) values
  (2001, 'etl fanout', 'data-platform', 4, 'PT30S', 'etlProcessor', 1, 'BROADCAST', 4, null, 1),
  (2002, 'daily warehouse compact', 'data-platform', 2, '0 30 1 * * ?', 'compactProcessor', 2, 'STANDALONE', 1, null, 1);
