#################### Init commands #######################
init-code-compiler:
	@echo "Installing all the dependencies for Code compiler ..."
	pip install ipfshttpclient flask flask_cors psycopg2

init-docker:
	@echo "Installing docker"
	sudo apt update
	sudo apt install -y apt-transport-https ca-certificates curl software-properties-common
	curl -fsSL https://download.docker.com/linux/ubuntu/gpg | sudo apt-key add -
	sudo add-apt-repository 'deb [arch=amd64] https://download.docker.com/linux/ubuntu bionic stable'
	sudo apt update
	apt-cache policy docker-ce
	sudo apt install -y docker-ce docker-compose
	sudo groupadd docker || true
	sudo gpasswd -a $USER docker
	sudo setfacl -m user:$USER:rw /var/run/docker.sock

init-python:
	sudo apt install -y python3
	sudo apt install -y python3.8
	sudo rm /usr/bin/python3
	sudo ln -s python3.8 /usr/bin/python3
	sudo apt install -y python3-pip wget unzip libpq-dev python3-dev
	sudo pip3 install setuptools-rust
	sudo pip3 install --upgrade pip
	sudo pip3 install PyQtWebEngine

init-npm:
	sudo apt install -y npm
	curl -fsSL https://deb.nodesource.com/setup_14.x | bash -
	sudo apt-get install -y nodejs

init-test:
	@echo "Installing some important libraries for scripting ..."
	sudo apt install -y tmux
	@echo "Installing all the dependencies for E2E tests ..."
	pip3 install robotframework robotframework-requests robotframework-databaselibrary rpaframework
	pip3 install psycopg2 rpaframework robotframework-seleniumlibrary robotframework-sshlibrary

#################### Test commands #######################
create-git-hook:
	@echo "Every push to origin need to run the E2E tests"
	@echo "Creating symlink..."
	ln -s -f ../../.githooks/pre-push .git/hooks/pre-push
	@echo "You can remove all the git-hooks with the 'make remove-all-git-hook'" command

