# We need to 
# start docker-compose 
# start manager server: https://github.com/massbitprotocol/substrate-indexer/blob/master/packages/manager/package.json
# start apollo (don't have 1 yet)

git clone https://github.com/massbitprotocol/substrate-indexer
cd substrate-indexer/packages/manager

# Install docker
sudo apt update
sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
sudo add-apt-repository 'deb [arch=amd64] https://download.docker.com/linux/ubuntu bionic stable'
sudo apt update
apt-cache policy docker-ce
sudo apt install -y docker-ce docker-compose

sudo docker-compose up -d

# Install yarn
curl -sS https://dl.yarnpkg.com/debian/pubkey.gpg | sudo apt-key add -
echo "deb https://dl.yarnpkg.com/debian/ stable main" | sudo tee /etc/apt/sources.list.d/yarn.list
sudo apt update
sudo apt install yarn -y

# Install node
curl -fsSL https://deb.nodesource.com/setup_14.x | bash - && apt-get install -y nodejs 


# Start
yarn install
tmux new -d -s indexer-manager yarn start:dev

# Nginx
sudo apt install -y nginx
sudo snap install core; sudo snap refresh core
sudo snap install --classic certbot
sudo ln -s /snap/bin/certbot /usr/bin/certbot
sudo certbot --nginx
# Use sub-index-staging.massbit.io

# Point to correct port
cd /etc/nginx/sites-available
sudo vi default
    proxy_pass http://localhost:3000;
sudo nginx -s reload