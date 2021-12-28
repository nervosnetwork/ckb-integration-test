CREATE DATABASE ckbtest;
\c ckbtest 
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

CREATE TABLE IF NOT EXISTS benchmark ( 
    github_run_id INT NOT NULL,
    github_run_state INT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
	github_branch VARCHAR (50) NOT NULL,
	github_trigger_event VARCHAR (50) NOT NULL,
    benchmark_report VARCHAR (200) NOT NULL
);

CREATE TABLE IF NOT EXISTS benchmark_report (
    github_run_id INT NOT NULL,
    "time" TIMESTAMP NOT NULL,
    ckb_version VARCHAR (60) NOT NULL,
	ckb_commit_id VARCHAR (20) NOT NULL,
	ckb_commit_time TIMESTAMP NOT NULL,
	transactions_per_second BIGINT NOT NULL,
	n_inout INT NOT NULL,
	n_nodes INT NOT NULL,
	delay_time_ms BIGINT NOT NULL,
    average_block_time_ms BIGINT NOT NULL,
    average_block_transactions INT NOT NULL,
    average_block_transactions_size INT NOT NULL,
    from_block_number BIGINT NOT NULL,
	to_block_number BIGINT NOT NULL,
	total_transactions BIGINT NOT NULL,
	total_transactions_size BIGINT NOT NULL,
	transactions_size_per_second BIGINT NOT NULL
);

SELECT create_hypertable('benchmark', 'start_time', migrate_data => true);

SELECT create_hypertable('benchmark', 'end_time', migrate_data => true);

SELECT create_hypertable('benchmark_report', 'time', migrate_data => true);