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


#This test for run all test when the component already up
test-run-all:
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
test-run-all-and-up:
	@echo "Close all services before running test"
	make services-down
	make kill-all-tmux || true

	@echo "Restart services before running test"

	tmux new -d -s services "make services-up"
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
	tmux new -d -s index-manager "make run-index-manager"

	@echo "Run chain-reader in tmux"
	tmux new -d -s chain-reader "make run-chain-reader"

	@echo "Run code-compiler in tmux"
	tmux new -d -s code-compiler "make run-code-compiler"

kill-all-tmux:
	@echo "Kill all tmux services"
	pkill chain-reader || true
	pkill code-compiler || true #Fixme: this cmd cannot kill code-compiler yet
	pkill index-manager || true
	tmux list-sessions | awk 'BEGIN{FS=":"}{print $1}' | xargs -n 1 tmux kill-session -t
	tmux ls

restart-chain-reader-index-manager:
	@echo "Run index-manager in tmux"
	pkill chain-reader || true
	tmux kill-session -t chain-reader || true
	pkill index-manager || true
	tmux kill-session -t index-manager || true
	sleep 3

	@echo "Run run-chain-reader and index-manager tmux"
	tmux new -d -s chain-reader "make run-chain-reader"
	tmux new -d -s index-manager "make run-index-manager"
	sleep 3
