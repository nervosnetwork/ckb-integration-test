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

download_ckb_version="latest"
JOB_ID=${JOB_ID:-"sync-mainnet-$(date +'%Y-%m-%d')-in-10h"}
TAR_FILENAME="ckb.sync-mainnet-$(date +'%Y-%m-%d')-in-10h.tar.gz"
SCRIPT_PATH="$(
  cd -- "$(dirname "$0")" >/dev/null 2>&1
  pwd -P
)"
JOB_DIRECTORY="$(dirname "$SCRIPT_PATH")/job/$JOB_ID"
ANSIBLE_DIRECTORY=$JOB_DIRECTORY/ansible
ANSIBLE_INVENTORY=$JOB_DIRECTORY/ansible/inventory.yml
TERRAFORM_DIRECTORY="$JOB_DIRECTORY/terraform"
SSH_PRIVATE_KEY_PATH=$JOB_DIRECTORY/ssh/id
SSH_PUBLIC_KEY_PATH=$JOB_DIRECTORY/ssh/id.pub
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
GITHUB_REF_NAME=${GITHUB_REF_NAME:-"develop"}
GITHUB_REPOSITORY=${GITHUB_REPOSITORY:-"nervosnetwork/ckb"}
GITHUB_BRANCH=${GITHUB_BRANCH:-"$GITHUB_REF_NAME"}

job_setup() {
  mkdir -p $JOB_DIRECTORY
  cp -r "$(dirname "$SCRIPT_PATH")/ansible" $JOB_DIRECTORY/ansible
  cp -r "$(dirname "$SCRIPT_PATH")/terraform" $JOB_DIRECTORY/terraform

  ssh_gen_key
  ansible_setup
}

job_clean() {
  rm -rf $JOB_DIRECTORY
}

