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
Massbit Indexer:
- CPU: 16 cores
- Ram: 32 GB
- SSD or HDD: 
  - If your goal is to index DEXs: 200 GB is recommended
  - If your goal is to index the chain: 2 TB is recommended for each chain

Custom Solana node (optional)
- The node needs to be Full Archival Node
- Use the hardware recommendation from https://docs.solana.com/running-validator/validator-reqs

Custom Polygon node (optional)
- to be added

Custom Ethereum node (optional)
- to be added

Custom BSC node (optional)
- to be added

## How to start
Index with public BSC/Ethereum/Polygon/Solana Node
- Start the docker services in production mode
  ```shell
  make services-prod-up
  ```
- Start indexing
  ```
  make index-quickswap
  make index-pancakeswap
  ```

Index with custom BSC/Ethereum/Polygon/Solana Node
- Start your BSC/Ethereum/Polygon/Solana node
- Override chain-reader environment config with BSC/Ethereum/Polygon/Solana endpoint in the docker-compose.prod.yml
  ```yaml
  chain-reader:
    ...
    environment:
      SOLANA_WS: add_your_ws
      SOLANA_URL: add_your_url
      POLYGON_WS: add_your_ws
      POLYGON_URL: add_your_url
      BSC_WS: add_your_ws
      BSC_URL: add_your_url
      ETHEREUM_WS: add_your_ws
      ETHEREUM_URL: add_your_url
  ```
- Start the docker services in production mode 
  ```shell
  make services-prod-up
  ```
- Start indexing
  ```
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

