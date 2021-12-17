#!/usr/bin/env bash

# ENVIRONMENT VARIABLES:
# 
#   * AWS_ACCESS_KEY, required, the AWS access key
#   * AWS_SECRET_KEY, required, the AWS secret key
#   * AWS_EC2_TYPE, optional, default is c5.xlarge, the AWS EC2 type
#   * GITHUB_TOKEN, required, GitHub API authentication token


set -euo pipefail

AWS_ACCESS_KEY=${AWS_ACCESS_KEY}
AWS_SECRET_KEY=${AWS_SECRET_KEY}
AWS_EC2_TYPE=${AWS_EC2_TYPE:-"c5.xlarge"}
GITHUB_TOKEN=${GITHUB_TOKEN}
GITHUB_REF_NAME=${GITHUB_REF_NAME:-"develop"}
GITHUB_REPO=${GITHUB_REPO:-"nervosnetwork/ckb"}
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
GITHUB_BRANCH=${GITHUB_BRANCH:-"$GITHUB_REF_NAME"}

JOB_ID=${JOB_ID:-"benchmark-$(date +'%Y-%m-%d')-in-10h"}
SCRIPT_PATH="$( cd -- "$(dirname "$0")" >/dev/null 2>&1 ; pwd -P )"
JOB_DIRECTORY="$(dirname "$SCRIPT_PATH")/job/$JOB_ID"
ANSIBLE_DIRECTORY=$JOB_DIRECTORY/ansible
ANSIBLE_INVENTORY=$JOB_DIRECTORY/ansible/inventory.yml
TERRAFORM_DIRECTORY="$JOB_DIRECTORY/terraform"
SSH_PRIVATE_KEY_PATH=$JOB_DIRECTORY/ssh/id
SSH_PUBLIC_KEY_PATH=$JOB_DIRECTORY/ssh/id.pub

function job_setup() {
    mkdir -p $JOB_DIRECTORY
    cp -r "$(dirname "$SCRIPT_PATH")/ansible"   $JOB_DIRECTORY/ansible
    cp -r "$(dirname "$SCRIPT_PATH")/terraform" $JOB_DIRECTORY/terraform

    ssh_gen_key
    ansible_setup
}

function job_clean() {
    rm -rf $JOB_DIRECTORY
}

function ssh_gen_key() {
    # Pre-check whether "./ssh" existed
    if [ -e "$SSH_PRIVATE_KEY_PATH" ]; then
        echo "Info: $SSH_PRIVATE_KEY_PATH already existed, reuse it"
        return 0
    fi

    mkdir -p "$(dirname $SSH_PRIVATE_KEY_PATH)"
    ssh-keygen -t rsa -N "" -f $SSH_PRIVATE_KEY_PATH
}

function terraform_config() {
    export TF_VAR_access_key=$AWS_ACCESS_KEY
    export TF_VAR_secret_key=$AWS_SECRET_KEY
    export TF_VAR_prefix=$JOB_ID
    export TF_VAR_private_key_path=$SSH_PRIVATE_KEY_PATH
    export TF_VAR_public_key_path=$SSH_PUBLIC_KEY_PATH
}

# Allocate AWS resources defined in Terraform.
#
# The Terraform directory is "./terraform".
function terraform_apply() {
    terraform_config

    cd $TERRAFORM_DIRECTORY
    terraform init
    terraform plan
    terraform apply -auto-approve
    terraform output | grep -v EOT | tee $ANSIBLE_INVENTORY
}

# Destroy AWS resources
function terraform_destroy() {
    terraform_config

    cd $TERRAFORM_DIRECTORY
    terraform destroy -auto-approve
}

function ansible_config() {
    export ANSIBLE_PRIVATE_KEY_FILE=$SSH_PRIVATE_KEY_PATH
    export ANSIBLE_INVENTORY=$ANSIBLE_INVENTORY
}

# Setup Ansible running environment.
function ansible_setup() {
    cd $ANSIBLE_DIRECTORY
    ansible-galaxy install -r requirements.yml --force
}

# Deploy CKB onto target AWS EC2 instances.
function ansible_deploy_ckb() {
    ansible_config

    cd $ANSIBLE_DIRECTORY
    ckb_local_source=$JOB_DIRECTORY/ckb/target/release/ckb.$JOB_ID.tar.gz
    ansible-playbook playbook.yml \
        -e 'hostname=instances' \
        -e "ckb_local_source=$ckb_local_source" \
        -t ckb_install,ckb_configure
}

# Wait for CKB synchronization completion.
function ansible_wait_ckb_benchmark() {
    ansible_config

    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml -e 'hostname=instances' -t ckb_start
    ansible-playbook playbook.yml -e 'hostname=bastions'  -t ckb_benchmark_install
    ansible-playbook playbook.yml -e 'hostname=bastions'  -t ckb_benchmark_prepare
    ansible-playbook playbook.yml -e 'hostname=bastions'  -t ckb_benchmark_start
    ansible-playbook playbook.yml -e 'hostname=bastions'  -t process_result
}

function markdown_report() {
    case "$OSTYPE" in
        darwin*)
            if ! type gsed &> /dev/null || ! type ggrep &> /dev/null; then
                echo "GNU sed and grep not found! You can install via Homebrew" >&2
                echo >&2
                echo "    brew install grep gnu-sed" >&2
                exit 1
            fi

            SED=gsed
            GREP=ggrep
            ;;
        *)
            SED=sed
            GREP=grep
            ;;
    esac

    ansible_config

    cd $ANSIBLE_DIRECTORY
    echo "**Benchmark Report**:"
    echo "| ckb_version | txs_per_second | n_inout | n_nodes | delay_time_ms | average_block_time_ms | average_block_transactions | average_block_transactions_size | from_block_number | to_block_number | total_transactions | total_transactions_size | transactions_size_per_second |"
    echo "| :---------- | :------------- | :------ | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- | :--- |"
    cat *.brief.md
}