# fetch and return current mainnet tip_block_number
job_target_tip_number() {
  # parse data like: {"jsonrpc":"2.0","result":"0x6f7959","id":42}
  tip=$(curl -s -X POST -H 'Content-Type: application/json' -d \
    '{ "jsonrpc": "2.0", "id": 42, "method":"get_tip_block_number", "params":[] }' \
    http://mainnet.ckb.dev:80 | grep "result" | awk -F '"' '{ print strtonum($8) }')
  echo "$tip"
}

ssh_gen_key() {
  # Pre-check whether "./ssh" existed
  if [ -e "$SSH_PRIVATE_KEY_PATH" ]; then
    echo "Info: $SSH_PRIVATE_KEY_PATH already existed, reuse it"
    return 0
  fi

  mkdir -p "$(dirname $SSH_PRIVATE_KEY_PATH)"
  ssh-keygen -t rsa -N "" -f $SSH_PRIVATE_KEY_PATH
}

terraform_config() {
  export TF_VAR_access_key=$AWS_ACCESS_KEY
  export TF_VAR_secret_key=$AWS_SECRET_KEY
  export TF_VAR_prefix=$JOB_ID
  export TF_VAR_private_key_path=$SSH_PRIVATE_KEY_PATH
  export TF_VAR_public_key_path=$SSH_PUBLIC_KEY_PATH
}

# Allocate AWS resources defined in Terraform.
#
# The Terraform directory is "./terraform".
terraform_apply() {
  terraform_config

  cd $TERRAFORM_DIRECTORY
  terraform init
  terraform plan
  terraform apply -auto-approve
  terraform output | grep -v EOT | tee $ANSIBLE_INVENTORY
}

# Destroy AWS resources
terraform_destroy() {
  terraform_config

  cd $TERRAFORM_DIRECTORY
  terraform destroy -auto-approve
}

ansible_config() {
  export ANSIBLE_PRIVATE_KEY_FILE=$SSH_PRIVATE_KEY_PATH
  export ANSIBLE_INVENTORY=$ANSIBLE_INVENTORY
}

# Setup Ansible running environment.
ansible_setup() {
  cd $ANSIBLE_DIRECTORY
  ansible-galaxy install -r requirements.yml --force
}

# Deploy CKB onto target AWS EC2 instances.
ansible_deploy_download_ckb() {
  ansible_config

  if [ ${download_ckb_version} == "latest" ]; then
    ckb_remote_url=`curl --silent "https://api.github.com/repos/nervosnetwork/ckb/releases/latest" | jq -r ".assets[].browser_download_url" | grep unknown-linux-gnu-portable | grep -v asc`
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "ckb_download_url=$ckb_remote_url" \
      -t ckb_install,ckb_configure
    return
  fi
  ckb_remote_url="https://github.com/nervosnetwork/ckb/releases/download/${download_ckb_version}/ckb_${download_ckb_version}_x86_64-unknown-centos-gnu.tar.gz"
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "ckb_download_url=$ckb_remote_url" \
    -t ckb_install,ckb_configure

}

ansible_deploy_local_ckb(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ckb_local_source=$JOB_DIRECTORY/ckb/target/release/"$TAR_FILENAME"
  ansible-playbook playbook.yml \
    -e "ckb_local_source=$ckb_local_source" \
    -t ckb_install,ckb_configure
}



# Wait for CKB synchronization completion.
ansible_wait_ckb_synchronization() {
  ansible_config

  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml -t ckb_restart
  ansible-playbook playbook.yml -t wait_ckb_synchronization -e "ckb_sync_target_number=$(job_target_tip_number)"
}

ansible_ckb_replay() {
  ansible_config

  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml -t ckb_replay
}

markdown_report() {
  case "$OSTYPE" in
    darwin*)
      if ! type gsed &>/dev/null || ! type ggrep &>/dev/null; then
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
github_add_comment() {
  export GITHUB_TOKEN=${GITHUB_TOKEN}
  report="$1"
  $SCRIPT_PATH/ok.sh add_comment nervosnetwork/ckb 2372 "$report"

  CKB_HEAD_REF=$(cd $JOB_DIRECTORY/ckb && git log --pretty=format:'%h' -n 1)
  $SCRIPT_PATH/ok.sh add_commit_comment nervosnetwork/ckb $CKB_HEAD_REF "$report"
}

clean_ckb_env(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -t ckb_clean
}

build_ckb() {
  git -C $JOB_DIRECTORY clone \
    --branch $GITHUB_BRANCH \
    --depth 1 \
    https://github.com/$GITHUB_REPOSITORY.git

  cd $JOB_DIRECTORY/ckb
  make build

  cd target/release
  tar czf "$TAR_FILENAME" ckb
}

parse_report_and_inster_to_postgres() {
  time=$START_TIME
  #cat *.brief.md if it exist
  if [ -n "'ls $ANSIBLE_DIRECTORY/*.brief.md'" ]; then
    cat $ANSIBLE_DIRECTORY/*.brief.md >$ANSIBLE_DIRECTORY/sync-mainnet.brief.md
  fi
  if [ -f "$ANSIBLE_DIRECTORY/sync-mainnet.brief.md" ]; then
    while read -r LINE; do
      LINE=$(echo "$LINE" | sed -e 's/\r//g')
      ckb_version=$(echo $LINE | awk -F '|' '{print $2}')
      time_s=$(echo $LINE | awk -F '|' '{print $3}')
      speed=$(echo $LINE | awk -F '|' '{print $4}')
      tip=$(echo $LINE | awk -F '|' '{print $5}')
      hostname=$(echo $LINE | awk -F '|' '{print $6}')
      replay_tps=$(echo $LINE | awk -F '|' '{print $8}')
      psql -c "INSERT INTO sync_mainnet_report (github_run_id,time,ckb_version,ckb_commit_id,ckb_commit_time,time_s,speed,tip,hostname,replay_tps)  \
             VALUES ('$GITHUB_RUN_ID','$time','$ckb_version','$CKB_COMMIT_ID','$CKB_COMMIT_TIME','$time_s','$speed','$tip','$hostname','$replay_tps');"
    done <"$ANSIBLE_DIRECTORY/sync-mainnet.brief.md"
  fi
}

insert_report_to_postgres() {
  export PGHOST=${PGHOST}
  export PGPORT=${PGPORT}
  export PGUSER=${PGUSER}
  export PGPASSWORD=${PGPASSWORD}
  export PGDATABASE=${PGDATABASE}
  export GITHUB_RUN_ID=${GITHUB_RUN_ID}
  export CKB_COMMIT_ID=${CKB_COMMIT_ID}
  export CKB_COMMIT_TIME=${CKB_COMMIT_TIME}
  export GITHUB_RUN_STATE=${GITHUB_RUN_STATE:-0} #0:success,1:failed
  export GITHUB_EVENT_NAME=${GITHUB_EVENT_NAME}
  END_TIME=$(date +%Y-%m-%d' '%H:%M:%S.%6N)
  GITHUB_RUN_LINK="https://github.com/${GITHUB_REPOSITORY}/actions/runs/$GITHUB_RUN_ID"
  psql -c "INSERT INTO sync_mainnet (github_run_id,github_run_state,start_time,end_time,github_branch,github_trigger_event,github_run_link)  \
             VALUES ('$GITHUB_RUN_ID','$GITHUB_RUN_STATE','$START_TIME','$END_TIME','$GITHUB_BRANCH','$GITHUB_EVENT_NAME','$GITHUB_RUN_LINK');"
  parse_report_and_inster_to_postgres
}

main() {
  case $1 in
    "run")
      job_setup
      terraform_apply
      build_ckb
      ansible_deploy_local_ckb
      ansible_wait_ckb_synchronization
      github_add_comment "$(markdown_report)"
      ;;
    "setup")
      job_setup
      ;;
    "build")
      build_ckb
      ;;
    "terraform")
      terraform_apply
      ;;
    "ansible")
      ansible_deploy_download_ckb
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
   "clean_ckb_env")
      clean_ckb_env
      ;;
   "clean_job")
      job_clean
      ;;
   "insert_report_to_postgres")
      insert_report_to_postgres
      ;;
  esac
}

main $*
