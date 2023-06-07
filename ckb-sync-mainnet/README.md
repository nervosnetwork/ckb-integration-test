# CKB Sync-Mainnet Test

This job will compile ckb locally, please use `ubuntu-focal-20.04-amd64` to run the scripts.

Required environment variables:

- `AWS_ACCESS_KEY`
- `AWS_SECRET_KEY`
- `GITHUB_TOKEN`

Usage:

```
# Start a sync-mainnet test
./script/sync-mainnet.sh run

# Clean environment
./script/sync-mainnet.sh clean
```

### Run with Exist Remote Service
Since there is no AWS key when debugging and running test cases, you need to use an existing remote server for debugging. Below is an example of debugging with a remote server.

#### 0. Install dependencies
- ansible

#### 1. Add ssh login method on the remote server

1. **Generate your key pair**: If you haven't done it yet, you can generate a new SSH key pair on your client machine with the following command:
   ```
   ssh-keygen
   ```
   Follow the prompts, and it will generate a public key and a private key.

2. **View and copy your public key**: Use the following command to view and copy your public key:
   ```
   cat ~/.ssh/id_rsa.pub
   ```
   This will print your public key. You should copy this public key.

3. **Create .ssh directory on the server**: If it does not exist yet, you need to create a `.ssh` directory for your user on your server:
   ```
   mkdir -p ~/.ssh
   ```
   This will create a `.ssh` directory in your home directory. The `-p` option allows `mkdir` to create parent directories as needed. If the `.ssh` directory already exists, it will not display an error.

4. **Create authorized_keys file on the server**: Then, create an `authorized_keys` file on the server:
   ```
   touch ~/.ssh/authorized_keys
   ```
   If the file already exists, this command will do nothing.

5. **Add your public key to the authorized_keys file**: Finally, add the copied public key to the `authorized_keys` file. You can use the `echo` command and redirection operation to achieve this:
   ```
   echo "your_public_key" >> ~/.ssh/authorized_keys
   ```
   Here, `your_public_key` is the public key you copied earlier. This command will append your public key to the end of the `authorized_keys` file.

After completing these steps, you should be able to connect to your server via SSH using your private key. Remember, you can only connect using the corresponding private key if your public key has been added to the server's `authorized_keys` file.

6. **Use the client to log in to the remote server**
```
ssh -i /path/to/your/private/key user@xxx.xxx.xxx.xxx
```

#### 2. Initialize ckb-sync-mainnet
Because we are not debugging on AWS, AWS_ACCESS_KEY and AWS_SECRET_KEY can be arbitrarily written. The setup is to copy ansible to the job directory. Modifications can be made in the files under the job directory later to prevent file contamination.
```shell
export AWS_ACCESS_KEY=xxx
export AWS_SECRET_KEY=xxx
export GITHUB_TOKEN="xxx"
bash ckb-sync-mainnet/script/sync-mainnet.sh setup
```
The output is as follows, `sync-mainnet-2023-06-07-in-10h` is the generated temporary directory.
```angular2html
Info: /workspace/ckb-integration-test/ckb-sync-mainnet/job/sync-mainnet-2023-06-07-in-10h/ssh/id already existed, reuse it
Starting galaxy role install process
- extracting ansible-ckb to /home/gitpod/.ansible/roles/ansible-ckb
- ansible-ckb (main) was installed successfully
```

Set the temporary work directory
```angular2html
export CURRENT_JOB_DIR=sync-mainnet-2023-06-07-in-10h
```

##### 3. Add the configuration of the remote server to the configuration file
1. **Add the server you want to execute remotely in

the configuration file**
Set the remote server ip: `18.162.180.86` to log in to this server through the user `ckb`.
```shell
echo "all:
  hosts:
    18.162.180.86:
      ansible_user: ckb" > ckb-sync-mainnet/job/${CURRENT_JOB_DIR}/ansible/inventory.yml
cat ckb-sync-mainnet/job/${CURRENT_JOB_DIR}/ansible/inventory.yml
```
The input result is as follows
```shell
all:
  hosts:
    18.162.180.86:
      ansible_user: ckb
```

