#!/usr/bin/env python3

# Usage: `./save_bench_to_db.py report.yml`

import os
import sys
import datetime;
from urllib.parse import urlparse

import psycopg2
import yaml
from psycopg2 import sql

BENCHMARK_POSTGRES_SECRET = os.getenv("BENCH_DB_CONN")


def update_benchmark_to_database(metrics) -> bool:
    """
    insert benchmark result into ckb analyzer database
    :param metric: metric as saved report information
    :return: bool indicate operation status(false if op failed)
    """
    o = urlparse(BENCHMARK_POSTGRES_SECRET, 'postgres')
    user = o.username.strip()
    password = o.password.strip()
    host = o.hostname.strip()
    port = o.port
    db = o.path.strip('/')

    if not user or not password or not host or port is None or not db:
        print("urlparse error, please check secrets")
        return False

    result = False
    conn = None
    try:
        conn = psycopg2.connect(
            host=host,
            database=db,
            user=user,
            password=password)
        cur = conn.cursor()
        # metrics(multi-dimension list) contains multiple bench result
        for metric in metrics:
            cur.execute(
                sql.SQL(
                    """insert into bench.ci_bench (time, average_block_time_ms, average_block_transactions,
                    average_block_transactions_size, ckb_version, delay_time_ms,
                    from_block_number, instance_bastion_type, instance_type,
                    n_inout, n_nodes, to_block_number,
                    total_transactions, total_transactions_size, transactions_per_second,
                    transactions_size_per_second) values (%s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s, %s)"""),
                [datetime.datetime.now(), metric["average_block_time_ms"], metric["average_block_transactions"],
                 metric["average_block_transactions_size"], metric["ckb_version"], metric["delay_time_ms"],
                 metric["from_block_number"], metric["instance_bastion_type"], metric["instance_type"],
                 metric["n_inout"],
                 metric["n_nodes"], metric["to_block_number"], metric["total_transactions"],
                 metric["total_transactions_size"], metric["transactions_per_second"],
                 metric["transactions_size_per_second"]])
        conn.commit()

    except (Exception, psycopg2.DatabaseError) as error:
        print(error)
    else:
        result = True
    finally:
        if conn is not None:
            conn.close()

    return result


def main():
    with open(sys.argv[1], 'r') as infile:
        try:
            metrics = yaml.safe_load(infile)
            update_benchmark_to_database(metrics)
        except yaml.YAMLError as error:
            print(error)


main()