# Upload report through GitHub issue comment
function github_add_comment() {
    report="$1"
    export GITHUB_TOKEN=${GITHUB_TOKEN}
    $SCRIPT_PATH/ok.sh add_comment nervosnetwork/ckb 2372 "$report"
}

function rust_build() {
    git -C $JOB_DIRECTORY clone \
        --branch $GITHUB_BRANCH \
        --depth 1 \
        https://github.com/$GITHUB_REPO.git

    cd $JOB_DIRECTORY/ckb
    sed -i 's/const TWO_IN_TWO_OUT_COUNT: u64 = .*;$/const TWO_IN_TWO_OUT_COUNT: u64 = 8_000;/g'            spec/src/consensus.rs
    sed -i 's/const MAX_BLOCK_PROPOSALS_LIMIT: u64 = .*;$/const MAX_BLOCK_PROPOSALS_LIMIT: u64 = 12_000;/g' spec/src/consensus.rs
    make build

    cd target/release
    tar czf ckb.$JOB_ID.tar.gz ckb
}

function parse_report_and_inster_to_postgres() {
  time=$START_TIME
  if [ -f "$ANSIBLE_DIRECTORY/ckb-bench.brief.md" ]; then
    while read -r LINE;
    do
      LINE=$(echo "$LINE" | sed -e 's/\r//g')
      ckb_version=$(echo $LINE | awk -F '|' '{print $2}')
      transactions_per_second=$(echo $LINE | awk -F '|' '{print $3}')
      n_inout=$(echo $LINE | awk -F '|' '{print $4}')
      n_nodes=$(echo $LINE | awk -F '|' '{print $5}')
      delay_time_ms=$(echo $LINE | awk -F '|' '{print $6}')
      average_block_time_ms=$(echo $LINE | awk -F '|' '{print $7}')
      average_block_transactions=$(echo $LINE | awk -F '|' '{print $8}')
      average_block_transactions_size=$(echo $LINE | awk -F '|' '{print $9}')
      from_block_number=$(echo $LINE | awk -F '|' '{print $10}')
      to_block_number=$(echo $LINE | awk -F '|' '{print $11}')
      total_transactions=$(echo $LINE | awk -F '|' '{print $12}')
      total_transactions_size=$(echo $LINE | awk -F '|' '{print $13}')
      transactions_size_per_second=$(echo $LINE | awk -F '|' '{print $14}')
      psql -c "INSERT INTO benchmark_report (benchmark_id,time,github_branch,trigger_event,ckb_version,ckb_commit_id,ckb_commit_time, \
               transactions_per_second,n_inout,n_nodes,delay_time_ms,average_block_time_ms,average_block_transactions,average_block_transactions_size, \
               from_block_number,to_block_number,total_transactions,total_transactions_size,transactions_size_per_second)  \
               VALUES ('$BENCHMARK_ID','$time','$GITHUB_BRANCH','$GITHUB_EVENT_NAME','$ckb_version','$CKB_COMMIT_ID','$CKB_COMMIT_TIME', \
               '$transactions_per_second','$n_inout','$n_nodes','$delay_time_ms','$average_block_time_ms','$average_block_transactions', \
               '$average_block_transactions_size','$from_block_number','$to_block_number','$total_transactions','$total_transactions_size','$transactions_size_per_second');"
    done < "$ANSIBLE_DIRECTORY/ckb-bench.brief.md"
  fi
}

function insert_report_to_postgres() {
    export PGHOST=${PGHOST}
    export PGPORT=${PGPORT}
    export PGUSER=${PGUSER}
    export PGPASSWORD=${PGPASSWORD}
    export PGDATABASE=${PGDATABASE}
    export BENCHMARK_ID=${GITHUB_RUN_ID:-"$RANDOM"}
    export CKB_COMMIT_ID=${CKB_COMMIT_ID}
    export CKB_COMMIT_TIME=${CKB_COMMIT_TIME}
    export STATE=${STATE:-0} #0:success,1:failed
    END_TIME=$(date +%Y-%m-%d' '%H:%M:%S.%6N)
    # dbname="ckbtest"
    BENCHMARK_REPORT="https://github.com/${GITHUB_REPOSITORY}actions/runs/$GITHUB_RUN_ID"
    psql -c "INSERT INTO benchmark (benchmark_id,state,start_time,end_time,github_branch,trigger_event,benchmark_report)  \
             VALUES ('$BENCHMARK_ID','$STATE','$START_TIME','$END_TIME','$GITHUB_BRANCH','$GITHUB_EVENT_NAME','$BENCHMARK_REPORT');"
    parse_report_and_inster_to_postgres
}

function main() {
    case $1 in
        "run")
            job_setup
            terraform_apply
            rust_build
            ansible_deploy_ckb
            ansible_wait_ckb_benchmark
            github_add_comment "$(markdown_report)"
            ;;
        "setup")
            job_setup
            ;;
        "build")
            rust_build
            ;;
        "terraform")
            terraform_apply
            ;;
        "ansible")
            ansible_deploy_ckb
            ansible_wait_ckb_benchmark
            markdown_report
            ;;
        "report")
            markdown_report
            ;;
        "clean")
            terraform_destroy
            job_clean
            ;;
        "insert_report_to_postgres")
          insert_report_to_postgres
          ;;
        esac
}

main $*