2. **Copy the `OPENSSH PRIVATE KEY` to the ssh in the work directory**
   Check if the key can log in to the server
```shell
cp ~/.ssh/id_rsa ckb-sync-mainnet/job/${CURRENT_JOB_DIR}ssh/id
ssh -i ckb-sync-mainnet/job/${CURRENT_JOB_DIR}/ssh/id ckb@18.162.180.86
```
The output is as follows, indicating a successful login
```shell

gitpod /workspace/ckb-integration-test (main) $ ssh -i ~/.ssh/id_rsa ckb@18.162.180.86
Welcome to Ubuntu 20.04.4 LTS (GNU/Linux 5.15.0-1023-aws x86_64)

 * Documentation:  https://help.ubuntu.com
 * Management:     https://landscape.canonical.com
 * Support:        https://ubuntu.com/advantage

  System information as of Wed Jun  7 08:35:28 UTC 2023

  System load:                      0.21
  Usage of /:                       68.6% of 581.57GB
  Memory usage:                     52%
  Swap usage:                       0%
  Processes:                        199
  Users logged in:                  1
  IPv4 address for br-b378b8ec3d03: 192.168.5.1
  IPv4 address for br-cb51c262c48a: 192.168.6.1
  IPv4 address for docker0:         172.17.0.1
  IPv4 address for ens5:            172.31.45.113

 * Ubuntu Pro delivers the most comprehensive open source security and
   compliance features.

   https://ubuntu.com/aws/pro

74 updates can be applied immediately.
To see these additional updates run: apt list --upgradable

New release '22.04.2 LTS' available.
Run 'do-release-upgrade' to upgrade to it.


*** System restart required ***
Last login: Wed Jun  7 08:34:14 2023 from 35.227.154.22
```

#### 3. run sync-test
```shell
export AWS_ACCESS_KEY=xxx
export AWS_SECRET_KEY=xxx
export GITHUB_TOKEN="xxx"
bash ckb-sync-mainnet/script/sync-mainnet.sh ansible
```
The output is as follows
```shell
PLAY [sync-mainnet] *****************************************************************************************************************************************

TASK [Gathering Facts] **************************************************************************************************************************************
ok: [18.162.180.86]

TASK [include_vars] *****************************************************************************************************************************************
ok: [18.162.180.86]

TASK [Operate CKB Via Ansible-CKB] **************************************************************************************************************************
```

You can see the started ckb on the server
```shell
ssh -i ckb-sync-mainnet/job/${CURRENT_JOB_DIR}/ssh/id ckb@18.162.180.86
cd /var/lib/ckb
ls
```
You should be able to

see the result
```shell
ckb@test-01:/var/lib/ckb$ ls
ckb  ckb-miner.toml  ckb.toml  data  default.db-options
```

#### 4. Clean up the running ckb environment

```shell
bash ckb-sync-mainnet/script/sync-mainnet.sh clean_ckb_env
```
The output is as follows
### clean job
```shell
gitpod /workspace/ckb-integration-test/ckb-sync-mainnet/script (main) $ bash sync-mainnet.sh clean_ckb_env

PLAY [sync-mainnet] ***********************************************************************************************************************************

TASK [Gathering Facts] ********************************************************************************************************************************
ok: [18.162.180.86]

TASK [include_vars] ***********************************************************************************************************************************
ok: [18.162.180.86]

PLAY RECAP ********************************************************************************************************************************************
18.162.180.86              : ok=2    changed=0    unreachable=0    failed=0    skipped=0    rescued=0    ignored=0   
```
You can check whether the ckb file has been deleted on the server
```shell
ssh -i ckb-sync-mainnet/job/${CURRENT_JOB_DIR}/ssh/id ckb@18.162.180.86
cd /var/lib/ckb
ls
```
The output is as follows, indicating that the ckb file was successfully deleted
```shell
-bash: cd: ckb: No such file or directory
```