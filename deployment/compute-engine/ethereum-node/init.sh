#!/bin/bash
add-apt-repository ppa:ethereum/ethereum
apt install -y ethereum
# setup new user
ETH_HOME=/home/ethereum/
ETH_USER=ethereum
SERVICE=/etc/systemd/system/ethereum.service
RUN_SCRIPT=/nodes/ethereum/run.sh
mkdir -p /nodes/ethereum
chown ${ETH_USER}:${ETH_USER} -R /nodes/ethereum

sudo mkdir "${ETH_HOME}"
sudo chmod -R 757 "${ETH_HOME}"
sudo chmod -R 757 /nodes/ethereum/
sudo adduser --disabled-password --gecos "" --home "${ETH_HOME}" "${ETH_USER}"
# create systemd
sudo cat >${SERVICE} <<EOL
  [Unit]
      Description=Geth Node
      After=network.target
      [Service]
LimitNOFILE=700000
LogRateLimitIntervalSec=0
      User=ethereum
      Group=ethereum
      WorkingDirectory=/nodes/ethereum/
      Type=simple
      ExecStart=/nodes/ethereum/run.sh
      Restart=always
      RestartSec=10
      [Install]
      WantedBy=multi-user.target
EOL
sudo cat >${RUN_SCRIPT} <<EOL
#!/usr/bin/bash
/usr/bin/geth --nousb --http --http.addr 0.0.0.0 --http.api db,eth,net,web3,personal,shh --http.vhosts "*" --http.corsdomain "*" --ws --ws.addr 0.0.0.0 --ws.origins "*" --ws.api db,eth,net,web3,personal,shh 2>&1  >> /nodes/ethereum/eth.log
EOL
chmod +x $RUN_SCRIPT
chown ${ETH_USER}:${ETH_USER} -R /nodes/ethereum
systemctl enable ethereum.service
systemctl start ethereum.service
