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

JOB_ID="sync-mainnet-$(date +'%Y-%m-%d')-in-10h"
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

function job_target_tip_number() {
    curl https://api.explorer.nervos.org/api/v1/statistics/tip_block_number \
        -H 'Accept: application/vnd.api+json' \
        -H 'Content-Type: application/vnd.api+json' | jq .data.attributes.tip_block_number
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
        -e "ckb_local_source=$ckb_local_source" \
        -t ckb_install,ckb_configure
}

# Wait for CKB synchronization completion.
function ansible_wait_ckb_synchronization() {
    ansible_config

    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml -t ckb_restart
    ansible-playbook playbook.yml -t wait_ckb_synchronization -e "ckb_sync_target_number=$(job_target_tip_number)"
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
    echo "**Sync-Mainnet Report**:"
    echo "| Version | Time(s) | Speed | Tip | Hostname | Network |"
    echo "| :--- | :--- | :--- | :--- | :--- | :--- |"
    cat *.brief.md
}

# Upload report through GitHub issue comment
function github_add_comment() {
    report="$1"
    $SCRIPT_PATH/ok.sh add_comment nervosnetwork/ckb 2372 "$report"
}

function rust_build() {
    git -C $JOB_DIRECTORY clone \
        --branch develop \
        --depth 1 \
        https://github.com/nervosnetwork/ckb.git

    cd $JOB_DIRECTORY/ckb
    make build

    cd target/release
    tar czf ckb.$JOB_ID.tar.gz ckb
}

function main() {
    case $1 in
        "run")
            job_setup
            terraform_apply
            rust_build
            ansible_deploy_ckb
            ansible_wait_ckb_synchronization
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
            ansible_wait_ckb_synchronization
            markdown_report
            ;;
        "report")
            markdown_report
            ;;
        "clean")
            terraform_destroy
            job_clean
            ;;
        esac
}

main $*
