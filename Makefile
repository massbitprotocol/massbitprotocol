#################### Init commands #######################
init-code-compiler:
	@echo "Installing all the dependencies for Code compiler ..."
	pip install ipfshttpclient flask flask_cors

#################### Test commands #######################
create-git-hook:
	@echo "Every push to origin need to run the E2E tests"
	@echo "Creating symlink..."
	ln -s -f ../../.githooks/pre-push .git/hooks/pre-push
	@echo "You can remove all the git-hooks with the 'make remove-all-git-hook'" command

remove-all-git-hook:
	@echo "Removing all symlinks..."
	rm .git/hooks/*


#This test for run all test when the component already up
test-run-all:
	@echo "Running health check tests ..."
	cd e2e-test/health-check && robot health-check.robot || true

	@echo "Running substrate tests ..."
	cd e2e-test/substrate && robot substrate.robot

	make restart-chain-reader-index-manager

	@echo "Running solana tests ..."
	cd e2e-test/solana && robot solana.robot

	make restart-chain-reader-index-manager

	@echo "Running ethereum tests ..."
	cd e2e-test/ethereum && robot ethereum.robot

#This test start/restart all service and run all test
test-run-all-and-up:
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

	@echo "Running substrate tests ..."
	cd e2e-test/substrate && robot substrate.robot || true

	make restart-chain-reader-index-manager

	@echo "Running solana tests ..."
	cd e2e-test/solana && robot solana.robot || true

	make restart-chain-reader-index-manager

	@echo "Running ethereum tests ..."
	cd e2e-test/ethereum && robot ethereum.robot || true


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

services-up:
	@echo "Run all service"
	docker-compose -f docker-compose.min.yml up

services-down:
	@echo "Stop all service"
	docker-compose -f docker-compose.min.yml down

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
	cd e2e-test/ethereum && robot ethereum.robot ;

	@echo "Running report email services"
	tmux new -d -s report_email "cd e2e-test && python check_log.py"
	tmux ls

test-long-running-quickswap-run-test-only:
	@echo "Running only the quickswap Ethereum test ..."
	cd e2e-test/ethereum && robot ethereum.robot