remove-all-git-hook:
	@echo "Removing all symlinks..."
	rm .git/hooks/*


test-run-contract:
	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true

	@echo "Running polygon contract tests ..."
	cd e2e-test/polygon && robot contract.robot
	make restart-chain-reader-index-manager

	@echo "Running bsc contract tests ..."
	cd e2e-test/bsc && robot contract.robot

test-run-chain:
	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true

	@echo "Running polygon contract tests ..."
	cd e2e-test/polygon && robot chain.robot


#This test for run all test when the component already up
test-run-basic:
	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true

	@echo "Running basic substrate tests ..."
	cd e2e-test/substrate && robot basic.robot
	make restart-chain-reader-index-manager

	@echo "Running basic solana tests ..."
	cd e2e-test/solana && robot basic.robot
	make restart-chain-reader-index-manager

	@echo "Running basic ethereum tests ..."
	cd e2e-test/ethereum && robot basic.robot
	make restart-chain-reader-index-manager


#This test start/restart all service and run all test
test-run-basic-and-up:
	@echo "Close all services before running test"
	make services-down
	make kill-all-tmux || true

	@echo "Restart services before running test"
	make services-up
	#tmux new -d -s services "make services-up"
	sleep 5;
	make run-all-tmux
	tmux ls
	sleep 5;

	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true

	@echo "Running basic substrate tests ..."
	cd e2e-test/substrate && robot basic.robot || true
	make restart-chain-reader-index-manager

	@echo "Running basic solana tests ..."
	cd e2e-test/solana && robot basic.robot || true
	make restart-chain-reader-index-manager

	@echo "Running basic ethereum tests ..."
	cd e2e-test/ethereum && robot basic.robot || true
	make restart-chain-reader-index-manager

test-init:
	@echo "Installing all the dependencies for E2E tests ..."
	pip install robotframework robotframework-requests robotframework-databaselibrary psycopg2 rpaframework robotframework-seleniumlibrary robotframework-sshlibrary
	@echo "Installing Webdriver for Selenium to run tests ..."
	sudo pip install webdrivermanager
	sudo webdrivermanager firefox chrome --linkpath /usr/local/bin

create-list-user-example-json-file:
	@echo "Create list user examples json file ..."
	cd user-example && python create_example_json.py

restart-chain-reader-index-manager:
	@echo "Stop index-manager and chain-reader in tmux"
	tmux kill-session -t chain-reader
	tmux kill-session -t index-manager
	@echo "Run index-manager in tmux"
	tmux new -d -s index-manager scripts/tmux-index-manager.sh
	@echo "Run chain-reader in tmux"
	tmux new -d -s chain-reader scripts/tmux-chain-reader.sh

#################### Dev commands ##########################

deploy:
	@echo "Deploy already build indexer $(id)"
	curl --location --request POST 'localhost:5000/deploy/wasm' \
    --header 'Content-Type: application/json' \
    --data-raw '{"configs": {"model": "Factory" }, "compilation_id": "$(id)" }'

run-index-manager:
	@echo "Run index-manager"
	cargo run --bin index-manager-main

run-chain-reader:
	@echo "Run chain-reader"
	cargo run --bin chain-reader

run-code-compiler:
	@echo "Run code-compiler"
	cd code-compiler/ && python app.py

services-dev-up:
	@echo "Run all services in dev mode"
	docker-compose -f docker-compose.min.yml up

services-dev-down:
	@echo "Stop all services"
	docker-compose -f docker-compose.min.yml down

services-prod-up:
	@echo "Run all services in production mode"
	docker-compose -f docker-compose.prod.yml up -d

services-prod-down:
	@echo "Stop all services"
	docker-compose -f docker-compose.prod.yml down

#################### Production commands ##################
run-all-tmux:
	@echo "A quick fix to bypass the not able to start tmux error"
	export TERM=xterm

	@echo "Run index-manager in tmux"
	tmux new -d -s index-manager scripts/tmux-index-manager.sh

	@echo "Run chain-reader in tmux"
	tmux new -d -s chain-reader scripts/tmux-chain-reader.sh

	@echo "Run code-compiler in tmux"
	tmux new -d -s code-compiler scripts/tmux-code-compiler.sh

kill-all-tmux:
	@echo "Kill all tmux services"
	tmux list-sessions | awk 'BEGIN{FS=":"}{print $1}' | xargs -n 1 tmux kill-session -t


#################### Long running test commands ##################
test-long-running-quickswap:
	@echo "A quick fix to bypass the not able to start tmux error"
	export TERM=xterm
	@echo "Run index-manager in tmux"
	tmux new -d -s index-manager scripts/tmux-index-manager.sh
	@echo "Run chain-reader in tmux"
	tmux new -d -s chain-reader scripts/tmux-chain-reader.sh
	@echo "Run code-compiler in tmux"
	tmux new -d -s code-compiler scripts/tmux-code-compiler.sh
	@echo "Wait for the services to start"
	sleep 15;
	@echo "Running only the quickswap Ethereum test ..."
	cd e2e-test/polygon && robot -t "Compile and Deploy WASM Test Quickswap" basic.robot;

	@echo "Running report email services"
	tmux new -d -s report_email "cd e2e-test && python check_log.py"
	tmux ls


index-quickswap:
	@echo "Running only the quickswap Polygon test ..."
	cd e2e-test/polygon && robot -t "Compile and Deploy WASM Test Quickswap" contract.robot
	@echo "Running report email services"
	tmux new -d -s report_email_quickswap "cd e2e-test && python check_log.py"
	tmux ls


index-pancakeswap:
	@echo "Running only the pancakeswap BSC test ..."
	cd e2e-test/bsc && robot -t "Compile and Deploy Pancakeswap Exchange WASM" contract.robot
	@echo "Running report email services"
	tmux new -d -s report_email_pancakeswap "cd e2e-test && python check_log.py"
	tmux ls


test-long-running-quickswap-run-test-only:
	@echo "Running only the quickswap Polygon test ..."
	cd e2e-test/polygon && robot -t "Compile and Deploy WASM Test Quickswap" contract.robot
