git clone https://github.com/massbitprotocol/massbitprotocol
cd massbitprotocol

# Install docker
sudo apt update
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository 'deb [arch=amd64] https://download.docker.com/linux/ubuntu bionic stable'
sudo apt update
apt-cache policy docker-ce
sudo apt install -y docker-ce docker-compose

# Nginx
sudo apt install -y nginx
sudo snap install core; sudo snap refresh core
sudo snap install --classic certbot
sudo ln -s /snap/bin/certbot /usr/bin/certbot

# Upload binaries
SERVER=hughie@34.159.173.231
scp ./target/release/indexer-api $SERVER:./massbitprotocol/deployment/binary/indexer-api
scp ./target/release/chain-reader $SERVER:./massbitprotocol/deployment/binary/chain-reader

# Start services
sudo docker-compose -f docker-compose.min.yml up -d
make tmux-chain-reader-binary
make tmux-indexer-api-binary

# Setup cert manually
sudo certbot --nginx # Use sol-index-staging.massbit.io and point to correct port

# Point to correct port
cd /etc/nginx/sites-available
sudo vi default
    proxy_pass http://localhost:3031;
sudo nginx -s reload