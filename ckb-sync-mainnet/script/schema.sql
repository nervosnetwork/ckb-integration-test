CREATE DATABASE ckbtest;
\c ckbtest 
CREATE EXTENSION IF NOT EXISTS timescaledb CASCADE;

CREATE TABLE IF NOT EXISTS sync_mainnet (
    github_run_id INT NOT NULL,
    github_run_state INT NOT NULL,
    start_time TIMESTAMP NOT NULL,
    end_time TIMESTAMP NOT NULL,
	github_branch VARCHAR (50) NOT NULL,
	github_trigger_event VARCHAR (50) NOT NULL,
    github_run_link VARCHAR (200) NOT NULL
);

CREATE TABLE IF NOT EXISTS sync_mainnet_report (
    github_run_id INT NOT NULL,
    "time" TIMESTAMP NOT NULL,
    ckb_version VARCHAR (60) NOT NULL,
	ckb_commit_id VARCHAR (20) NOT NULL,
	ckb_commit_time TIMESTAMP NOT NULL,
	time_s TIMESTAMP NOT NULL,
	speed BIGINT NOT NULL,
	tip BIGINT NOT NULL,
	hostname BIGINT NOT NULL
);

SELECT create_hypertable('sync_mainnet', 'start_time', migrate_data => true);

SELECT create_hypertable('sync_mainnet', 'end_time', migrate_data => true);

SELECT create_hypertable('sync_mainnet_report', 'time', migrate_data => true);

SELECT create_hypertable('sync_mainnet_report', 'time_s', migrate_data => true);
