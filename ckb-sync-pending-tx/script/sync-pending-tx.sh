#!/usr/bin/env bash

# run step 
# 1. 部署节点1
# 2. 部署节点2
# 3. 部署节点3
# 4. 部署节点4
# 5. 等待节点4交易同步结束
# 6. 将节点4active 关闭
# 7. 节点1和节点2，节点3连接
# 8. 节点4和节点2, 节点3连接
# 9. 节点1 启动挖矿
# 10. 等待pending 全部同步
# 11. 生成图表

# ENVIRONMENT VARIABLES:
#
#   * AWS_ACCESS_KEY, required, the AWS access key
#   * AWS_SECRET_KEY, required, the AWS secret key
#   * AWS_EC2_TYPE, optional, default is c5.xlarge, the AWS EC2 type
#   * GITHUB_TOKEN, required, GitHub API authentication token

set -x
set -euo pipefail

AWS_EC2_TYPE=${AWS_EC2_TYPE:-"c5.xlarge"}
GITHUB_REF_NAME=${GITHUB_REF_NAME:-"develop"}
GITHUB_REPOSITORY=${GITHUB_REPOSITORY:-"nervosnetwork/ckb"}
START_TIME=${START_TIME:-"$(date +%Y-%m-%d' '%H:%M:%S.%6N)"}
GITHUB_BRANCH=${GITHUB_BRANCH:-"$GITHUB_REF_NAME"}
#  latest or v0.110.0 ...
download_ckb_version="latest"

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

ansible_config() {
  export ANSIBLE_PRIVATE_KEY_FILE=$SSH_PRIVATE_KEY_PATH
  export ANSIBLE_INVENTORY=$ANSIBLE_INVENTORY
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

# Setup Ansible running environment.
function ansible_setup() {
    cd $ANSIBLE_DIRECTORY
    ansible-galaxy install -r requirements.yml --force
}


# Deploy CKB onto target AWS EC2 instances.
### $1 : node1 ,node2 node3...
ansible_deploy_download_ckb() {
  ansible_config

  if [ ${download_ckb_version} == "latest" ]; then
    ckb_remote_url=`curl --silent "https://api.github.com/repos/nervosnetwork/ckb/releases/latest" | jq -r ".assets[].browser_download_url" | grep unknown-linux-gnu-portable | grep -v asc`
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "ckb_download_url=$ckb_remote_url node=$1" \
      -t ckb_install,ckb_data_install,ckb_configure,ckb_start
    return
  fi
  ckb_remote_url="https://github.com/nervosnetwork/ckb/releases/download/${download_ckb_version}/ckb_${download_ckb_version}_x86_64-unknown-centos-gnu.tar.gz"
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "ckb_download_url=$ckb_remote_url node=$1" \
    -t ckb_install,ckb_data_install,ckb_configure,ckb_start

}

### $1 : node1 ,node2 node3...
ansible_run_ckb() {
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_start
}

#
#function wait_pending_tx_load() {
#    ## 等待节点4的pending tx 同步
#}
#
# node1, node2
function link_node_p2p() {

  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=localhost" \
    -e "n1=$1" \
    -e "n2=$2" \
    -t ckb_add_node
    ## 连接节点1,2 1,3
    ## 连接节点4,2 4,3
}
#
#function start_node_miner() {
#    ## 启动节点1 挖矿
#}
#
#function start_monit_pending_tx() {
#    ## 启动monit 监控4个节点pending 池和高度
#}
#
#function wait_txs_commit() {
#    ##
#}

# $1: node1 ,node2 ...
clean_ckb_env(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=$1" \
    -t ckb_stop,ckb_clean
}

ckb_miner(){
  ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "node=$1" \
      -t ckb_miner_start
}

# node1 10
ckb_wait_pending_load(){
  ansible_config
  cd $ANSIBLE_DIRECTORY
  ansible-playbook playbook.yml \
    -e "node=localhost" \
    -e "n1=$1" \
    -e "pending=$2" \
    -t ckb_wait_pending_load
}
#// node false
function ckb_set_network_active() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    ansible-playbook playbook.yml \
      -e "node=localhost" \
      -e "n1=$1" \
      -e "network_status=$2" \
      -t ckb_set_network_active
}
function get_prepare_data() {
    ansible_config
    cd $ANSIBLE_DIRECTORY
    cd files
    git clone https://github.com/gpBlockchain/ckb-prepare-data.git && \
    cd ckb-prepare-data && \
    git checkout 10b6d09b5f0cf898f46d4fa6f018007262461918 && \
    git rm -r * && \
    git checkout HEAD -- big-tx
}

#todo
# 添加监控指标
main() {
  case $1 in
    "get-prepare-data")
      get_prepare_data
      ;;
    "run")
#      job_setup
      ansible_deploy_download_ckb node1
      ansible_deploy_download_ckb node2
      ansible_deploy_download_ckb node3
      ansible_deploy_download_ckb node4
      ckb_wait_pending_load node4 8000
      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node4
      link_node_p2p node3 node4
      ckb_miner node1
      ;;
    "setup")
      job_setup
      get_prepare_data
      ;;
    "deploy_ckb")
      ansible_deploy_download_ckb node1
      ansible_deploy_download_ckb node2
      ansible_deploy_download_ckb node3
      ansible_deploy_download_ckb node4
      ;;
    "run_ckb")
      ansible_run_ckb node1
      ansible_run_ckb node2
      ansible_run_ckb node3
      ansible_run_ckb node4
      ;;
    "miner")
      ckb_miner node1
      ;;
   "clean_ckb_env")
      clean_ckb_env node1
      clean_ckb_env node2
      clean_ckb_env node3
      clean_ckb_env node4
#      clean_ckb_env node5
      ;;
   "clean_job")
#      job_clean
      ;;
   "insert_report_to_postgres")
#      insert_report_to_postgres
      ;;
    "add_node")
      link_node_p2p node1 node2
      link_node_p2p node1 node3
      link_node_p2p node2 node4
      link_node_p2p node3 node4
      ;;
    "ckb_pending")
      ckb_wait_pending_load node4 100
      ;;
  esac
}

main $*


