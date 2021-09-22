# Massbit Indexer
The goal of Massbit Indexer is to bring scalability and interoperability to Indexers.
In order to embrace the existing popular community and advanced technology, it will bring huge benefits by staying compatible with all the existing indexing mapping logics.
And to achieve that, the easiest solution is to develop with some existing features from the-graph, as we respect the great work of the-graph very much.

## Prerequisites
- Docker
- Python
- make

```shell
sudo apt install make
make init-docker
make init-python
make init-test
```

## Hardware requirements
Running with public BSC/Ethereum/Solana node
- CPU: 16 cores
- Ram: 32 GB
- SSD or HDD: 200 GB

Running with local BSC/Ethereum/Solana node
- The node needs to be Full Archival Node
- Use the hardware recommendation from https://docs.solana.com/running-validator/validator-reqs

## How to start
Running with public BSC/Ethereum/Solana Node
```shell
make services-prod-up
make index-quickswap
make index-pancakeswap
```

Running with local BSC/Ethereum/Solana Node
- Start your BSC/Ethereum/Solana node
- Modify chain-reader/chain-reader/src/lib.rs pointing to your local ws and http url
- ```shell
  make services-prod-up
  make index-quickswap
  make index-pancakeswap
  ```
  
## OS tuning tips
## Increase max open files
* Sysctl: increase max open file to 6816768
  Edit file /etc/sysctl.conf with line
  ` fs.file-max=6816768`
  Apply config with command
  `sysctl -p`

* Ulimit: Increase number file open to 99999 and max process to 20000 to massbit (fill * in case all users)
  Edit file  /etc/security/limits.conf with line
```
massbit       soft    nofile  99999
massbit      hard    nofile  99999
massbit      soft    noproc  20000
massbit       hard    noproc  20000

```
Check config with command:
`su - massbit -c 'ulimit -a'`

## Other kernel tunning in sysctl.conf
```
vm.max_map_count=1048576
vm.min_free_kbytes=65535
vm.overcommit_memory=1
vm.swappiness=0
vm.vfs_cache_pressure=50
net.ipv4.ip_local_port_range = 18000    65535
net.netfilter.nf_conntrack_tcp_timeout_established=86400
net.core.rmem_max = 134217728
net.core.wmem_max = 134217728
net.ipv4.tcp_rmem = 4096 87380 67108864
net.ipv4.tcp_wmem = 4096 65536 67108864
net.ipv4.tcp_congestion_control=bbr
net.core.default_qdisc = fq
net.ipv4.tcp_mtu_probing=1
net.ipv4.tcp_keepalive_time = 30
net.ipv4.tcp_keepalive_probes = 5
net.ipv4.tcp_keepalive_intvl = 15
net.ipv4.tcp_fastopen=3

```

