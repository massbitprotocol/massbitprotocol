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


# Start services
sudo docker-compose -f docker-compose.min.yml up -d
make tmux-chain-reader-binary
make tmux-index-api-binary

# Setup cert manually
sudo certbot --nginx # Use sub-index-staging.massbit.io and point to correct port
