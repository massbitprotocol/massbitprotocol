git clone https://github.com/massbitprotocol/massbitprotocol
cd massbitprotocol

# Install docker
sudo apt update && 
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common &&
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add - &&
sudo add-apt-repository 'deb [arch=amd64] https://download.docker.com/linux/ubuntu bionic stable' &&
apt-cache policy docker-ce && 
sudo apt install -y docker-ce docker-compose

# Install some dependencies so we can run Rust binaries
sudo apt update && 
sudo apt install -y git curl && 
DEBIAN_FRONTEND=noninteractive curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y &&
sudo apt install -y cmake pkg-config libssl-dev git gcc build-essential clang libclang-dev libpq-dev 
    libssl-dev libudev-dev pkg-config zlib1g-dev llvm clang make && 

# Nginx
sudo apt install -y nginx &&
sudo snap install core; sudo snap refresh core &&
sudo snap install --classic certbot &&
sudo ln -s /snap/bin/certbot /usr/bin/certbot

# Start services
sudo docker-compose -f docker-compose.min.yml up -d

# Go to route53 and point domain to IP address
# Setup cert manually
sudo certbot --nginx # Use sol-index-staging.massbit.io and point to correct port

# Point to correct port
cd /etc/nginx/sites-available
sudo vi default
    proxy_pass http://localhost:3031;
sudo nginx -s reload

# Setup systemd for chain-reader
cd /etc/systemd/system
sudo touch chain-reader.service && sudo vi chain-reader.service
-----
[Unit]
Description=Chain Reader For Solana

[Service]
User=root
WorkingDirectory=/home/hughie/massbitprotocol/deployment/binary/chain-reader
ExecStart=/home/hughie/massbitprotocol/deployment/binary/chain-reader/chain-reader
Restart=always

[Install]
WantedBy=multi-user.target
-----

# Setup systemd for indexer-api
cd /etc/systemd/system
sudo touch indexer-api.service && sudo vi indexer-api.service
-----
[Unit]
Description=Indexer API For Solana

[Service]
User=root
WorkingDirectory=/home/hughie/massbitprotocol/deployment/binary/indexer-api
ExecStart=/home/hughie/massbitprotocol/deployment/binary/indexer-api/indexer-api
Restart=always

[Install]
WantedBy=multi-user.target
-----

# Config SSH 
cd ~/.ssh
# Go to Google Cloud metadata and add the SSH key
# Add SSH_PRIVATE_KEY with github-action value - so Github action can deploy
# ADD SSH_HOST with the IP Address - so Github action can deploy
# ADD SSH_USER in github repo's secret

# Deploy with merge to master or by creating tags 


############## Useful scripts ####################
sudo systemctl daemon-reload
sudo systemctl start chain-reader.service
sudo systemctl start indexer-api.service
systemctl | grep Solana
sudo sysctl -p --system  # To reload system services
journalctl -xefu indexer-api.service -b  # View logs